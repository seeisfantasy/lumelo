use dsd_reader::DsdReader;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ipc_proto::{
    command_socket_path_from_env, event_socket_path_from_env, format_ack_line, format_error_line,
    format_event_line, format_history_snapshot_line, format_queue_snapshot_line,
    format_status_line, history_state_path_from_env, parse_command_line, queue_state_path_from_env,
    state_dir_path, HistorySnapshotEntryView, HistorySnapshotView, PlaybackCommand, PlaybackEvent,
    PlaybackFailureClass, PlaybackState, PlaybackStatusSnapshot, QueueSnapshotEntryView,
    QueueSnapshotView,
};
use media_model::{HistoryEntry, HistoryLog, OrderMode, QueueEntry, QueueSnapshot, RepeatMode};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};

const QUEUE_FILE_VERSION: u32 = 1;
const HISTORY_FILE_VERSION: u32 = 1;
const HISTORY_LIMIT: usize = 100;
const LIBRARY_DB_FILENAME: &str = "library.db";
const DEFAULT_ALSA_CARDS_PATH: &str = "/proc/asound/cards";
const DEFAULT_CONFIG_PATH: &str = "/etc/lumelo/config.toml";
const ALLOW_ABSOLUTE_PATHS_ENV: &str = "LUMELO_PLAYBACK_ALLOW_ABSOLUTE_PATHS";
const AUDIO_DEVICE_ENV: &str = "LUMELO_AUDIO_DEVICE";
const ALSA_CARDS_PATH_ENV: &str = "LUMELO_ALSA_CARDS_PATH";
const DEFAULT_DSD_POLICY: DsdOutputPolicy = DsdOutputPolicy::NativeDsd;
const DSD64_RATE_HZ: u32 = 2_822_400;
const DSD_SILENCE_BYTE: u8 = 0x69;
const DSD_PCM_FALLBACK_RATE_HZ: u32 = 44_100;

fn main() {
    if let Err(err) = run() {
        eprintln!("playbackd failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let command_socket = command_socket_path_from_env();
    let event_socket = event_socket_path_from_env();
    let queue_path = Arc::new(queue_state_path_from_env());
    let history_path = Arc::new(history_state_path_from_env());
    let dsd_policy = load_dsd_output_policy();
    let track_resolver = TrackResolver::new(state_dir_path().join(LIBRARY_DB_FILENAME));
    let output_controller = OutputController::new(
        configured_audio_device_from_env(),
        alsa_cards_path_from_env(),
        dsd_policy,
    );

    prepare_socket_path(&command_socket)?;
    let event_hub = EventHub::bind(&event_socket)?;
    let listener = UnixListener::bind(&command_socket)
        .map_err(|err| format!("bind {}: {err}", command_socket.display()))?;

    let initial_state = load_runtime_state(queue_path.as_ref(), history_path.as_ref());
    let state = Arc::new(Mutex::new(initial_state));

    println!("playbackd listening");
    println!("  command socket: {}", command_socket.display());
    println!("  event socket:   {}", event_socket.display());
    println!("  queue state:    {}", queue_path.display());
    println!("  history state:  {}", history_path.display());
    println!("  library db:     {}", track_resolver.db_path.display());
    println!(
        "  absolute paths: {}",
        if track_resolver.allows_absolute_paths() {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("  audio device:   {}", output_controller.device_label());
    println!("  dsd policy:     {}", output_controller.dsd_policy_label());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&state);
                let event_hub = event_hub.clone();
                let queue_path = Arc::clone(&queue_path);
                let history_path = Arc::clone(&history_path);
                let track_resolver = track_resolver.clone();
                let output_controller = output_controller.clone();
                thread::spawn(move || {
                    if let Err(err) = handle_client(
                        stream,
                        state,
                        event_hub,
                        queue_path,
                        history_path,
                        track_resolver,
                        output_controller,
                    ) {
                        eprintln!("playbackd client error: {err}");
                    }
                });
            }
            Err(err) => eprintln!("playbackd accept error: {err}"),
        }
    }

    Ok(())
}

#[derive(Debug)]
struct RuntimeState {
    snapshot: QueueSnapshot,
    history_log: HistoryLog,
    playback_state: PlaybackState,
    current_track: Option<String>,
    last_command: Option<String>,
    next_queue_entry_id: u64,
}

#[derive(Debug)]
struct CommandOutcome {
    response_line: String,
    events: Vec<PlaybackEvent>,
    persist_queue: bool,
    persist_history: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum OutputAction {
    None,
    Start(String),
    Pause,
    Resume(String),
    Stop,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum TrackFinishAction {
    None,
    Stop { reason: String },
    Start { track_id: String },
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ResolvedTrack {
    track_uid: String,
    volume_uuid: String,
    relative_path: String,
    title: Option<String>,
    duration_ms: Option<u64>,
    format: String,
    absolute_path: PathBuf,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct DecodedPCMConfig {
    sample_rate: u32,
    channels: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum DsdOutputPolicy {
    NativeDsd,
    Dop,
}

impl DsdOutputPolicy {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "native_dsd" | "strict_native" | "native_dop" => Some(Self::NativeDsd),
            "dop" => Some(Self::Dop),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::NativeDsd => "native_dsd",
            Self::Dop => "dop",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum DsdTransport {
    Native(AlsaDsdFormat),
    Dop(DopPcmFormat),
    Pcm,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum AlsaDsdFormat {
    U8,
    U16Le,
    U16Be,
    U32Le,
    U32Be,
}

impl AlsaDsdFormat {
    fn aplay_format(self) -> &'static str {
        match self {
            Self::U8 => "DSD_U8",
            Self::U16Le => "DSD_U16_LE",
            Self::U16Be => "DSD_U16_BE",
            Self::U32Le => "DSD_U32_LE",
            Self::U32Be => "DSD_U32_BE",
        }
    }

    fn sample_rate_divisor(self) -> u32 {
        match self {
            Self::U8 => 8,
            Self::U16Le | Self::U16Be => 16,
            Self::U32Le | Self::U32Be => 32,
        }
    }

    fn push_dsd_byte(self, byte: u8, out: &mut Vec<u8>) {
        match self {
            Self::U8 => out.push(byte),
            Self::U16Le => out.extend_from_slice(&[byte, 0x00]),
            Self::U16Be => out.extend_from_slice(&[0x00, byte]),
            Self::U32Le => out.extend_from_slice(&[byte, 0x00, 0x00, 0x00]),
            Self::U32Be => out.extend_from_slice(&[0x00, 0x00, 0x00, byte]),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum DopPcmFormat {
    S32Le,
    S24Le,
    S24_3Le,
}

impl DopPcmFormat {
    fn aplay_format(self) -> &'static str {
        match self {
            Self::S32Le => "S32_LE",
            Self::S24Le => "S24_LE",
            Self::S24_3Le => "S24_3LE",
        }
    }

    fn push_dop_sample(self, lo: u8, hi: u8, marker: u8, out: &mut Vec<u8>) {
        match self {
            Self::S24_3Le => out.extend_from_slice(&[lo, hi, marker]),
            Self::S24Le | Self::S32Le => {
                out.extend_from_slice(&[lo, hi, marker, 0x00]);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct DsdSourceInfo {
    channels: usize,
    dsd_rate_hz: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DsdPlaybackPlan {
    device: String,
    transport: DsdTransport,
    channels: usize,
    sample_rate: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
struct AlsaStreamCapabilities {
    formats: Vec<String>,
    rates: Vec<u32>,
}

impl AlsaStreamCapabilities {
    fn supports(&self, format: &str, rate: u32) -> bool {
        self.formats.iter().any(|value| value == format)
            && self.rates.iter().any(|value| *value == rate)
    }

    fn summary(&self) -> String {
        let formats = if self.formats.is_empty() {
            "(none)".to_string()
        } else {
            self.formats.join(", ")
        };
        let rates = if self.rates.is_empty() {
            "(none)".to_string()
        } else {
            self.rates
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };

        format!("formats=[{formats}] rates=[{rates}]")
    }
}

#[derive(Debug, Clone)]
struct TrackResolver {
    db_path: PathBuf,
    allow_absolute_paths: bool,
}

#[derive(Debug, Clone)]
struct OutputController {
    inner: Arc<Mutex<OutputState>>,
    configured_device: Option<Arc<String>>,
    alsa_cards_path: Arc<PathBuf>,
    dsd_policy: DsdOutputPolicy,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct UsbAudioOutputDevice {
    card_index: usize,
    card_id: String,
    name: String,
    alsa_device: String,
}

#[derive(Debug, Default)]
struct OutputState {
    generation: u64,
    pid: Option<u32>,
    track_id: Option<String>,
    paused: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PlaybackOperationError {
    code: &'static str,
    reason: String,
    class: PlaybackFailureClass,
    recoverable: bool,
    keep_quiet: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct PersistedQueueState {
    version: u32,
    updated_at: u64,
    order_mode: OrderMode,
    repeat_mode: RepeatMode,
    current_order_index: Option<usize>,
    play_order: Vec<String>,
    tracks: Vec<QueueEntry>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct PersistedHistoryState {
    version: u32,
    updated_at: u64,
    entries: Vec<HistoryEntry>,
}

impl PersistedQueueState {
    fn from_snapshot(snapshot: &QueueSnapshot) -> Self {
        Self {
            version: QUEUE_FILE_VERSION,
            updated_at: unix_timestamp_secs(),
            order_mode: snapshot.order_mode,
            repeat_mode: snapshot.repeat_mode,
            current_order_index: snapshot.current_order_index,
            play_order: snapshot.play_order.clone(),
            tracks: snapshot.tracks.clone(),
        }
    }

    fn into_snapshot(self) -> QueueSnapshot {
        QueueSnapshot {
            order_mode: self.order_mode,
            repeat_mode: self.repeat_mode,
            current_order_index: self.current_order_index,
            play_order: self.play_order,
            tracks: self.tracks,
        }
    }
}

impl PersistedHistoryState {
    fn from_log(history_log: &HistoryLog) -> Self {
        Self {
            version: HISTORY_FILE_VERSION,
            updated_at: unix_timestamp_secs(),
            entries: history_log.entries.clone(),
        }
    }

    fn into_log(self) -> HistoryLog {
        HistoryLog {
            entries: self.entries,
        }
    }
}

impl PlaybackOperationError {
    fn content(code: &'static str, reason: impl Into<String>, keep_quiet: bool) -> Self {
        Self {
            code,
            reason: reason.into(),
            class: PlaybackFailureClass::Content,
            recoverable: true,
            keep_quiet,
        }
    }

    fn output(code: &'static str, reason: impl Into<String>) -> Self {
        Self {
            code,
            reason: reason.into(),
            class: PlaybackFailureClass::Output,
            recoverable: true,
            keep_quiet: false,
        }
    }
}

impl TrackResolver {
    fn new(db_path: PathBuf) -> Self {
        Self::new_with_absolute_path_playback(db_path, absolute_path_playback_enabled())
    }

    fn new_with_absolute_path_playback(db_path: PathBuf, allow_absolute_paths: bool) -> Self {
        Self {
            db_path,
            allow_absolute_paths,
        }
    }

    fn allows_absolute_paths(&self) -> bool {
        self.allow_absolute_paths
    }

    fn resolve(&self, track_id: &str) -> Result<ResolvedTrack, PlaybackOperationError> {
        if let Some(track) = self.resolve_path_track(track_id)? {
            return Ok(track);
        }

        let connection = Connection::open(&self.db_path).map_err(|err| {
            PlaybackOperationError::content(
                "library_unavailable",
                format!("open library db {}: {err}", self.db_path.display()),
                false,
            )
        })?;

        let row = connection
            .query_row(
                "
                SELECT
                    t.track_uid,
                    t.volume_uuid,
                    t.relative_path,
                    COALESCE(NULLIF(t.title, ''), t.filename),
                    t.duration_ms,
                    COALESCE(LOWER(t.format), ''),
                    COALESCE(v.is_available, 1),
                    v.mount_path
                FROM tracks t
                JOIN volumes v ON v.volume_uuid = t.volume_uuid
                WHERE t.track_uid = ?1
                LIMIT 1
                ",
                params![track_id],
                |row| {
                    let duration_ms = row
                        .get::<_, Option<i64>>(4)?
                        .and_then(|value| u64::try_from(value).ok());
                    let is_available = row.get::<_, i64>(6)? != 0;
                    let mount_path: String = row.get(7)?;
                    let relative_path: String = row.get(2)?;
                    Ok((
                        ResolvedTrack {
                            track_uid: row.get(0)?,
                            volume_uuid: row.get(1)?,
                            relative_path: relative_path.clone(),
                            title: row.get::<_, Option<String>>(3)?,
                            duration_ms,
                            format: row.get(5)?,
                            absolute_path: PathBuf::from(mount_path).join(&relative_path),
                        },
                        is_available,
                    ))
                },
            )
            .optional()
            .map_err(|err| {
                PlaybackOperationError::content(
                    "library_query_failed",
                    format!("resolve track {track_id} from library db: {err}"),
                    false,
                )
            })?;

        let (track, is_available) = row.ok_or_else(|| {
            PlaybackOperationError::content(
                "track_not_found",
                format!("track is not present in library.db: {track_id}"),
                true,
            )
        })?;
        if !is_available {
            return Err(PlaybackOperationError::content(
                "track_volume_unavailable",
                format!("track volume is offline: {}", track.volume_uuid),
                false,
            ));
        }

        Ok(track)
    }

    fn resolve_path_track(
        &self,
        track_id: &str,
    ) -> Result<Option<ResolvedTrack>, PlaybackOperationError> {
        let candidate = Path::new(track_id);
        if !candidate.is_absolute() {
            return Ok(None);
        }
        if !self.allow_absolute_paths {
            return Err(PlaybackOperationError::content(
                "absolute_path_playback_disabled",
                "absolute path playback is disabled; use a library track uid",
                false,
            ));
        }
        if !candidate.exists() {
            return Ok(None);
        }

        let file_name = candidate
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(track_id)
            .to_string();

        Ok(Some(ResolvedTrack {
            track_uid: track_id.to_string(),
            volume_uuid: "manual-path".to_string(),
            relative_path: track_id.to_string(),
            title: Some(file_name),
            duration_ms: None,
            format: detect_format(candidate),
            absolute_path: candidate.to_path_buf(),
        }))
    }
}

fn absolute_path_playback_enabled() -> bool {
    match std::env::var(ALLOW_ABSOLUTE_PATHS_ENV) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    }
}

impl OutputController {
    fn new(
        configured_device: Option<String>,
        alsa_cards_path: PathBuf,
        dsd_policy: DsdOutputPolicy,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(OutputState::default())),
            configured_device: configured_device.map(Arc::new),
            alsa_cards_path: Arc::new(alsa_cards_path),
            dsd_policy,
        }
    }

    fn device_label(&self) -> String {
        match self.configured_device.as_deref() {
            Some(device) => format!("configured {device}"),
            None => format!("auto USB DAC via {}", self.alsa_cards_path.display()),
        }
    }

    fn dsd_policy_label(&self) -> &'static str {
        self.dsd_policy.as_str()
    }

    fn start(
        &self,
        track: ResolvedTrack,
        state: Arc<Mutex<RuntimeState>>,
        event_hub: EventHub,
        queue_path: Arc<PathBuf>,
        history_path: Arc<PathBuf>,
        track_resolver: TrackResolver,
    ) -> Result<(), PlaybackOperationError> {
        if !track.absolute_path.exists() {
            return Err(PlaybackOperationError::content(
                "track_file_missing",
                format!(
                    "resolved media path is missing: {}",
                    track.absolute_path.display()
                ),
                true,
            ));
        }

        let (generation, stale_pid) = self.begin_generation();
        self.stop_pid_and_wait(stale_pid);

        let output_device = self.resolve_output_device()?;
        let child = self.spawn_output_child(&track, output_device.as_str())?;

        let pid = child.id();
        {
            let mut output = self.inner.lock().map_err(|_| {
                PlaybackOperationError::output("output_lock_poisoned", "output lock poisoned")
            })?;
            if output.generation != generation {
                return Ok(());
            }
            output.pid = Some(pid);
            output.track_id = Some(track.track_uid.clone());
            output.paused = false;
        }

        let controller = self.clone();
        let track_id = track.track_uid.clone();
        thread::spawn(move || {
            controller.watch_playback_process(
                generation,
                pid,
                track_id,
                child,
                state,
                event_hub,
                queue_path,
                history_path,
                track_resolver,
            );
        });

        Ok(())
    }

    fn stop(&self) {
        let (_, stale_pid) = self.begin_generation();
        self.stop_pid_and_wait(stale_pid);
    }

    fn pause(&self) -> Result<(), PlaybackOperationError> {
        let pid = {
            let output = self.inner.lock().map_err(|_| {
                PlaybackOperationError::output("output_lock_poisoned", "output lock poisoned")
            })?;
            output.pid
        };

        let Some(pid) = pid else {
            return Err(PlaybackOperationError::output(
                "no_output_process",
                "no active playback process to pause",
            ));
        };

        send_signal(pid, "STOP")?;
        if let Ok(mut output) = self.inner.lock() {
            if output.pid == Some(pid) {
                output.paused = true;
            }
        }
        Ok(())
    }

    fn resume(&self, track_id: &str) -> Result<(), PlaybackOperationError> {
        let pid = {
            let output = self.inner.lock().map_err(|_| {
                PlaybackOperationError::output("output_lock_poisoned", "output lock poisoned")
            })?;
            if output.track_id.as_deref() != Some(track_id) || !output.paused {
                return Err(PlaybackOperationError::output(
                    "resume_mismatch",
                    format!("paused output does not match current track: {track_id}"),
                ));
            }
            output.pid
        };

        let Some(pid) = pid else {
            return Err(PlaybackOperationError::output(
                "no_output_process",
                "no paused playback process to resume",
            ));
        };

        send_signal(pid, "CONT")?;
        if let Ok(mut output) = self.inner.lock() {
            if output.pid == Some(pid) {
                output.paused = false;
            }
        }
        Ok(())
    }

    fn begin_generation(&self) -> (u64, Option<u32>) {
        let mut output = self.inner.lock().expect("output lock poisoned");
        let stale_pid = output.pid.take();
        output.generation += 1;
        output.track_id = None;
        output.paused = false;
        (output.generation, stale_pid)
    }

    fn stop_pid(&self, stale_pid: Option<u32>) {
        if let Some(pid) = stale_pid {
            let _ = send_signal(pid, "TERM");
        }
    }

    fn stop_pid_and_wait(&self, stale_pid: Option<u32>) {
        let Some(pid) = stale_pid else {
            return;
        };

        self.stop_pid(Some(pid));
        if wait_for_process_exit(pid, Duration::from_millis(1200)) {
            return;
        }

        let _ = send_signal(pid, "KILL");
        let _ = wait_for_process_exit(pid, Duration::from_millis(400));
    }

    fn resolve_output_device(&self) -> Result<String, PlaybackOperationError> {
        if let Some(device) = self.configured_device.as_deref() {
            return Ok(device.to_string());
        }

        let selected = resolve_auto_usb_audio_output(&self.alsa_cards_path)?;
        println!(
            "playbackd selected USB audio output: {} ({})",
            selected.name, selected.alsa_device
        );
        Ok(selected.alsa_device)
    }

    fn spawn_output_child(
        &self,
        track: &ResolvedTrack,
        device: &str,
    ) -> Result<Child, PlaybackOperationError> {
        if is_dsd_format(track) {
            return self.spawn_dsd_aplay(track, device);
        }

        if resolved_format(track) == "wav" {
            return self.spawn_aplay_file(track, device);
        }

        self.spawn_decoded_aplay(track, device)
    }

    fn spawn_aplay_file(
        &self,
        track: &ResolvedTrack,
        device: &str,
    ) -> Result<Child, PlaybackOperationError> {
        Command::new("aplay")
            .arg("-D")
            .arg(device)
            .arg(&track.absolute_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| {
                PlaybackOperationError::output(
                    "alsa_open_failed",
                    format!(
                        "spawn aplay for {} on {}: {err}",
                        track.absolute_path.display(),
                        device
                    ),
                )
            })
    }

    fn spawn_dsd_aplay(
        &self,
        track: &ResolvedTrack,
        device: &str,
    ) -> Result<Child, PlaybackOperationError> {
        let plan = select_dsd_playback_plan(track, device, self.dsd_policy)?;
        let mut child = Command::new("aplay")
            .arg("-D")
            .arg(plan.device.as_str())
            .arg("-t")
            .arg("raw")
            .arg("-f")
            .arg(plan.aplay_format())
            .arg("-c")
            .arg(plan.channels.to_string())
            .arg("-r")
            .arg(plan.sample_rate.to_string())
            .arg("--disable-resample")
            .arg("--disable-channels")
            .arg("--disable-format")
            .arg("--disable-softvol")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| {
                PlaybackOperationError::output(
                    "alsa_open_failed",
                    format!(
                        "spawn DSD aplay for {} on {} ({}): {err}",
                        track.absolute_path.display(),
                        plan.device,
                        plan.aplay_format()
                    ),
                )
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            PlaybackOperationError::output(
                "alsa_pipe_unavailable",
                format!("dsd aplay stdin is unavailable for {}", track.track_uid),
            )
        })?;

        let path = track.absolute_path.clone();
        let track_id = track.track_uid.clone();
        let transport = plan.transport;
        thread::spawn(move || {
            if let Err(err) = stream_dsd_audio(path, stdin, transport) {
                eprintln!("playbackd dsd stream error for {track_id}: {err}");
            }
        });

        Ok(child)
    }

    fn spawn_decoded_aplay(
        &self,
        track: &ResolvedTrack,
        device: &str,
    ) -> Result<Child, PlaybackOperationError> {
        let pcm = inspect_decoded_pcm(track)?;
        let mut child = Command::new("aplay")
            .arg("-D")
            .arg(device)
            .arg("-t")
            .arg("raw")
            .arg("-f")
            .arg("S16_LE")
            .arg("-c")
            .arg(pcm.channels.to_string())
            .arg("-r")
            .arg(pcm.sample_rate.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| {
                PlaybackOperationError::output(
                    "alsa_open_failed",
                    format!(
                        "spawn decoded aplay for {} on {}: {err}",
                        track.absolute_path.display(),
                        device
                    ),
                )
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            PlaybackOperationError::output(
                "alsa_pipe_unavailable",
                format!("decoded aplay stdin is unavailable for {}", track.track_uid),
            )
        })?;

        let path = track.absolute_path.clone();
        let track_id = track.track_uid.clone();
        thread::spawn(move || {
            if let Err(err) = stream_decoded_audio(path, stdin) {
                eprintln!("playbackd decoder stream error for {track_id}: {err}");
            }
        });

        Ok(child)
    }

    fn watch_playback_process(
        &self,
        generation: u64,
        pid: u32,
        track_id: String,
        mut child: std::process::Child,
        state: Arc<Mutex<RuntimeState>>,
        event_hub: EventHub,
        queue_path: Arc<PathBuf>,
        history_path: Arc<PathBuf>,
        track_resolver: TrackResolver,
    ) {
        let status = child.wait();

        let should_update = {
            let mut output = match self.inner.lock() {
                Ok(output) => output,
                Err(_) => return,
            };
            if output.generation != generation || output.pid != Some(pid) {
                return;
            }
            output.pid = None;
            output.track_id = None;
            output.paused = false;
            true
        };
        if !should_update {
            return;
        }

        match status {
            Ok(status) if status.success() => {
                handle_finished_track_success(
                    self,
                    state,
                    &event_hub,
                    queue_path,
                    history_path,
                    track_resolver,
                    &track_id,
                );
            }
            Ok(status) => {
                apply_async_operation_failure(
                    state,
                    &event_hub,
                    Some(track_id.as_str()),
                    PlaybackOperationError::output(
                        "alsa_playback_failed",
                        format!("aplay exited {}", exit_status_label(&status)),
                    ),
                );
            }
            Err(err) => {
                apply_async_operation_failure(
                    state,
                    &event_hub,
                    Some(track_id.as_str()),
                    PlaybackOperationError::output(
                        "alsa_wait_failed",
                        format!("wait aplay: {err}"),
                    ),
                );
            }
        }
    }
}

impl RuntimeState {
    fn new() -> Self {
        Self {
            snapshot: QueueSnapshot::empty(),
            history_log: HistoryLog::empty(),
            playback_state: PlaybackState::Idle,
            current_track: None,
            last_command: None,
            next_queue_entry_id: 1,
        }
    }

    fn restore(snapshot: QueueSnapshot, history_log: HistoryLog) -> Self {
        let snapshot = normalize_snapshot(snapshot);
        let playback_state = if snapshot.tracks.is_empty() {
            PlaybackState::Idle
        } else {
            PlaybackState::Stopped
        };
        let current_track = resolve_track_uid(&snapshot, snapshot.current_order_index);
        let next_queue_entry_id = next_queue_entry_id(&snapshot);

        Self {
            snapshot,
            history_log,
            playback_state,
            current_track,
            last_command: None,
            next_queue_entry_id,
        }
    }

    fn apply_command(&mut self, command: PlaybackCommand) -> CommandOutcome {
        match command {
            PlaybackCommand::Ping => CommandOutcome {
                response_line: format_ack_line(
                    "ping",
                    self.playback_state,
                    self.current_track.as_deref(),
                ),
                events: Vec::new(),
                persist_queue: false,
                persist_history: false,
            },
            PlaybackCommand::Status => CommandOutcome {
                response_line: format_status_line(&self.status_snapshot()),
                events: Vec::new(),
                persist_queue: false,
                persist_history: false,
            },
            PlaybackCommand::QueueSnapshot => CommandOutcome {
                response_line: format_queue_snapshot_line(&self.queue_snapshot_view()),
                events: Vec::new(),
                persist_queue: false,
                persist_history: false,
            },
            PlaybackCommand::HistorySnapshot => CommandOutcome {
                response_line: format_history_snapshot_line(&self.history_snapshot_view()),
                events: Vec::new(),
                persist_queue: false,
                persist_history: false,
            },
            PlaybackCommand::Play(track_id) => {
                if self.playback_state == PlaybackState::Paused
                    && self.current_track.as_deref() == Some(track_id.as_str())
                {
                    self.playback_state = PlaybackState::QuietActive;
                    self.last_command = Some(format!("resume:{track_id}"));
                    return CommandOutcome {
                        response_line: format_ack_line(
                            "play",
                            self.playback_state,
                            self.current_track.as_deref(),
                        ),
                        events: vec![PlaybackEvent::PlaybackResumed],
                        persist_queue: false,
                        persist_history: false,
                    };
                }

                self.enqueue_and_activate(track_id.clone(), "play");
                self.record_current_track_history();
                CommandOutcome {
                    response_line: format_ack_line("play", self.playback_state, Some(&track_id)),
                    events: vec![
                        PlaybackEvent::PlayRequestAccepted {
                            track_id: track_id.clone(),
                        },
                        PlaybackEvent::PlaybackStarted { track_id },
                    ],
                    persist_queue: true,
                    persist_history: true,
                }
            }
            PlaybackCommand::QueuePlay(track_ids) => self.queue_play(track_ids),
            PlaybackCommand::QueueAppend(track_id) => {
                self.queue_append(track_id.clone());
                CommandOutcome {
                    response_line: format_ack_line(
                        "queue_append",
                        self.playback_state,
                        self.current_track.as_deref(),
                    ),
                    events: Vec::new(),
                    persist_queue: true,
                    persist_history: false,
                }
            }
            PlaybackCommand::QueueInsertNext(track_id) => {
                self.queue_insert_next(track_id.clone());
                CommandOutcome {
                    response_line: format_ack_line(
                        "queue_insert_next",
                        self.playback_state,
                        self.current_track.as_deref(),
                    ),
                    events: Vec::new(),
                    persist_queue: true,
                    persist_history: false,
                }
            }
            PlaybackCommand::QueueRemove(queue_entry_id) => self.queue_remove(&queue_entry_id),
            PlaybackCommand::QueueClear => self.queue_clear(),
            PlaybackCommand::QueueReplace(track_ids) => self.queue_replace(track_ids),
            PlaybackCommand::SetOrderMode(mode) => self.set_order_mode(&mode),
            PlaybackCommand::SetRepeatMode(mode) => self.set_repeat_mode(&mode),
            PlaybackCommand::PlayHistory(track_id) => {
                self.play_history_now(track_id.clone());
                self.record_current_track_history();
                CommandOutcome {
                    response_line: format_ack_line(
                        "play_history",
                        self.playback_state,
                        Some(&track_id),
                    ),
                    events: vec![
                        PlaybackEvent::PlayRequestAccepted {
                            track_id: track_id.clone(),
                        },
                        PlaybackEvent::PlaybackStarted { track_id },
                    ],
                    persist_queue: true,
                    persist_history: true,
                }
            }
            PlaybackCommand::Pause => {
                if !matches!(
                    self.playback_state,
                    PlaybackState::PreQuiet | PlaybackState::QuietActive
                ) {
                    return CommandOutcome {
                        response_line: format_error_line("no_active_track", "nothing_is_playing"),
                        events: Vec::new(),
                        persist_queue: false,
                        persist_history: false,
                    };
                }

                self.playback_state = PlaybackState::Paused;
                self.last_command = Some("pause".to_string());
                CommandOutcome {
                    response_line: format_ack_line(
                        "pause",
                        self.playback_state,
                        self.current_track.as_deref(),
                    ),
                    events: vec![PlaybackEvent::PlaybackPaused],
                    persist_queue: false,
                    persist_history: false,
                }
            }
            PlaybackCommand::Stop => {
                self.playback_state = PlaybackState::Stopped;
                self.current_track = self.resolved_current_track();
                self.last_command = Some("stop".to_string());
                CommandOutcome {
                    response_line: format_ack_line(
                        "stop",
                        self.playback_state,
                        self.current_track.as_deref(),
                    ),
                    events: vec![PlaybackEvent::PlaybackStopped {
                        reason: "user_stop".to_string(),
                    }],
                    persist_queue: false,
                    persist_history: false,
                }
            }
            PlaybackCommand::Next => self.step(true),
            PlaybackCommand::Prev => self.step(false),
        }
    }

    fn play_history_now(&mut self, track_id: String) {
        self.replace_current_queue_track(track_id.clone());
        self.playback_state = PlaybackState::QuietActive;
        self.current_track = self.resolved_current_track();
        self.last_command = Some(format!("play_history:{track_id}"));
    }

    fn replace_current_queue_track(&mut self, track_id: String) {
        let target_order_index = self
            .snapshot
            .current_order_index
            .filter(|index| *index < self.snapshot.play_order.len())
            .unwrap_or(0);

        if self.snapshot.play_order.is_empty() {
            let queue_entry_id = self.push_track_entry(track_id);
            self.snapshot.play_order.push(queue_entry_id);
            self.snapshot.current_order_index = Some(0);
            return;
        }

        self.snapshot.current_order_index = Some(target_order_index);

        let queue_entry_id = self.snapshot.play_order[target_order_index].clone();
        let entry = self
            .snapshot
            .tracks
            .iter_mut()
            .find(|entry| entry.queue_entry_id == queue_entry_id)
            .expect("play_order must reference an existing queue entry");

        entry.track_uid = track_id.clone();
        entry.volume_uuid = "manual".to_string();
        entry.relative_path = track_id.clone();
        entry.title = Some(track_id);
        entry.duration_ms = None;
    }

    fn enqueue_and_activate(&mut self, track_id: String, action: &str) {
        let queue_entry_id = self.push_track_entry(track_id.clone());
        self.snapshot.play_order.push(queue_entry_id);
        self.snapshot.current_order_index = Some(self.snapshot.play_order.len().saturating_sub(1));

        self.playback_state = PlaybackState::QuietActive;
        self.current_track = self.resolved_current_track();
        self.last_command = Some(format!("{action}:{track_id}"));
    }

    fn queue_append(&mut self, track_id: String) {
        let queue_entry_id = self.push_track_entry(track_id.clone());
        self.snapshot.play_order.push(queue_entry_id);
        self.ensure_selected_track_for_non_empty_queue();
        self.last_command = Some(format!("queue_append:{track_id}"));
    }

    fn queue_play(&mut self, track_ids: Vec<String>) -> CommandOutcome {
        if track_ids.is_empty() {
            return CommandOutcome {
                response_line: format_error_line("empty_queue", "no_track_ids"),
                events: Vec::new(),
                persist_queue: false,
                persist_history: false,
            };
        }

        let order_mode = self.snapshot.order_mode;
        let repeat_mode = self.snapshot.repeat_mode;
        self.snapshot = QueueSnapshot {
            order_mode,
            repeat_mode,
            current_order_index: None,
            play_order: Vec::new(),
            tracks: Vec::new(),
        };
        self.next_queue_entry_id = 1;

        for track_id in &track_ids {
            let queue_entry_id = self.push_track_entry(track_id.clone());
            self.snapshot.play_order.push(queue_entry_id);
        }

        self.snapshot.current_order_index = Some(0);
        let current_queue_entry_id = self.snapshot.play_order.first().cloned();
        self.rebuild_play_order_for_mode(current_queue_entry_id);
        let track_id = track_ids[0].clone();
        self.playback_state = PlaybackState::QuietActive;
        self.current_track = self.resolved_current_track();
        self.last_command = Some(format!("queue_play:{track_id}"));
        self.record_current_track_history();

        CommandOutcome {
            response_line: format_ack_line("queue_play", self.playback_state, Some(&track_id)),
            events: vec![
                PlaybackEvent::PlayRequestAccepted {
                    track_id: track_id.clone(),
                },
                PlaybackEvent::PlaybackStarted { track_id },
            ],
            persist_queue: true,
            persist_history: true,
        }
    }

    fn queue_insert_next(&mut self, track_id: String) {
        let queue_entry_id = self.push_track_entry(track_id.clone());
        let insert_index = self
            .snapshot
            .current_order_index
            .map(|index| (index + 1).min(self.snapshot.play_order.len()))
            .unwrap_or(self.snapshot.play_order.len());
        self.snapshot
            .play_order
            .insert(insert_index, queue_entry_id);
        self.ensure_selected_track_for_non_empty_queue();
        self.last_command = Some(format!("queue_insert_next:{track_id}"));
    }

    fn queue_remove(&mut self, queue_entry_id: &str) -> CommandOutcome {
        let remove_index = match self
            .snapshot
            .play_order
            .iter()
            .position(|entry_id| entry_id == queue_entry_id)
        {
            Some(index) => index,
            None => {
                return CommandOutcome {
                    response_line: format_error_line(
                        "queue_entry_not_found",
                        "queue_entry_id_not_found",
                    ),
                    events: Vec::new(),
                    persist_queue: false,
                    persist_history: false,
                };
            }
        };
        let current_index = self.snapshot.current_order_index;
        let active_output = self.has_active_output();
        let mut persist_history = false;

        self.snapshot.play_order.remove(remove_index);
        self.snapshot
            .tracks
            .retain(|entry| entry.queue_entry_id != queue_entry_id);

        let mut events = Vec::new();
        if self.snapshot.play_order.is_empty() {
            self.snapshot.current_order_index = None;
            self.current_track = None;
            self.playback_state = PlaybackState::Idle;
            if active_output {
                events.push(PlaybackEvent::PlaybackStopped {
                    reason: "queue_item_removed".to_string(),
                });
            }
        } else {
            match current_index {
                Some(index) if remove_index < index => {
                    self.snapshot.current_order_index = Some(index - 1);
                    self.current_track = self.resolved_current_track();
                }
                Some(index) if remove_index == index => {
                    if active_output && remove_index < self.snapshot.play_order.len() {
                        self.snapshot.current_order_index = Some(remove_index);
                        self.current_track = self.resolved_current_track();
                        self.playback_state = PlaybackState::QuietActive;
                        self.record_current_track_history();
                        persist_history = true;
                        if let Some(track_id) = self.current_track.clone() {
                            events.push(PlaybackEvent::TrackChanged { track_id });
                        }
                    } else {
                        let fallback_index = if remove_index < self.snapshot.play_order.len() {
                            remove_index
                        } else {
                            self.snapshot.play_order.len().saturating_sub(1)
                        };
                        self.snapshot.current_order_index = Some(fallback_index);
                        self.current_track = self.resolved_current_track();
                        self.playback_state = PlaybackState::Stopped;
                        if active_output {
                            events.push(PlaybackEvent::PlaybackStopped {
                                reason: "queue_item_removed".to_string(),
                            });
                        }
                    }
                }
                Some(index) => {
                    self.snapshot.current_order_index = Some(index);
                    self.current_track = self.resolved_current_track();
                }
                None => self.ensure_selected_track_for_non_empty_queue(),
            }

            if !active_output && self.playback_state == PlaybackState::Idle {
                self.playback_state = PlaybackState::Stopped;
            }
        }

        self.last_command = Some(format!("queue_remove:{queue_entry_id}"));
        CommandOutcome {
            response_line: format_ack_line(
                "queue_remove",
                self.playback_state,
                self.current_track.as_deref(),
            ),
            events,
            persist_queue: true,
            persist_history,
        }
    }

    fn queue_clear(&mut self) -> CommandOutcome {
        let active_output = self.has_active_output();
        self.snapshot = QueueSnapshot::empty();
        self.next_queue_entry_id = 1;
        self.current_track = None;
        self.playback_state = PlaybackState::Idle;
        self.last_command = Some("queue_clear".to_string());

        CommandOutcome {
            response_line: format_ack_line("queue_clear", self.playback_state, None),
            events: if active_output {
                vec![PlaybackEvent::PlaybackStopped {
                    reason: "queue_cleared".to_string(),
                }]
            } else {
                Vec::new()
            },
            persist_queue: true,
            persist_history: false,
        }
    }

    fn queue_replace(&mut self, track_ids: Vec<String>) -> CommandOutcome {
        let active_output = self.has_active_output();
        self.snapshot = QueueSnapshot {
            order_mode: self.snapshot.order_mode,
            repeat_mode: self.snapshot.repeat_mode,
            current_order_index: None,
            play_order: Vec::new(),
            tracks: Vec::new(),
        };
        self.next_queue_entry_id = 1;

        for track_id in &track_ids {
            let queue_entry_id = self.push_track_entry(track_id.clone());
            self.snapshot.play_order.push(queue_entry_id);
        }

        if self.snapshot.play_order.is_empty() {
            self.playback_state = PlaybackState::Idle;
            self.current_track = None;
        } else {
            self.snapshot.current_order_index = Some(0);
            let current_queue_entry_id = self.snapshot.play_order.first().cloned();
            self.rebuild_play_order_for_mode(current_queue_entry_id);
            self.playback_state = PlaybackState::Stopped;
            self.current_track = self.resolved_current_track();
        }

        self.last_command = Some(format!("queue_replace:{}", self.snapshot.play_order.len()));
        CommandOutcome {
            response_line: format_ack_line(
                "queue_replace",
                self.playback_state,
                self.current_track.as_deref(),
            ),
            events: if active_output {
                vec![PlaybackEvent::PlaybackStopped {
                    reason: "queue_replaced".to_string(),
                }]
            } else {
                Vec::new()
            },
            persist_queue: true,
            persist_history: false,
        }
    }

    fn set_order_mode(&mut self, raw_mode: &str) -> CommandOutcome {
        let Some(mode) = parse_order_mode_value(raw_mode) else {
            return CommandOutcome {
                response_line: format_error_line("invalid_order_mode", "unsupported_order_mode"),
                events: Vec::new(),
                persist_queue: false,
                persist_history: false,
            };
        };

        let current_queue_entry_id = self.current_queue_entry_id();
        self.snapshot.order_mode = mode;
        self.rebuild_play_order_for_mode(current_queue_entry_id);
        self.current_track = self.resolved_current_track();
        self.last_command = Some(format!("set_order_mode:{}", order_mode_label(mode)));

        CommandOutcome {
            response_line: format_ack_line(
                "set_order_mode",
                self.playback_state,
                self.current_track.as_deref(),
            ),
            events: Vec::new(),
            persist_queue: true,
            persist_history: false,
        }
    }

    fn set_repeat_mode(&mut self, raw_mode: &str) -> CommandOutcome {
        let Some(mode) = parse_repeat_mode_value(raw_mode) else {
            return CommandOutcome {
                response_line: format_error_line("invalid_repeat_mode", "unsupported_repeat_mode"),
                events: Vec::new(),
                persist_queue: false,
                persist_history: false,
            };
        };

        self.snapshot.repeat_mode = mode;
        self.last_command = Some(format!("set_repeat_mode:{}", repeat_mode_label(mode)));

        CommandOutcome {
            response_line: format_ack_line(
                "set_repeat_mode",
                self.playback_state,
                self.current_track.as_deref(),
            ),
            events: Vec::new(),
            persist_queue: true,
            persist_history: false,
        }
    }

    fn current_queue_entry_id(&self) -> Option<String> {
        self.snapshot
            .current_order_index
            .and_then(|index| self.snapshot.play_order.get(index))
            .cloned()
    }

    fn rebuild_play_order_for_mode(&mut self, current_queue_entry_id: Option<String>) {
        let mut play_order = self
            .snapshot
            .tracks
            .iter()
            .map(|entry| entry.queue_entry_id.clone())
            .collect::<Vec<_>>();

        if self.snapshot.order_mode == OrderMode::Shuffle {
            shuffle_queue_entry_ids(&mut play_order);
            if let Some(current_id) = current_queue_entry_id.as_deref() {
                if let Some(index) = play_order
                    .iter()
                    .position(|entry_id| entry_id == current_id)
                {
                    let current_id = play_order.remove(index);
                    play_order.insert(0, current_id);
                }
            }
        }

        self.snapshot.play_order = play_order;
        self.snapshot.current_order_index = current_queue_entry_id
            .as_deref()
            .and_then(|current_id| {
                self.snapshot
                    .play_order
                    .iter()
                    .position(|entry_id| entry_id == current_id)
            })
            .or_else(|| (!self.snapshot.play_order.is_empty()).then_some(0));
    }

    fn step(&mut self, forward: bool) -> CommandOutcome {
        if self.snapshot.play_order.is_empty() {
            return CommandOutcome {
                response_line: format_error_line("empty_queue", "no_queue_entries"),
                events: Vec::new(),
                persist_queue: false,
                persist_history: false,
            };
        }

        let next_index = match (self.snapshot.current_order_index, forward) {
            (None, _) => 0,
            (Some(index), true) if index + 1 < self.snapshot.play_order.len() => index + 1,
            (Some(index), false) if index > 0 => index - 1,
            _ => {
                self.playback_state = PlaybackState::Stopped;
                self.current_track = self.resolved_current_track();
                self.last_command = Some(if forward {
                    "next:queue_end".to_string()
                } else {
                    "prev:queue_start".to_string()
                });
                return CommandOutcome {
                    response_line: format_ack_line(
                        if forward { "next" } else { "prev" },
                        self.playback_state,
                        self.current_track.as_deref(),
                    ),
                    events: vec![PlaybackEvent::PlaybackStopped {
                        reason: if forward {
                            "queue_end".to_string()
                        } else {
                            "queue_start".to_string()
                        },
                    }],
                    persist_queue: false,
                    persist_history: false,
                };
            }
        };

        self.snapshot.current_order_index = Some(next_index);
        let track = self
            .resolved_current_track()
            .unwrap_or_else(|| "unknown_track".to_string());
        self.current_track = Some(track.clone());
        self.playback_state = PlaybackState::QuietActive;
        self.last_command = Some(format!("{}:{track}", if forward { "next" } else { "prev" }));
        self.record_current_track_history();

        CommandOutcome {
            response_line: format_ack_line(
                if forward { "next" } else { "prev" },
                self.playback_state,
                Some(&track),
            ),
            events: vec![PlaybackEvent::TrackChanged { track_id: track }],
            persist_queue: true,
            persist_history: true,
        }
    }

    fn status_snapshot(&self) -> PlaybackStatusSnapshot {
        PlaybackStatusSnapshot {
            state: self.playback_state,
            order_mode: order_mode_label(self.snapshot.order_mode).to_string(),
            repeat_mode: repeat_mode_label(self.snapshot.repeat_mode).to_string(),
            current_track: self.current_track.clone(),
            last_command: self.last_command.clone(),
            queue_entries: self.snapshot.tracks.len(),
        }
    }

    fn queue_snapshot_view(&self) -> QueueSnapshotView {
        let entries = self
            .snapshot
            .play_order
            .iter()
            .enumerate()
            .filter_map(|(order_index, queue_entry_id)| {
                self.snapshot
                    .tracks
                    .iter()
                    .find(|entry| &entry.queue_entry_id == queue_entry_id)
                    .map(|entry| QueueSnapshotEntryView {
                        order_index,
                        queue_entry_id: entry.queue_entry_id.clone(),
                        track_uid: entry.track_uid.clone(),
                        volume_uuid: entry.volume_uuid.clone(),
                        relative_path: entry.relative_path.clone(),
                        title: entry.title.clone(),
                        duration_ms: entry.duration_ms,
                        is_current: self.snapshot.current_order_index == Some(order_index),
                    })
            })
            .collect();

        QueueSnapshotView {
            order_mode: order_mode_label(self.snapshot.order_mode).to_string(),
            repeat_mode: repeat_mode_label(self.snapshot.repeat_mode).to_string(),
            current_order_index: self.snapshot.current_order_index,
            entries,
        }
    }

    fn history_snapshot_view(&self) -> HistorySnapshotView {
        let entries = self
            .history_log
            .entries
            .iter()
            .map(|entry| HistorySnapshotEntryView {
                played_at: entry.played_at,
                track_uid: entry.track_uid.clone(),
                volume_uuid: entry.volume_uuid.clone(),
                relative_path: entry.relative_path.clone(),
                title: entry.title.clone(),
                duration_ms: entry.duration_ms,
            })
            .collect();

        HistorySnapshotView { entries }
    }

    fn resolved_current_track(&self) -> Option<String> {
        resolve_track_uid(&self.snapshot, self.snapshot.current_order_index)
    }

    fn push_track_entry(&mut self, track_id: String) -> String {
        let queue_entry_id = format!("q{}", self.next_queue_entry_id);
        self.next_queue_entry_id += 1;

        self.snapshot.tracks.push(QueueEntry {
            queue_entry_id: queue_entry_id.clone(),
            track_uid: track_id.clone(),
            volume_uuid: "manual".to_string(),
            relative_path: track_id.clone(),
            title: Some(track_id),
            duration_ms: None,
        });

        queue_entry_id
    }

    fn ensure_selected_track_for_non_empty_queue(&mut self) {
        if self.snapshot.play_order.is_empty() {
            self.snapshot.current_order_index = None;
            self.current_track = None;
            self.playback_state = PlaybackState::Idle;
            return;
        }

        if self.snapshot.current_order_index.is_none() {
            self.snapshot.current_order_index = Some(0);
        }

        self.current_track = self.resolved_current_track();
        if !self.has_active_output() {
            self.playback_state = PlaybackState::Stopped;
        }
    }

    fn has_active_output(&self) -> bool {
        matches!(
            self.playback_state,
            PlaybackState::PreQuiet | PlaybackState::QuietActive
        )
    }

    fn record_current_track_history(&mut self) {
        if let Some(entry) = self.current_history_entry() {
            self.history_log.push_recent(entry, HISTORY_LIMIT);
        }
    }

    fn sync_recent_history_entry_from_track(&mut self, track: &ResolvedTrack) -> bool {
        let Some(entry) = self.history_log.entries.first_mut() else {
            return false;
        };
        if entry.track_uid != track.track_uid {
            return false;
        }

        let mut changed = false;
        if entry.volume_uuid != track.volume_uuid {
            entry.volume_uuid = track.volume_uuid.clone();
            changed = true;
        }
        if entry.relative_path != track.relative_path {
            entry.relative_path = track.relative_path.clone();
            changed = true;
        }
        if entry.title != track.title {
            entry.title = track.title.clone();
            changed = true;
        }
        if entry.duration_ms != track.duration_ms {
            entry.duration_ms = track.duration_ms;
            changed = true;
        }

        changed
    }

    fn rollback_recent_history_for_track(&mut self, track_id: &str) -> bool {
        let Some(first_entry) = self.history_log.entries.first() else {
            return false;
        };
        if first_entry.track_uid != track_id {
            return false;
        }

        let Some(current_entry) = self.current_history_entry() else {
            return false;
        };
        if first_entry.volume_uuid != current_entry.volume_uuid
            || first_entry.relative_path != current_entry.relative_path
            || first_entry.title != current_entry.title
            || first_entry.duration_ms != current_entry.duration_ms
        {
            return false;
        }

        self.history_log.entries.remove(0);
        true
    }

    fn current_history_entry(&self) -> Option<HistoryEntry> {
        let queue_entry = resolve_queue_entry(&self.snapshot, self.snapshot.current_order_index)?;
        Some(HistoryEntry {
            played_at: unix_timestamp_secs(),
            track_uid: queue_entry.track_uid,
            volume_uuid: queue_entry.volume_uuid,
            relative_path: queue_entry.relative_path,
            title: queue_entry.title,
            duration_ms: queue_entry.duration_ms,
        })
    }

    fn enrich_current_track(&mut self, track: &ResolvedTrack) -> bool {
        let Some(current_queue_entry_id) = self
            .snapshot
            .current_order_index
            .and_then(|index| self.snapshot.play_order.get(index))
            .cloned()
        else {
            return false;
        };

        let Some(entry) = self
            .snapshot
            .tracks
            .iter_mut()
            .find(|entry| entry.queue_entry_id == current_queue_entry_id)
        else {
            return false;
        };

        let mut changed = false;
        if entry.track_uid != track.track_uid {
            entry.track_uid = track.track_uid.clone();
            changed = true;
        }
        if entry.volume_uuid != track.volume_uuid {
            entry.volume_uuid = track.volume_uuid.clone();
            changed = true;
        }
        if entry.relative_path != track.relative_path {
            entry.relative_path = track.relative_path.clone();
            changed = true;
        }
        if entry.title != track.title {
            entry.title = track.title.clone();
            changed = true;
        }
        if entry.duration_ms != track.duration_ms {
            entry.duration_ms = track.duration_ms;
            changed = true;
        }

        let _ = self.sync_recent_history_entry_from_track(track);
        changed
    }

    fn apply_failure_state(&mut self, track_id: &str, err: &PlaybackOperationError) -> bool {
        if self.current_track.as_deref() != Some(track_id) {
            return false;
        }
        if !matches!(
            self.playback_state,
            PlaybackState::QuietActive | PlaybackState::Paused
        ) {
            return false;
        }

        self.playback_state = match err.class {
            PlaybackFailureClass::Output => PlaybackState::Stopped,
            PlaybackFailureClass::Content if err.keep_quiet => PlaybackState::QuietErrorHold,
            PlaybackFailureClass::Content => PlaybackState::Stopped,
        };
        self.last_command = Some(format!("playback_failed:{track_id}:{}", err.reason));
        true
    }

    fn finish_track_output(&mut self, track_id: &str) -> TrackFinishAction {
        if self.current_track.as_deref() != Some(track_id) {
            return TrackFinishAction::None;
        }
        if self.playback_state != PlaybackState::QuietActive {
            return TrackFinishAction::None;
        }

        if let Some(next_index) = self.next_index_after_track_finish() {
            self.snapshot.current_order_index = Some(next_index);
            let Some(next_track) = self.resolved_current_track() else {
                self.playback_state = PlaybackState::Stopped;
                self.current_track = None;
                self.last_command = Some(format!("finished:{track_id}:missing_next"));
                return TrackFinishAction::Stop {
                    reason: "track_finished".to_string(),
                };
            };

            self.current_track = Some(next_track.clone());
            self.playback_state = PlaybackState::QuietActive;
            self.last_command = Some(format!("auto_next:{next_track}"));
            self.record_current_track_history();
            return TrackFinishAction::Start {
                track_id: next_track,
            };
        }

        self.playback_state = PlaybackState::Stopped;
        self.last_command = Some(format!("finished:{track_id}"));
        TrackFinishAction::Stop {
            reason: "track_finished".to_string(),
        }
    }

    fn next_index_after_track_finish(&self) -> Option<usize> {
        let current_index = self.snapshot.current_order_index?;
        let queue_len = self.snapshot.play_order.len();
        if queue_len == 0 || current_index >= queue_len {
            return None;
        }

        match self.snapshot.repeat_mode {
            RepeatMode::Off => (current_index + 1 < queue_len).then_some(current_index + 1),
            RepeatMode::One => Some(current_index),
            RepeatMode::All => Some((current_index + 1) % queue_len),
        }
    }
}

#[derive(Clone, Debug)]
struct EventHub {
    subscribers: Arc<Mutex<Vec<UnixStream>>>,
}

impl EventHub {
    fn bind(path: &Path) -> Result<Self, String> {
        prepare_socket_path(path)?;
        let listener =
            UnixListener::bind(path).map_err(|err| format!("bind {}: {err}", path.display()))?;
        let subscribers = Arc::new(Mutex::new(Vec::new()));
        let acceptors = Arc::clone(&subscribers);

        thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        if let Err(err) = stream.set_nonblocking(true) {
                            eprintln!("playbackd event subscriber nonblocking error: {err}");
                            continue;
                        }

                        match acceptors.lock() {
                            Ok(mut subscribers) => subscribers.push(stream),
                            Err(_) => {
                                eprintln!("playbackd event subscriber lock poisoned");
                                break;
                            }
                        }
                    }
                    Err(err) => eprintln!("playbackd event accept error: {err}"),
                }
            }
        });

        Ok(Self { subscribers })
    }

    fn broadcast(&self, event: &PlaybackEvent) -> Result<(), String> {
        let line = format_event_line(event);
        let mut subscribers = self
            .subscribers
            .lock()
            .map_err(|_| "event subscriber lock poisoned".to_string())?;

        subscribers.retain_mut(|stream| {
            stream.write_all(line.as_bytes()).is_ok() && stream.write_all(b"\n").is_ok()
        });

        Ok(())
    }
}

fn handle_client(
    stream: UnixStream,
    state: Arc<Mutex<RuntimeState>>,
    event_hub: EventHub,
    queue_path: Arc<PathBuf>,
    history_path: Arc<PathBuf>,
    track_resolver: TrackResolver,
    output_controller: OutputController,
) -> Result<(), String> {
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|err| format!("clone client stream: {err}"))?,
    );
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .map_err(|err| format!("read command: {err}"))?;
    if read == 0 {
        return Err("client disconnected before sending a command".to_string());
    }

    let command = match parse_command_line(&line) {
        Ok(command) => command,
        Err(err) => {
            let response = format_error_line(err.code, &err.message);
            let mut stream = stream;
            stream
                .write_all(format!("{response}\n").as_bytes())
                .map_err(|write_err| format!("write parse failure: {write_err}"))?;
            return Ok(());
        }
    };

    let preflight_track_id = {
        let state = state
            .lock()
            .map_err(|_| "playback state lock poisoned".to_string())?;
        playback_track_id_for_preflight(&state, &command)
    };
    if let Some(track_id) = preflight_track_id {
        if let Err(err) = preflight_track_request(&track_resolver, &track_id) {
            let response = format_error_line(err.code, &err.reason);
            let mut stream = stream;
            stream
                .write_all(format!("{response}\n").as_bytes())
                .map_err(|write_err| format!("write preflight failure: {write_err}"))?;
            return Ok(());
        }
    }

    let (mut outcome, mut snapshot_to_persist, output_action) = {
        let mut state = state
            .lock()
            .map_err(|_| "playback state lock poisoned".to_string())?;
        let outcome = state.apply_command(command);
        let output_action = derive_output_action(&state, &outcome);
        let snapshot = outcome.persist_queue.then(|| state.snapshot.clone());
        (outcome, snapshot, output_action)
    };

    apply_output_action(
        Arc::clone(&state),
        &event_hub,
        &output_controller,
        &track_resolver,
        Arc::clone(&queue_path),
        Arc::clone(&history_path),
        output_action,
        &mut outcome,
        &mut snapshot_to_persist,
    );

    let history_to_persist = if outcome.persist_history {
        state.lock().ok().map(|runtime| runtime.history_log.clone())
    } else {
        None
    };

    if let Some(snapshot) = snapshot_to_persist {
        persist_queue_snapshot(queue_path.as_ref(), &snapshot)?;
    }
    if let Some(history_log) = history_to_persist {
        persist_history_log(history_path.as_ref(), &history_log)?;
    }

    let mut stream = stream;
    stream
        .write_all(format!("{}\n", outcome.response_line).as_bytes())
        .map_err(|err| format!("write response: {err}"))?;

    for event in &outcome.events {
        if let Err(err) = event_hub.broadcast(event) {
            eprintln!("playbackd event broadcast error: {err}");
        }
    }

    Ok(())
}

fn derive_output_action(state: &RuntimeState, outcome: &CommandOutcome) -> OutputAction {
    for event in &outcome.events {
        match event {
            PlaybackEvent::PlaybackStarted { track_id } => {
                return OutputAction::Start(track_id.clone())
            }
            PlaybackEvent::TrackChanged { track_id } => {
                return OutputAction::Start(track_id.clone())
            }
            PlaybackEvent::PlaybackPaused => return OutputAction::Pause,
            PlaybackEvent::PlaybackResumed => {
                if let Some(track_id) = state.current_track.clone() {
                    return OutputAction::Resume(track_id);
                }
            }
            PlaybackEvent::PlaybackStopped { .. } => return OutputAction::Stop,
            PlaybackEvent::PlayRequestAccepted { .. } | PlaybackEvent::PlaybackFailed { .. } => {}
        }
    }

    OutputAction::None
}

fn apply_output_action(
    state: Arc<Mutex<RuntimeState>>,
    event_hub: &EventHub,
    output_controller: &OutputController,
    track_resolver: &TrackResolver,
    queue_path: Arc<PathBuf>,
    history_path: Arc<PathBuf>,
    output_action: OutputAction,
    outcome: &mut CommandOutcome,
    snapshot_to_persist: &mut Option<QueueSnapshot>,
) {
    match output_action {
        OutputAction::None => {}
        OutputAction::Stop => output_controller.stop(),
        OutputAction::Pause => {
            if let Err(err) = output_controller.pause() {
                apply_operation_failure(state, outcome, None, err);
            }
        }
        OutputAction::Resume(track_id) => {
            if let Err(err) = output_controller.resume(&track_id) {
                apply_operation_failure(state, outcome, Some(track_id), err);
            }
        }
        OutputAction::Start(track_id) => {
            let track = match track_resolver.resolve(&track_id) {
                Ok(track) => track,
                Err(err) => {
                    apply_operation_failure(state, outcome, Some(track_id), err);
                    return;
                }
            };

            if let Ok(mut runtime) = state.lock() {
                if runtime.enrich_current_track(&track) {
                    *snapshot_to_persist = Some(runtime.snapshot.clone());
                }
            }

            if let Err(err) = output_controller.start(
                track,
                Arc::clone(&state),
                event_hub.clone(),
                queue_path,
                history_path,
                track_resolver.clone(),
            ) {
                apply_operation_failure(state, outcome, Some(track_id), err);
            }
        }
    }
}

fn playback_track_id_for_preflight(
    state: &RuntimeState,
    command: &PlaybackCommand,
) -> Option<String> {
    match command {
        PlaybackCommand::Play(track_id)
            if state.playback_state == PlaybackState::Paused
                && state.current_track.as_deref() == Some(track_id.as_str()) =>
        {
            None
        }
        PlaybackCommand::Play(track_id) | PlaybackCommand::PlayHistory(track_id) => {
            Some(track_id.clone())
        }
        PlaybackCommand::QueuePlay(track_ids) => track_ids.first().cloned(),
        _ => None,
    }
}

fn preflight_track_request(
    track_resolver: &TrackResolver,
    track_id: &str,
) -> Result<(), PlaybackOperationError> {
    let track = track_resolver.resolve(track_id)?;
    if !track.absolute_path.exists() {
        return Err(PlaybackOperationError::content(
            "track_file_missing",
            format!(
                "resolved media path is missing: {}",
                track.absolute_path.display()
            ),
            false,
        ));
    }

    Ok(())
}

fn apply_operation_failure(
    state: Arc<Mutex<RuntimeState>>,
    outcome: &mut CommandOutcome,
    track_id: Option<String>,
    err: PlaybackOperationError,
) {
    if let Some(track_id) = track_id.as_deref() {
        if let Ok(mut runtime) = state.lock() {
            let _ = runtime.apply_failure_state(track_id, &err);
            if outcome.persist_history {
                let _ = runtime.rollback_recent_history_for_track(track_id);
            }
        }
    }

    outcome.response_line = format_error_line(err.code, &err.reason);
    outcome.events = vec![PlaybackEvent::PlaybackFailed {
        reason: err.code.to_string(),
        class: err.class,
        recoverable: err.recoverable,
        keep_quiet: err.keep_quiet,
    }];
    outcome.persist_history = false;
}

fn apply_async_operation_failure(
    state: Arc<Mutex<RuntimeState>>,
    event_hub: &EventHub,
    track_id: Option<&str>,
    err: PlaybackOperationError,
) {
    if let Some(track_id) = track_id {
        if let Ok(mut runtime) = state.lock() {
            let _ = runtime.apply_failure_state(track_id, &err);
            let _ = runtime.rollback_recent_history_for_track(track_id);
        }
    }

    let _ = event_hub.broadcast(&PlaybackEvent::PlaybackFailed {
        reason: err.code.to_string(),
        class: err.class,
        recoverable: err.recoverable,
        keep_quiet: err.keep_quiet,
    });
}

fn handle_finished_track_success(
    output_controller: &OutputController,
    state: Arc<Mutex<RuntimeState>>,
    event_hub: &EventHub,
    queue_path: Arc<PathBuf>,
    history_path: Arc<PathBuf>,
    track_resolver: TrackResolver,
    finished_track_id: &str,
) {
    let (finish_action, queue_snapshot, history_log) = {
        let mut runtime = match state.lock() {
            Ok(runtime) => runtime,
            Err(_) => return,
        };
        let finish_action = runtime.finish_track_output(finished_track_id);
        let queue_snapshot = matches!(finish_action, TrackFinishAction::Start { .. })
            .then(|| runtime.snapshot.clone());
        let history_log = matches!(finish_action, TrackFinishAction::Start { .. })
            .then(|| runtime.history_log.clone());
        (finish_action, queue_snapshot, history_log)
    };

    if let Some(snapshot) = queue_snapshot.as_ref() {
        if let Err(err) = persist_queue_snapshot(queue_path.as_ref(), snapshot) {
            eprintln!("playbackd async queue persist error: {err}");
        }
    }
    if let Some(history_log) = history_log.as_ref() {
        if let Err(err) = persist_history_log(history_path.as_ref(), history_log) {
            eprintln!("playbackd async history persist error: {err}");
        }
    }

    match finish_action {
        TrackFinishAction::None => {}
        TrackFinishAction::Stop { reason } => {
            let _ = event_hub.broadcast(&PlaybackEvent::PlaybackStopped { reason });
        }
        TrackFinishAction::Start { track_id } => {
            let track = match track_resolver.resolve(&track_id) {
                Ok(track) => track,
                Err(err) => {
                    apply_async_operation_failure(state, event_hub, Some(track_id.as_str()), err);
                    return;
                }
            };

            let maybe_snapshot = {
                let mut runtime = match state.lock() {
                    Ok(runtime) => runtime,
                    Err(_) => return,
                };
                runtime
                    .enrich_current_track(&track)
                    .then(|| runtime.snapshot.clone())
            };
            if let Some(snapshot) = maybe_snapshot.as_ref() {
                if let Err(err) = persist_queue_snapshot(queue_path.as_ref(), snapshot) {
                    eprintln!("playbackd async queue persist error: {err}");
                }
            }

            if let Err(err) = output_controller.start(
                track,
                Arc::clone(&state),
                event_hub.clone(),
                Arc::clone(&queue_path),
                Arc::clone(&history_path),
                track_resolver,
            ) {
                apply_async_operation_failure(state, event_hub, Some(track_id.as_str()), err);
                return;
            }

            let _ = event_hub.broadcast(&PlaybackEvent::TrackChanged { track_id });
        }
    }
}

fn prepare_socket_path(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("socket path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("create socket dir {}: {err}", parent.display()))?;

    if path.exists() {
        fs::remove_file(path)
            .map_err(|err| format!("remove stale socket {}: {err}", path.display()))?;
    }

    Ok(())
}

fn load_runtime_state(queue_path: &Path, history_path: &Path) -> RuntimeState {
    let snapshot = match load_queue_snapshot(queue_path) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            eprintln!("playbackd queue restore fallback: {err}");
            QueueSnapshot::empty()
        }
    };
    let history_log = match load_history_log(history_path) {
        Ok(history_log) => history_log,
        Err(err) => {
            eprintln!("playbackd history restore fallback: {err}");
            HistoryLog::empty()
        }
    };

    if snapshot.tracks.is_empty() {
        let mut state = RuntimeState::new();
        state.history_log = history_log;
        state
    } else {
        RuntimeState::restore(snapshot, history_log)
    }
}

fn load_queue_snapshot(queue_path: &Path) -> Result<QueueSnapshot, String> {
    if !queue_path.exists() {
        return Ok(QueueSnapshot::empty());
    }

    let raw = fs::read_to_string(queue_path)
        .map_err(|err| format!("read queue state {}: {err}", queue_path.display()))?;
    let persisted: PersistedQueueState = serde_json::from_str(&raw)
        .map_err(|err| format!("parse queue state {}: {err}", queue_path.display()))?;
    if persisted.version != QUEUE_FILE_VERSION {
        return Err(format!(
            "unsupported queue state version {} in {}",
            persisted.version,
            queue_path.display()
        ));
    }

    Ok(persisted.into_snapshot())
}

fn load_history_log(history_path: &Path) -> Result<HistoryLog, String> {
    if !history_path.exists() {
        return Ok(HistoryLog::empty());
    }

    let raw = fs::read_to_string(history_path)
        .map_err(|err| format!("read history state {}: {err}", history_path.display()))?;
    let persisted: PersistedHistoryState = serde_json::from_str(&raw)
        .map_err(|err| format!("parse history state {}: {err}", history_path.display()))?;
    if persisted.version != HISTORY_FILE_VERSION {
        return Err(format!(
            "unsupported history state version {} in {}",
            persisted.version,
            history_path.display()
        ));
    }

    Ok(persisted.into_log())
}

fn persist_queue_snapshot(queue_path: &Path, snapshot: &QueueSnapshot) -> Result<(), String> {
    let persisted = PersistedQueueState::from_snapshot(snapshot);
    persist_state_file(queue_path, &persisted, "queue state")
}

fn persist_history_log(history_path: &Path, history_log: &HistoryLog) -> Result<(), String> {
    let persisted = PersistedHistoryState::from_log(history_log);
    persist_state_file(history_path, &persisted, "history state")
}

fn persist_state_file<T>(path: &Path, value: &T, label: &str) -> Result<(), String>
where
    T: Serialize,
{
    let parent = path
        .parent()
        .ok_or_else(|| format!("{label} path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("create {label} dir {}: {err}", parent.display()))?;

    let encoded = serde_json::to_string_pretty(value)
        .map_err(|err| format!("encode {label} {}: {err}", path.display()))?;
    let tmp_path = unique_state_tmp_path(path);
    let mut file = fs::File::create(&tmp_path)
        .map_err(|err| format!("create {label} temp {}: {err}", tmp_path.display()))?;
    file.write_all(format!("{encoded}\n").as_bytes())
        .map_err(|err| format!("write {label} temp {}: {err}", tmp_path.display()))?;
    file.sync_all()
        .map_err(|err| format!("fsync {label} temp {}: {err}", tmp_path.display()))?;
    fs::rename(&tmp_path, path).map_err(|err| {
        format!(
            "replace {label} {} from {}: {err}",
            path.display(),
            tmp_path.display()
        )
    })?;

    Ok(())
}

fn unique_state_tmp_path(path: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("state.json");

    path.with_file_name(format!("{file_name}.tmp.{nonce}"))
}

fn send_signal(pid: u32, signal: &str) -> Result<(), PlaybackOperationError> {
    let status = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(pid.to_string())
        .status()
        .map_err(|err| {
            PlaybackOperationError::output(
                "signal_failed",
                format!("send SIG{signal} to pid {pid}: {err}"),
            )
        })?;

    if status.success() {
        return Ok(());
    }

    Err(PlaybackOperationError::output(
        "signal_failed",
        format!(
            "send SIG{signal} to pid {pid}: {}",
            exit_status_label(&status)
        ),
    ))
}

fn wait_for_process_exit(pid: u32, timeout: Duration) -> bool {
    let started = std::time::Instant::now();
    loop {
        if !process_exists(pid) {
            return true;
        }
        if started.elapsed() >= timeout {
            return false;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn process_exists(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn configured_audio_device_from_env() -> Option<String> {
    std::env::var(AUDIO_DEVICE_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn alsa_cards_path_from_env() -> PathBuf {
    std::env::var(ALSA_CARDS_PATH_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ALSA_CARDS_PATH))
}

fn resolve_auto_usb_audio_output(
    cards_path: &Path,
) -> Result<UsbAudioOutputDevice, PlaybackOperationError> {
    let contents = fs::read_to_string(cards_path).map_err(|err| {
        PlaybackOperationError::output(
            "audio_output_unavailable",
            format!("read {}: {err}", cards_path.display()),
        )
    })?;
    select_unique_usb_audio_output(&contents)
}

fn select_unique_usb_audio_output(
    cards: &str,
) -> Result<UsbAudioOutputDevice, PlaybackOperationError> {
    let devices = usb_audio_output_devices_from_cards(cards);
    match devices.as_slice() {
        [] => Err(PlaybackOperationError::output(
            "audio_output_unavailable",
            "no USB Audio DAC found in /proc/asound/cards",
        )),
        [device] => Ok(device.clone()),
        _ => Err(PlaybackOperationError::output(
            "audio_output_ambiguous",
            format!(
                "multiple USB Audio DACs connected: {}",
                devices
                    .iter()
                    .map(|device| format!("{} ({})", device.name, device.alsa_device))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

fn usb_audio_output_devices_from_cards(cards: &str) -> Vec<UsbAudioOutputDevice> {
    let lines = cards.lines().collect::<Vec<_>>();
    let mut devices = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let Some((card_index, card_id, driver, name)) = parse_alsa_card_line(line) else {
            continue;
        };
        let detail = lines
            .get(index + 1)
            .filter(|next| parse_alsa_card_line(next).is_none())
            .map(|next| next.trim())
            .unwrap_or("");

        if !is_usb_audio_card(&driver, detail) {
            continue;
        }

        devices.push(UsbAudioOutputDevice {
            card_index,
            card_id: card_id.clone(),
            name: decoder_display_name(&name, detail),
            alsa_device: format!("plughw:CARD={card_id},DEV=0"),
        });
    }

    devices
}

fn parse_alsa_card_line(line: &str) -> Option<(usize, String, String, String)> {
    let trimmed = line.trim_start();
    let (index_text, after_index) = trimmed.split_once('[')?;
    let card_index = index_text.trim().parse::<usize>().ok()?;
    let (card_id, after_card) = after_index.split_once(']')?;
    let after_card = after_card.trim_start();
    let after_card = after_card.strip_prefix(':')?.trim_start();
    let (driver, name) = after_card.split_once(" - ")?;

    Some((
        card_index,
        card_id.trim().to_string(),
        driver.trim().to_string(),
        name.trim().to_string(),
    ))
}

fn is_usb_audio_card(driver: &str, detail: &str) -> bool {
    let driver = driver.trim().to_ascii_lowercase();
    let detail = detail.trim().to_ascii_lowercase();
    driver == "usb-audio" || detail.contains(" at usb-")
}

fn decoder_display_name(name: &str, detail: &str) -> String {
    let detail = detail.trim();
    if !detail.is_empty() {
        let mut display = detail;
        for marker in [" at usb-", " at "] {
            if let Some(index) = display.find(marker) {
                display = display[..index].trim();
                break;
            }
        }
        if !display.is_empty() {
            return display.to_string();
        }
    }

    name.trim().to_string()
}

fn load_dsd_output_policy() -> DsdOutputPolicy {
    if let Ok(value) = std::env::var("LUMELO_DSD_OUTPUT_POLICY") {
        if let Some(policy) = DsdOutputPolicy::parse(&value) {
            return policy;
        }
    }

    if let Ok(config_path) = std::env::var("LUMELO_CONFIG_PATH") {
        if let Ok(contents) = fs::read_to_string(config_path) {
            if let Some(policy) = parse_dsd_output_policy_from_config(&contents) {
                return policy;
            }
        }
    }

    if let Ok(contents) = fs::read_to_string(DEFAULT_CONFIG_PATH) {
        if let Some(policy) = parse_dsd_output_policy_from_config(&contents) {
            return policy;
        }
    }

    DEFAULT_DSD_POLICY
}

fn parse_dsd_output_policy_from_config(contents: &str) -> Option<DsdOutputPolicy> {
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() != "dsd_output_policy" {
            continue;
        }
        let value = value.trim().trim_matches('"');
        return DsdOutputPolicy::parse(value);
    }

    None
}

fn resolved_format(track: &ResolvedTrack) -> String {
    if !track.format.is_empty() {
        return track.format.clone();
    }

    detect_format(&track.absolute_path)
}

fn is_dsd_format(track: &ResolvedTrack) -> bool {
    matches!(resolved_format(track).as_str(), "dff" | "dsf" | "dsd")
}

impl DsdPlaybackPlan {
    fn aplay_format(&self) -> &'static str {
        match self.transport {
            DsdTransport::Native(format) => format.aplay_format(),
            DsdTransport::Dop(format) => format.aplay_format(),
            DsdTransport::Pcm => "S16_LE",
        }
    }
}

fn select_dsd_playback_plan(
    track: &ResolvedTrack,
    device: &str,
    policy: DsdOutputPolicy,
) -> Result<DsdPlaybackPlan, PlaybackOperationError> {
    let source = inspect_dsd_source_info(&track.absolute_path).map_err(|err| {
        PlaybackOperationError::content(
            "unsupported_format",
            format!(
                "prepare DSD transport for {} ({}): {err}",
                track.track_uid,
                empty_label(&resolved_format(track))
            ),
            true,
        )
    })?;
    let capabilities = load_alsa_stream_capabilities(device).map_err(|err| {
        PlaybackOperationError::output(
            "dsd_capability_probe_failed",
            format!("probe DSD capability on {}: {err}", raw_dsd_device(device)),
        )
    })?;

    let selected = match policy {
        DsdOutputPolicy::NativeDsd => select_native_dsd_plan(source, device, &capabilities),
        DsdOutputPolicy::Dop => select_dop_plan(source, device, &capabilities),
    };
    if let Some(plan) = selected {
        return Ok(plan);
    }

    if let Some(plan) = select_pcm_fallback_plan(source, device, &capabilities) {
        return Ok(plan);
    }

    let device_label = raw_dsd_device(device);
    let capability_summary = capabilities.summary();
    let policy_label = policy.as_str();
    Err(PlaybackOperationError::output(
        "dsd_pcm_fallback_unavailable",
        format!(
            "selected DSD policy {policy_label} and PCM fallback are unavailable on {device_label}, {capability_summary}"
        ),
    ))
}

fn inspect_dsd_source_info(path: &Path) -> Result<DsdSourceInfo, String> {
    let reader = DsdReader::from_container(path.to_path_buf())
        .map_err(|err| format!("open {}: {err}", path.display()))?;
    let dsd_multiplier = u32::try_from(reader.dsd_rate())
        .map_err(|_| format!("invalid DSD rate for {}", path.display()))?;

    Ok(DsdSourceInfo {
        channels: reader.channels_num(),
        dsd_rate_hz: DSD64_RATE_HZ
            .checked_mul(dsd_multiplier)
            .ok_or_else(|| format!("DSD rate overflow for {}", path.display()))?,
    })
}

fn select_native_dsd_plan(
    source: DsdSourceInfo,
    device: &str,
    capabilities: &AlsaStreamCapabilities,
) -> Option<DsdPlaybackPlan> {
    let device = raw_dsd_device(device);
    for format in [
        AlsaDsdFormat::U32Le,
        AlsaDsdFormat::U32Be,
        AlsaDsdFormat::U16Le,
        AlsaDsdFormat::U16Be,
        AlsaDsdFormat::U8,
    ] {
        let sample_rate = source.dsd_rate_hz / format.sample_rate_divisor();
        if capabilities.supports(format.aplay_format(), sample_rate) {
            return Some(DsdPlaybackPlan {
                device: device.clone(),
                transport: DsdTransport::Native(format),
                channels: source.channels,
                sample_rate,
            });
        }
    }

    None
}

fn select_dop_plan(
    source: DsdSourceInfo,
    device: &str,
    capabilities: &AlsaStreamCapabilities,
) -> Option<DsdPlaybackPlan> {
    let device = raw_dsd_device(device);
    let sample_rate = source.dsd_rate_hz / 16;
    for format in [
        DopPcmFormat::S32Le,
        DopPcmFormat::S24_3Le,
        DopPcmFormat::S24Le,
    ] {
        if capabilities.supports(format.aplay_format(), sample_rate) {
            return Some(DsdPlaybackPlan {
                device: device.clone(),
                transport: DsdTransport::Dop(format),
                channels: source.channels,
                sample_rate,
            });
        }
    }

    None
}

fn select_pcm_fallback_plan(
    source: DsdSourceInfo,
    device: &str,
    capabilities: &AlsaStreamCapabilities,
) -> Option<DsdPlaybackPlan> {
    if !capabilities.supports("S16_LE", DSD_PCM_FALLBACK_RATE_HZ) {
        return None;
    }

    Some(DsdPlaybackPlan {
        device: raw_dsd_device(device),
        transport: DsdTransport::Pcm,
        channels: source.channels,
        sample_rate: DSD_PCM_FALLBACK_RATE_HZ,
    })
}

fn raw_dsd_device(device: &str) -> String {
    device
        .strip_prefix("plughw:")
        .map(|value| format!("hw:{value}"))
        .unwrap_or_else(|| device.to_string())
}

fn load_alsa_stream_capabilities(device: &str) -> Result<AlsaStreamCapabilities, String> {
    let raw_device = raw_dsd_device(device);
    let card_token = parse_card_token(&raw_device)
        .ok_or_else(|| format!("unsupported device label: {raw_device}"))?;
    let device_index = parse_device_index(&raw_device).unwrap_or(0);
    let card_dir = resolve_proc_card_dir(&card_token)?;
    let stream_path = card_dir.join(format!("stream{device_index}"));
    let stream = fs::read_to_string(&stream_path)
        .map_err(|err| format!("read {}: {err}", stream_path.display()))?;

    Ok(parse_alsa_stream_capabilities(&stream))
}

fn parse_card_token(device: &str) -> Option<String> {
    if let Some(card) = device
        .split("CARD=")
        .nth(1)
        .and_then(|value| value.split(',').next())
    {
        return Some(card.to_string());
    }

    device
        .split(':')
        .nth(1)
        .and_then(|value| value.split(',').next())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
}

fn parse_device_index(device: &str) -> Option<usize> {
    if let Some(index) = device
        .split("DEV=")
        .nth(1)
        .and_then(|value| value.split(',').next())
        .and_then(|value| value.parse::<usize>().ok())
    {
        return Some(index);
    }

    device
        .split(':')
        .nth(1)
        .and_then(|value| value.split(',').nth(1))
        .and_then(|value| value.parse::<usize>().ok())
}

fn resolve_proc_card_dir(card_token: &str) -> Result<PathBuf, String> {
    if card_token.chars().all(|value| value.is_ascii_digit()) {
        return Ok(PathBuf::from(format!("/proc/asound/card{card_token}")));
    }

    let proc_root = Path::new("/proc/asound");
    for entry in
        fs::read_dir(proc_root).map_err(|err| format!("read {}: {err}", proc_root.display()))?
    {
        let entry = entry.map_err(|err| format!("read proc asound entry: {err}"))?;
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if !name.starts_with("card") {
            continue;
        }
        let id_path = entry.path().join("id");
        let Ok(id) = fs::read_to_string(&id_path) else {
            continue;
        };
        if id.trim() == card_token {
            return Ok(entry.path());
        }
    }

    Err(format!(
        "no /proc/asound/card*/id entry matches {card_token}"
    ))
}

fn parse_alsa_stream_capabilities(stream: &str) -> AlsaStreamCapabilities {
    let mut formats = Vec::new();
    let mut rates = Vec::new();

    for line in stream.lines() {
        let trimmed = line.trim();
        if let Some(format) = trimmed.strip_prefix("Format: ") {
            let format = format.trim().to_string();
            if !formats.iter().any(|value| value == &format) {
                formats.push(format);
            }
            continue;
        }

        if let Some(list) = trimmed.strip_prefix("Rates: ") {
            for part in list.split(',') {
                let Ok(rate) = part.trim().parse::<u32>() else {
                    continue;
                };
                if !rates.iter().any(|value| *value == rate) {
                    rates.push(rate);
                }
            }
        }
    }

    AlsaStreamCapabilities { formats, rates }
}

fn inspect_decoded_pcm(track: &ResolvedTrack) -> Result<DecodedPCMConfig, PlaybackOperationError> {
    let (mut format, track_id, mut decoder) =
        open_decoder(&track.absolute_path).map_err(|err| {
            PlaybackOperationError::content(
                "unsupported_format",
                format!(
                    "prepare decoder for {} ({}): {err}",
                    track.track_uid,
                    empty_label(&resolved_format(track))
                ),
                true,
            )
        })?;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                return Err(PlaybackOperationError::content(
                    "decode_probe_failed",
                    format!(
                        "decoded stream ended before yielding audio: {}",
                        track.track_uid
                    ),
                    true,
                ));
            }
            Err(err) => {
                return Err(PlaybackOperationError::content(
                    "decode_probe_failed",
                    format!("read decoded packet for {}: {err}", track.track_uid),
                    true,
                ));
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                return Ok(DecodedPCMConfig {
                    sample_rate: decoded.spec().rate,
                    channels: decoded.spec().channels.count(),
                });
            }
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(err) => {
                return Err(PlaybackOperationError::content(
                    "decode_probe_failed",
                    format!("decode first audio packet for {}: {err}", track.track_uid),
                    true,
                ));
            }
        }
    }
}

fn stream_decoded_audio(path: PathBuf, mut stdin: ChildStdin) -> Result<(), String> {
    let (mut format, track_id, mut decoder) = open_decoder(&path)?;
    let mut pcm_bytes = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(err) => return Err(format!("read decoded packet for {}: {err}", path.display())),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(err) => return Err(format!("decode packet for {}: {err}", path.display())),
        };

        let mut sample_buffer =
            SampleBuffer::<i16>::new(decoded.capacity() as u64, *decoded.spec());
        sample_buffer.copy_interleaved_ref(decoded);

        pcm_bytes.clear();
        pcm_bytes.reserve(sample_buffer.samples().len() * 2);
        for sample in sample_buffer.samples() {
            pcm_bytes.extend_from_slice(&sample.to_le_bytes());
        }
        stdin
            .write_all(&pcm_bytes)
            .map_err(|err| format!("write pcm to aplay for {}: {err}", path.display()))?;
    }

    drop(stdin);
    Ok(())
}

fn stream_dsd_audio(
    path: PathBuf,
    mut stdin: ChildStdin,
    transport: DsdTransport,
) -> Result<(), String> {
    let reader = DsdReader::from_container(path.clone())
        .map_err(|err| format!("open DSD container {}: {err}", path.display()))?;
    let channels = reader.channels_num();
    let mut iter = reader
        .interl_iter(false, None)
        .map_err(|err| format!("create DSD iterator for {}: {err}", path.display()))?;

    match transport {
        DsdTransport::Native(format) => {
            let mut native_bytes = Vec::new();
            for (read_size, buffers) in &mut iter {
                let bytes = buffers
                    .first()
                    .ok_or_else(|| format!("empty DSD frame buffer for {}", path.display()))?;
                native_bytes.clear();
                native_bytes.reserve(read_size * format.sample_rate_divisor() as usize / 8);
                for &byte in &bytes[..read_size] {
                    format.push_dsd_byte(byte, &mut native_bytes);
                }
                stdin.write_all(&native_bytes).map_err(|err| {
                    format!("write native DSD to aplay for {}: {err}", path.display())
                })?;
            }
        }
        DsdTransport::Dop(format) => {
            let mut pending = Vec::new();
            let mut dop_bytes = Vec::new();
            let mut marker_index = 0usize;

            for (read_size, buffers) in &mut iter {
                let bytes = buffers
                    .first()
                    .ok_or_else(|| format!("empty DoP frame buffer for {}", path.display()))?;
                pending.extend_from_slice(&bytes[..read_size]);

                while pending.len() >= channels * 2 {
                    let marker = if marker_index % 2 == 0 { 0x05 } else { 0xFA };
                    marker_index += 1;
                    dop_bytes.clear();
                    for channel in 0..channels {
                        let low = pending[channel];
                        let high = pending[channels + channel];
                        format.push_dop_sample(low, high, marker, &mut dop_bytes);
                    }
                    stdin.write_all(&dop_bytes).map_err(|err| {
                        format!("write DoP frame to aplay for {}: {err}", path.display())
                    })?;
                    pending.drain(..channels * 2);
                }
            }

            if !pending.is_empty() {
                while pending.len() < channels * 2 {
                    pending.push(DSD_SILENCE_BYTE);
                }
                let marker = if marker_index % 2 == 0 { 0x05 } else { 0xFA };
                dop_bytes.clear();
                for channel in 0..channels {
                    let low = pending[channel];
                    let high = pending[channels + channel];
                    format.push_dop_sample(low, high, marker, &mut dop_bytes);
                }
                stdin.write_all(&dop_bytes).map_err(|err| {
                    format!(
                        "write final DoP frame to aplay for {}: {err}",
                        path.display()
                    )
                })?;
            }
        }
        DsdTransport::Pcm => {
            let decimation_bytes_per_channel = usize::try_from(reader.dsd_rate())
                .unwrap_or(1)
                .saturating_mul(8);
            let mut pending = Vec::new();
            let mut pcm_bytes = Vec::new();

            for (read_size, buffers) in &mut iter {
                let bytes = buffers.first().ok_or_else(|| {
                    format!("empty PCM fallback frame buffer for {}", path.display())
                })?;
                pending.extend_from_slice(&bytes[..read_size]);

                while pending.len() >= channels * decimation_bytes_per_channel {
                    pcm_bytes.clear();
                    for channel in 0..channels {
                        let mut sum = 0i32;
                        for byte_index in 0..decimation_bytes_per_channel {
                            let byte = pending[channel + byte_index * channels];
                            for bit_index in 0..8 {
                                let bit = (byte >> (7 - bit_index)) & 1;
                                sum += if bit == 0 { -1 } else { 1 };
                            }
                        }
                        let normalized = sum as f32 / (decimation_bytes_per_channel as f32 * 8.0);
                        let sample = (normalized * i16::MAX as f32) as i16;
                        pcm_bytes.extend_from_slice(&sample.to_le_bytes());
                    }
                    stdin.write_all(&pcm_bytes).map_err(|err| {
                        format!(
                            "write PCM fallback frame to aplay for {}: {err}",
                            path.display()
                        )
                    })?;
                    pending.drain(..channels * decimation_bytes_per_channel);
                }
            }

            if !pending.is_empty() {
                while pending.len() < channels * decimation_bytes_per_channel {
                    pending.push(DSD_SILENCE_BYTE);
                }
                pcm_bytes.clear();
                for channel in 0..channels {
                    let mut sum = 0i32;
                    for byte_index in 0..decimation_bytes_per_channel {
                        let byte = pending[channel + byte_index * channels];
                        for bit_index in 0..8 {
                            let bit = (byte >> (7 - bit_index)) & 1;
                            sum += if bit == 0 { -1 } else { 1 };
                        }
                    }
                    let normalized = sum as f32 / (decimation_bytes_per_channel as f32 * 8.0);
                    let sample = (normalized * i16::MAX as f32) as i16;
                    pcm_bytes.extend_from_slice(&sample.to_le_bytes());
                }
                stdin.write_all(&pcm_bytes).map_err(|err| {
                    format!(
                        "write final PCM fallback frame to aplay for {}: {err}",
                        path.display()
                    )
                })?;
            }
        }
    }

    drop(stdin);
    Ok(())
}

fn open_decoder(path: &Path) -> Result<(Box<dyn FormatReader>, u32, Box<dyn Decoder>), String> {
    let file = fs::File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    let media_source = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
        hint.with_extension(extension);
    }

    let probed = get_probe()
        .format(
            &hint,
            media_source,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|err| format!("probe {}: {err}", path.display()))?;

    let format = probed.format;
    let (track_id, codec_params) = {
        let track = format
            .default_track()
            .ok_or_else(|| format!("no default audio track in {}", path.display()))?;
        (track.id, track.codec_params.clone())
    };
    let decoder = get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|err| format!("create decoder for {}: {err}", path.display()))?;

    Ok((format, track_id, decoder))
}

fn detect_format(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default()
}

fn empty_label(value: &str) -> &str {
    if value.is_empty() {
        "(unknown)"
    } else {
        value
    }
}

fn exit_status_label(status: &ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("exit_code={code}");
    }

    if let Some(signal) = status.signal() {
        return format!("signal={signal}");
    }

    "unknown_exit".to_string()
}

fn normalize_snapshot(mut snapshot: QueueSnapshot) -> QueueSnapshot {
    if snapshot.tracks.is_empty() {
        snapshot.play_order.clear();
        snapshot.current_order_index = None;
        return snapshot;
    }

    let track_ids = snapshot
        .tracks
        .iter()
        .map(|entry| entry.queue_entry_id.clone())
        .collect::<Vec<_>>();

    if snapshot.play_order.is_empty() {
        snapshot.play_order = track_ids.clone();
    } else {
        snapshot.play_order.retain(|entry_id| {
            track_ids
                .iter()
                .any(|track_entry_id| track_entry_id == entry_id)
        });

        for track_id in &track_ids {
            if !snapshot
                .play_order
                .iter()
                .any(|entry_id| entry_id == track_id)
            {
                snapshot.play_order.push(track_id.clone());
            }
        }
    }

    if snapshot
        .current_order_index
        .is_some_and(|index| index >= snapshot.play_order.len())
    {
        snapshot.current_order_index = None;
    }

    snapshot
}

fn resolve_queue_entry(snapshot: &QueueSnapshot, index: Option<usize>) -> Option<QueueEntry> {
    let queue_entry_id = snapshot
        .play_order
        .get(index?)
        .map(|value| value.as_str())?;
    snapshot
        .tracks
        .iter()
        .find(|entry| entry.queue_entry_id == queue_entry_id)
        .cloned()
}

fn resolve_track_uid(snapshot: &QueueSnapshot, index: Option<usize>) -> Option<String> {
    resolve_queue_entry(snapshot, index).map(|entry| entry.track_uid)
}

fn next_queue_entry_id(snapshot: &QueueSnapshot) -> u64 {
    snapshot
        .tracks
        .iter()
        .filter_map(|entry| entry.queue_entry_id.strip_prefix('q'))
        .filter_map(|value| value.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn order_mode_label(mode: OrderMode) -> &'static str {
    match mode {
        OrderMode::Sequential => "sequential",
        OrderMode::Shuffle => "shuffle",
    }
}

fn parse_order_mode_value(raw: &str) -> Option<OrderMode> {
    match raw.trim() {
        "sequential" => Some(OrderMode::Sequential),
        "shuffle" => Some(OrderMode::Shuffle),
        _ => None,
    }
}

fn repeat_mode_label(mode: RepeatMode) -> &'static str {
    match mode {
        RepeatMode::Off => "off",
        RepeatMode::One => "one",
        RepeatMode::All => "all",
    }
}

fn parse_repeat_mode_value(raw: &str) -> Option<RepeatMode> {
    match raw.trim() {
        "off" => Some(RepeatMode::Off),
        "one" => Some(RepeatMode::One),
        "all" => Some(RepeatMode::All),
        _ => None,
    }
}

fn shuffle_queue_entry_ids(play_order: &mut [String]) {
    if play_order.len() < 2 {
        return;
    }

    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    seed ^= (play_order.len() as u64) << 32;

    for index in (1..play_order.len()).rev() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let swap_index = (seed as usize) % (index + 1);
        play_order.swap(index, swap_index);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_operation_failure, load_history_log, load_runtime_state, normalize_snapshot,
        parse_alsa_stream_capabilities, parse_dsd_output_policy_from_config, persist_history_log,
        persist_queue_snapshot, playback_track_id_for_preflight, preflight_track_request,
        resolve_track_uid, select_dop_plan, select_native_dsd_plan, select_pcm_fallback_plan,
        select_unique_usb_audio_output, usb_audio_output_devices_from_cards, AlsaDsdFormat,
        AlsaStreamCapabilities, DopPcmFormat, DsdOutputPolicy, DsdSourceInfo,
        PlaybackOperationError, ResolvedTrack, RuntimeState, TrackResolver, HISTORY_LIMIT,
    };
    use ipc_proto::parse_queue_snapshot_line;
    use media_model::{HistoryLog, OrderMode, QueueEntry, QueueSnapshot, RepeatMode};
    use rusqlite::Connection;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn restore_keeps_pointer_but_forces_stopped_state() {
        let path = temp_queue_path("restore_queue");
        let history_path = temp_queue_path("restore_history");
        let snapshot = shuffled_snapshot();
        persist_queue_snapshot(&path, &snapshot).unwrap();
        persist_history_log(&history_path, &HistoryLog::empty()).unwrap();

        let restored = load_runtime_state(&path, &history_path);

        assert_eq!(restored.playback_state.as_str(), "stopped");
        assert_eq!(restored.current_track.as_deref(), Some("track-b"));
        assert_eq!(restored.snapshot.current_order_index, Some(0));

        cleanup_temp_path(&path);
        cleanup_temp_path(&history_path);
    }

    #[test]
    fn stop_preserves_current_order_index() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::Stop);

        assert_eq!(state.playback_state.as_str(), "stopped");
        assert_eq!(state.snapshot.current_order_index, Some(0));
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
        assert!(!outcome.persist_queue);
    }

    #[test]
    fn resolve_track_uses_play_order_not_track_vector_index() {
        let snapshot = shuffled_snapshot();

        assert_eq!(
            resolve_track_uid(&snapshot, Some(0)).as_deref(),
            Some("track-b")
        );
        assert_eq!(
            resolve_track_uid(&snapshot, Some(1)).as_deref(),
            Some("track-a")
        );
    }

    #[test]
    fn normalize_snapshot_appends_missing_track_ids() {
        let snapshot = QueueSnapshot {
            order_mode: OrderMode::Sequential,
            repeat_mode: RepeatMode::Off,
            current_order_index: Some(0),
            play_order: vec!["q2".to_string()],
            tracks: vec![queue_entry("q1", "track-a"), queue_entry("q2", "track-b")],
        };

        let normalized = normalize_snapshot(snapshot);

        assert_eq!(
            normalized.play_order,
            vec!["q2".to_string(), "q1".to_string()]
        );
    }

    #[test]
    fn play_appends_recent_history_entry() {
        let mut state = RuntimeState::new();

        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-b".to_string()));

        assert_eq!(state.history_log.entries.len(), 2);
        assert_eq!(state.history_log.entries[0].track_uid, "track-b");
        assert_eq!(state.history_log.entries[1].track_uid, "track-a");
    }

    #[test]
    fn failed_play_rolls_back_recent_history_entry() {
        let state = Arc::new(Mutex::new(RuntimeState::new()));
        let mut outcome = {
            let mut runtime = state.lock().unwrap();
            runtime.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()))
        };

        {
            let runtime = state.lock().unwrap();
            assert_eq!(runtime.history_log.entries.len(), 1);
            assert_eq!(runtime.history_log.entries[0].track_uid, "track-a");
        }

        apply_operation_failure(
            Arc::clone(&state),
            &mut outcome,
            Some("track-a".to_string()),
            PlaybackOperationError::content(
                "missing_media",
                "resolved media path is missing",
                true,
            ),
        );

        let runtime = state.lock().unwrap();
        assert!(runtime.history_log.entries.is_empty());
        assert_eq!(runtime.playback_state.as_str(), "quiet_error_hold");
        assert!(!outcome.persist_history);
        assert!(outcome.response_line.contains("ERR"));
    }

    #[test]
    fn enrich_current_track_updates_recent_history_metadata() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));

        let resolved = resolved_track(
            "track-a",
            "media-vol-1",
            "albums/track-a.flac",
            Some("Track A".to_string()),
            Some(123_000),
            "flac",
        );

        assert!(state.enrich_current_track(&resolved));
        assert_eq!(state.history_log.entries.len(), 1);
        assert_eq!(state.history_log.entries[0].track_uid, "track-a");
        assert_eq!(state.history_log.entries[0].volume_uuid, "media-vol-1");
        assert_eq!(
            state.history_log.entries[0].relative_path,
            "albums/track-a.flac"
        );
        assert_eq!(
            state.history_log.entries[0].title,
            Some("Track A".to_string())
        );
        assert_eq!(state.history_log.entries[0].duration_ms, Some(123_000));
    }

    #[test]
    fn failed_play_after_enrich_rolls_back_recent_history_entry() {
        let state = Arc::new(Mutex::new(RuntimeState::new()));
        let mut outcome = {
            let mut runtime = state.lock().unwrap();
            runtime.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()))
        };

        {
            let mut runtime = state.lock().unwrap();
            assert!(runtime.enrich_current_track(&resolved_track(
                "track-a",
                "media-vol-1",
                "albums/track-a.flac",
                Some("Track A".to_string()),
                Some(123_000),
                "flac",
            )));
            assert_eq!(runtime.history_log.entries[0].volume_uuid, "media-vol-1");
        }

        apply_operation_failure(
            Arc::clone(&state),
            &mut outcome,
            Some("track-a".to_string()),
            PlaybackOperationError::content(
                "track_file_missing",
                "resolved media path is missing",
                true,
            ),
        );

        let runtime = state.lock().unwrap();
        assert!(runtime.history_log.entries.is_empty());
        assert_eq!(runtime.playback_state.as_str(), "quiet_error_hold");
        assert!(!outcome.persist_history);
    }

    #[test]
    fn output_failure_stops_instead_of_entering_quiet_error_hold() {
        let state = Arc::new(Mutex::new(RuntimeState::new()));
        let mut outcome = {
            let mut runtime = state.lock().unwrap();
            runtime.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()))
        };

        apply_operation_failure(
            Arc::clone(&state),
            &mut outcome,
            Some("track-a".to_string()),
            PlaybackOperationError::output(
                "dsd_transport_unavailable",
                "Native DSD / DoP unavailable",
            ),
        );

        let runtime = state.lock().unwrap();
        assert!(runtime.history_log.entries.is_empty());
        assert_eq!(runtime.playback_state.as_str(), "stopped");
        assert_eq!(runtime.current_track.as_deref(), Some("track-a"));
        assert!(!outcome.persist_history);
    }

    #[test]
    fn preflight_track_request_rejects_offline_volume_tracks() {
        let db_path = temp_queue_path("preflight_offline_volume");
        let connection = prepare_test_library_db(&db_path);
        connection
            .execute(
                "INSERT INTO volumes (volume_uuid, mount_path, is_available, last_seen_at) VALUES (?1, ?2, ?3, ?4)",
                ("vol-offline", "/media/offline", 0, 1_i64),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO tracks (track_uid, volume_uuid, relative_path, filename, title, duration_ms, format, indexed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    "track-offline",
                    "vol-offline",
                    "albums/track.flac",
                    "track.flac",
                    "Track Offline",
                    123_i64,
                    "flac",
                    1_i64,
                ),
            )
            .unwrap();

        let err = preflight_track_request(&TrackResolver::new(db_path.clone()), "track-offline")
            .expect_err("offline volume should fail preflight");

        assert_eq!(err.code, "track_volume_unavailable");
        assert!(!err.keep_quiet);
        cleanup_temp_path(&db_path);
    }

    #[test]
    fn track_resolver_rejects_absolute_paths_by_default() {
        let db_path = temp_queue_path("absolute_path_disabled_db");
        let media_path = temp_queue_path("absolute_path_disabled_track.wav");
        fs::write(&media_path, b"not real audio").unwrap();

        let err = TrackResolver::new_with_absolute_path_playback(db_path.clone(), false)
            .resolve(media_path.to_str().unwrap())
            .expect_err("absolute path playback should be disabled by default");

        assert_eq!(err.code, "absolute_path_playback_disabled");
        assert!(!err.keep_quiet);
        cleanup_temp_path(&db_path);
        cleanup_temp_path(&media_path);
    }

    #[test]
    fn track_resolver_allows_absolute_paths_only_when_explicitly_enabled() {
        let db_path = temp_queue_path("absolute_path_enabled_db");
        let media_path = temp_queue_path("absolute_path_enabled_track.wav");
        fs::write(&media_path, b"not real audio").unwrap();

        let track = TrackResolver::new_with_absolute_path_playback(db_path.clone(), true)
            .resolve(media_path.to_str().unwrap())
            .expect("absolute path playback should be available behind explicit dev flag");

        assert_eq!(track.track_uid, media_path.to_string_lossy());
        assert_eq!(track.volume_uuid, "manual-path");
        assert_eq!(track.absolute_path, media_path);
        cleanup_temp_path(&db_path);
        cleanup_temp_path(&media_path);
    }

    #[test]
    fn preflight_track_request_rejects_missing_paths_before_queue_mutation() {
        let db_path = temp_queue_path("preflight_missing_path");
        let mount_root = std::env::temp_dir().join("lumelo-missing-track-mount");
        let connection = prepare_test_library_db(&db_path);
        connection
            .execute(
                "INSERT INTO volumes (volume_uuid, mount_path, is_available, last_seen_at) VALUES (?1, ?2, ?3, ?4)",
                (
                    "vol-online",
                    mount_root.to_string_lossy().to_string(),
                    1,
                    1_i64,
                ),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO tracks (track_uid, volume_uuid, relative_path, filename, title, duration_ms, format, indexed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    "track-missing",
                    "vol-online",
                    "albums/missing.flac",
                    "missing.flac",
                    "Track Missing",
                    123_i64,
                    "flac",
                    1_i64,
                ),
            )
            .unwrap();

        let err = preflight_track_request(&TrackResolver::new(db_path.clone()), "track-missing")
            .expect_err("missing file should fail preflight");

        assert_eq!(err.code, "track_file_missing");
        assert!(!err.keep_quiet);
        cleanup_temp_path(&db_path);
    }

    #[test]
    fn playback_track_id_for_preflight_skips_paused_resume() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.playback_state = ipc_proto::PlaybackState::Paused;
        state.current_track = Some("track-a".to_string());

        assert_eq!(
            playback_track_id_for_preflight(
                &state,
                &ipc_proto::PlaybackCommand::Play("track-a".to_string())
            ),
            None
        );
        assert_eq!(
            playback_track_id_for_preflight(
                &state,
                &ipc_proto::PlaybackCommand::Play("track-b".to_string())
            ),
            Some("track-b".to_string())
        );
    }

    #[test]
    fn queue_append_selects_first_track_without_starting_playback() {
        let mut state = RuntimeState::new();

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-a".to_string(),
        ));

        assert_eq!(state.playback_state.as_str(), "stopped");
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
        assert_eq!(state.snapshot.current_order_index, Some(0));
        assert!(outcome.events.is_empty());
        assert!(outcome.persist_queue);
        assert!(!outcome.persist_history);
    }

    #[test]
    fn queue_insert_next_places_track_after_current_pointer() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-c".to_string(),
        ));

        state.apply_command(ipc_proto::PlaybackCommand::QueueInsertNext(
            "track-b".to_string(),
        ));

        let play_order_tracks = state
            .snapshot
            .play_order
            .iter()
            .map(|queue_entry_id| {
                state
                    .snapshot
                    .tracks
                    .iter()
                    .find(|entry| &entry.queue_entry_id == queue_entry_id)
                    .map(|entry| entry.track_uid.clone())
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(play_order_tracks, vec!["track-a", "track-b", "track-c"]);
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
        assert_eq!(state.snapshot.current_order_index, Some(0));
    }

    #[test]
    fn queue_snapshot_command_exposes_order_and_current_marker() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-c".to_string(),
        ));
        state.apply_command(ipc_proto::PlaybackCommand::QueueInsertNext(
            "track-b".to_string(),
        ));

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueSnapshot);
        let snapshot = parse_queue_snapshot_line(&outcome.response_line).unwrap();

        assert_eq!(snapshot.current_order_index, Some(0));
        assert_eq!(snapshot.entries.len(), 3);
        assert_eq!(snapshot.entries[0].track_uid, "track-a");
        assert_eq!(snapshot.entries[1].track_uid, "track-b");
        assert_eq!(snapshot.entries[2].track_uid, "track-c");
        assert!(snapshot.entries[0].is_current);
        assert!(!snapshot.entries[1].is_current);
    }

    #[test]
    fn queue_remove_before_current_shifts_pointer_without_switching_track() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-b".to_string(),
        ));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-c".to_string(),
        ));
        state.apply_command(ipc_proto::PlaybackCommand::Next);

        let remove_id = queue_entry_id_by_track(&state, "track-a");
        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueRemove(remove_id));

        assert_eq!(state.snapshot.current_order_index, Some(0));
        assert_eq!(state.current_track.as_deref(), Some("track-b"));
        assert!(outcome.events.is_empty());
        assert!(outcome.persist_queue);
    }

    #[test]
    fn queue_remove_current_active_track_advances_and_records_history() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-b".to_string(),
        ));

        let remove_id = queue_entry_id_by_track(&state, "track-a");
        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueRemove(remove_id));

        assert_eq!(state.playback_state.as_str(), "quiet_active");
        assert_eq!(state.current_track.as_deref(), Some("track-b"));
        assert!(outcome.persist_history);
        assert_eq!(state.history_log.entries[0].track_uid, "track-b");
        assert_eq!(
            outcome.events,
            vec![ipc_proto::PlaybackEvent::TrackChanged {
                track_id: "track-b".to_string()
            }]
        );
    }

    #[test]
    fn queue_clear_from_active_state_stops_and_empties_queue() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-b".to_string(),
        ));

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueClear);

        assert_eq!(state.playback_state.as_str(), "idle");
        assert!(state.snapshot.play_order.is_empty());
        assert!(state.snapshot.tracks.is_empty());
        assert_eq!(
            outcome.events,
            vec![ipc_proto::PlaybackEvent::PlaybackStopped {
                reason: "queue_cleared".to_string()
            }]
        );
    }

    #[test]
    fn queue_replace_stops_active_playback_and_selects_first_replacement() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueueReplace(vec![
            "track-x".to_string(),
            "track-y".to_string(),
        ]));

        assert_eq!(state.playback_state.as_str(), "stopped");
        assert_eq!(state.current_track.as_deref(), Some("track-x"));
        assert_eq!(state.snapshot.play_order.len(), 2);
        assert_eq!(
            outcome.events,
            vec![ipc_proto::PlaybackEvent::PlaybackStopped {
                reason: "queue_replaced".to_string()
            }]
        );
    }

    #[test]
    fn queue_play_replaces_queue_and_starts_first_track() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::QueuePlay(vec![
            "track-x".to_string(),
            "track-y".to_string(),
        ]));

        assert_eq!(state.playback_state.as_str(), "quiet_active");
        assert_eq!(state.current_track.as_deref(), Some("track-x"));
        assert_eq!(state.snapshot.play_order.len(), 2);
        assert_eq!(state.snapshot.current_order_index, Some(0));
        assert_eq!(state.history_log.entries[0].track_uid, "track-x");
        assert_eq!(
            outcome.events,
            vec![
                ipc_proto::PlaybackEvent::PlayRequestAccepted {
                    track_id: "track-x".to_string()
                },
                ipc_proto::PlaybackEvent::PlaybackStarted {
                    track_id: "track-x".to_string()
                }
            ]
        );
    }

    #[test]
    fn set_order_mode_rebuilds_play_order_without_changing_current_track() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::QueuePlay(vec![
            "track-a".to_string(),
            "track-b".to_string(),
            "track-c".to_string(),
        ]));
        state.apply_command(ipc_proto::PlaybackCommand::Next);

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::SetOrderMode(
            "shuffle".to_string(),
        ));

        assert_eq!(state.snapshot.order_mode, OrderMode::Shuffle);
        assert_eq!(state.current_track.as_deref(), Some("track-b"));
        assert_eq!(state.snapshot.current_order_index, Some(0));
        assert_eq!(
            resolve_track_uid(&state.snapshot, Some(0)).as_deref(),
            Some("track-b")
        );
        assert!(outcome.persist_queue);
        assert!(outcome.events.is_empty());

        state.apply_command(ipc_proto::PlaybackCommand::SetOrderMode(
            "sequential".to_string(),
        ));

        assert_eq!(state.snapshot.order_mode, OrderMode::Sequential);
        assert_eq!(state.snapshot.current_order_index, Some(1));
        assert_eq!(state.current_track.as_deref(), Some("track-b"));
        assert_eq!(
            state
                .snapshot
                .play_order
                .iter()
                .filter_map(|entry_id| state
                    .snapshot
                    .tracks
                    .iter()
                    .find(|entry| &entry.queue_entry_id == entry_id)
                    .map(|entry| entry.track_uid.as_str()))
                .collect::<Vec<_>>(),
            vec!["track-a", "track-b", "track-c"]
        );
    }

    #[test]
    fn set_repeat_mode_updates_queue_state_only() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::QueuePlay(vec![
            "track-a".to_string(),
            "track-b".to_string(),
        ]));

        let outcome =
            state.apply_command(ipc_proto::PlaybackCommand::SetRepeatMode("all".to_string()));

        assert_eq!(state.snapshot.repeat_mode, RepeatMode::All);
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
        assert_eq!(state.playback_state, ipc_proto::PlaybackState::QuietActive);
        assert!(outcome.persist_queue);
        assert!(!outcome.persist_history);
        assert!(outcome.events.is_empty());
    }

    #[test]
    fn play_history_replaces_current_entry_and_preserves_following_queue() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::QueuePlay(vec![
            "track-a".to_string(),
            "track-b".to_string(),
            "track-c".to_string(),
        ]));
        state.apply_command(ipc_proto::PlaybackCommand::Next);
        state.snapshot.order_mode = OrderMode::Shuffle;
        state.snapshot.repeat_mode = RepeatMode::All;
        let play_order_before = state.snapshot.play_order.clone();
        let current_queue_entry_id = play_order_before[1].clone();
        let next_queue_entry_id = play_order_before[2].clone();
        let queue_len_before = state.snapshot.tracks.len();

        let outcome = state.apply_command(ipc_proto::PlaybackCommand::PlayHistory(
            "history-track".to_string(),
        ));

        assert_eq!(state.playback_state.as_str(), "quiet_active");
        assert_eq!(state.current_track.as_deref(), Some("history-track"));
        assert_eq!(state.snapshot.current_order_index, Some(1));
        assert_eq!(state.snapshot.play_order, play_order_before);
        assert_eq!(state.snapshot.tracks.len(), queue_len_before);
        assert_eq!(state.snapshot.order_mode, OrderMode::Shuffle);
        assert_eq!(state.snapshot.repeat_mode, RepeatMode::All);
        assert_eq!(
            state
                .snapshot
                .tracks
                .iter()
                .find(|entry| entry.queue_entry_id == current_queue_entry_id)
                .map(|entry| entry.track_uid.as_str()),
            Some("history-track")
        );
        assert_eq!(
            state
                .snapshot
                .tracks
                .iter()
                .find(|entry| entry.queue_entry_id == next_queue_entry_id)
                .map(|entry| entry.track_uid.as_str()),
            Some("track-c")
        );
        assert_eq!(state.history_log.entries[0].track_uid, "history-track");
        assert!(outcome.persist_queue);
        assert!(outcome.persist_history);
        assert_eq!(
            outcome.events,
            vec![
                ipc_proto::PlaybackEvent::PlayRequestAccepted {
                    track_id: "history-track".to_string()
                },
                ipc_proto::PlaybackEvent::PlaybackStarted {
                    track_id: "history-track".to_string()
                }
            ]
        );
    }

    #[test]
    fn finished_track_advances_to_next_entry_and_records_history() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-b".to_string(),
        ));

        let action = state.finish_track_output("track-a");

        assert_eq!(
            action,
            super::TrackFinishAction::Start {
                track_id: "track-b".to_string()
            }
        );
        assert_eq!(state.playback_state.as_str(), "quiet_active");
        assert_eq!(state.current_track.as_deref(), Some("track-b"));
        assert_eq!(state.snapshot.current_order_index, Some(1));
        assert_eq!(state.history_log.entries[0].track_uid, "track-b");
    }

    #[test]
    fn finished_track_stops_at_queue_end_when_repeat_is_off() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));

        let action = state.finish_track_output("track-a");

        assert_eq!(
            action,
            super::TrackFinishAction::Stop {
                reason: "track_finished".to_string()
            }
        );
        assert_eq!(state.playback_state.as_str(), "stopped");
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
    }

    #[test]
    fn finished_track_wraps_to_queue_start_when_repeat_is_all() {
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::QueueAppend(
            "track-b".to_string(),
        ));
        state.apply_command(ipc_proto::PlaybackCommand::Next);
        state.snapshot.repeat_mode = RepeatMode::All;

        let action = state.finish_track_output("track-b");

        assert_eq!(
            action,
            super::TrackFinishAction::Start {
                track_id: "track-a".to_string()
            }
        );
        assert_eq!(state.playback_state.as_str(), "quiet_active");
        assert_eq!(state.current_track.as_deref(), Some("track-a"));
        assert_eq!(state.snapshot.current_order_index, Some(0));
    }

    #[test]
    fn parse_dsd_output_policy_accepts_native_dsd_dop_and_legacy_aliases() {
        assert_eq!(
            parse_dsd_output_policy_from_config("dsd_output_policy = \"native_dsd\""),
            Some(DsdOutputPolicy::NativeDsd)
        );
        assert_eq!(
            parse_dsd_output_policy_from_config("dsd_output_policy = \"native_dop\""),
            Some(DsdOutputPolicy::NativeDsd)
        );
        assert_eq!(
            parse_dsd_output_policy_from_config("dsd_output_policy = \"dop\""),
            Some(DsdOutputPolicy::Dop)
        );
        assert_eq!(
            parse_dsd_output_policy_from_config("dsd_output_policy = \"strict_native\""),
            Some(DsdOutputPolicy::NativeDsd)
        );
    }

    #[test]
    fn parse_alsa_stream_capabilities_collects_unique_formats_and_rates() {
        let parsed = parse_alsa_stream_capabilities(
            r#"
Playback:
  Interface 2
    Altset 1
    Format: S16_LE
    Rates: 48000, 44100
  Interface 2
    Altset 2
    Format: DSD_U32_LE
    Rates: 88200
  Interface 2
    Altset 3
    Format: S32_LE
    Rates: 176400, 352800
    "#,
        );

        assert_eq!(
            parsed.formats,
            vec![
                "S16_LE".to_string(),
                "DSD_U32_LE".to_string(),
                "S32_LE".to_string()
            ]
        );
        assert_eq!(parsed.rates, vec![48000, 44100, 88200, 176400, 352800]);
    }

    #[test]
    fn usb_audio_output_devices_from_cards_selects_usb_dac() {
        let devices = usb_audio_output_devices_from_cards(
            r#" 0 [rockchiphdmi   ]: rockchip_hdmi - rockchip,hdmi
                      rockchip,hdmi
 1 [realtekrt5651co]: realtek_rt5651- - realtek,rt5651-codec
                      realtek,rt5651-codec
 2 [iBassoDC04Pro  ]: USB-Audio - iBasso-DC04-Pro
                      ibasso iBasso-DC04-Pro at usb-xhci-hcd.0.auto-1, high speed
"#,
        );

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].card_index, 2);
        assert_eq!(devices[0].card_id, "iBassoDC04Pro");
        assert_eq!(devices[0].name, "ibasso iBasso-DC04-Pro");
        assert_eq!(devices[0].alsa_device, "plughw:CARD=iBassoDC04Pro,DEV=0");
    }

    #[test]
    fn select_unique_usb_audio_output_reports_no_dac() {
        let err = select_unique_usb_audio_output(
            r#" 0 [rockchiphdmi   ]: rockchip_hdmi - rockchip,hdmi
                      rockchip,hdmi
"#,
        )
        .expect_err("no USB DAC should be rejected");

        assert_eq!(err.code, "audio_output_unavailable");
    }

    #[test]
    fn select_unique_usb_audio_output_reports_multiple_dacs() {
        let err = select_unique_usb_audio_output(
            r#" 1 [Audio          ]: USB-Audio - USB Audio
                      DAC One at usb-xhci-hcd.1.auto-1, high speed
 2 [Device         ]: USB-Audio - USB Audio Device
                      DAC Two at usb-xhci-hcd.1.auto-2, full speed
"#,
        )
        .expect_err("multiple USB DACs should be explicit");

        assert_eq!(err.code, "audio_output_ambiguous");
        assert!(err.reason.contains("DAC One"));
        assert!(err.reason.contains("DAC Two"));
    }

    #[test]
    fn select_native_dsd_plan_prefers_native_transport() {
        let source = DsdSourceInfo {
            channels: 2,
            dsd_rate_hz: 2_822_400,
        };
        let capabilities = AlsaStreamCapabilities {
            formats: vec!["DSD_U32_LE".to_string(), "S32_LE".to_string()],
            rates: vec![88_200, 176_400],
        };

        let plan = select_native_dsd_plan(source, "plughw:CARD=Audio,DEV=0", &capabilities)
            .expect("native plan should exist");

        assert_eq!(plan.device, "hw:CARD=Audio,DEV=0");
        assert_eq!(
            plan.transport,
            super::DsdTransport::Native(AlsaDsdFormat::U32Le)
        );
        assert_eq!(plan.sample_rate, 88_200);
    }

    #[test]
    fn select_dop_plan_accepts_pcm_carrier_when_native_missing() {
        let source = DsdSourceInfo {
            channels: 2,
            dsd_rate_hz: 5_644_800,
        };
        let capabilities = AlsaStreamCapabilities {
            formats: vec!["S24_3LE".to_string()],
            rates: vec![352_800],
        };

        let plan = select_dop_plan(source, "plughw:CARD=Audio,DEV=0", &capabilities)
            .expect("dop plan should exist");

        assert_eq!(plan.device, "hw:CARD=Audio,DEV=0");
        assert_eq!(
            plan.transport,
            super::DsdTransport::Dop(DopPcmFormat::S24_3Le)
        );
        assert_eq!(plan.sample_rate, 352_800);
    }

    #[test]
    fn select_pcm_fallback_plan_uses_s16le_44k1() {
        let source = DsdSourceInfo {
            channels: 2,
            dsd_rate_hz: 2_822_400,
        };
        let capabilities = AlsaStreamCapabilities {
            formats: vec!["S16_LE".to_string(), "S24_3LE".to_string()],
            rates: vec![44_100, 48_000],
        };

        let plan = select_pcm_fallback_plan(source, "plughw:CARD=Audio,DEV=0", &capabilities)
            .expect("pcm fallback plan should exist");

        assert_eq!(plan.device, "hw:CARD=Audio,DEV=0");
        assert_eq!(plan.transport, super::DsdTransport::Pcm);
        assert_eq!(plan.sample_rate, 44_100);
    }

    #[test]
    fn native_and_dop_packers_emit_expected_bytes() {
        let mut native = Vec::new();
        AlsaDsdFormat::U32Le.push_dsd_byte(0xA5, &mut native);
        assert_eq!(native, vec![0xA5, 0x00, 0x00, 0x00]);

        let mut dop = Vec::new();
        DopPcmFormat::S24_3Le.push_dop_sample(0x12, 0x34, 0x05, &mut dop);
        assert_eq!(dop, vec![0x12, 0x34, 0x05]);
        dop.clear();
        DopPcmFormat::S32Le.push_dop_sample(0x12, 0x34, 0xFA, &mut dop);
        assert_eq!(dop, vec![0x12, 0x34, 0xFA, 0x00]);
    }

    #[test]
    fn history_log_is_capped_to_latest_limit() {
        let mut state = RuntimeState::new();

        for index in 0..(HISTORY_LIMIT + 5) {
            state.apply_command(ipc_proto::PlaybackCommand::Play(format!("track-{index}")));
        }

        assert_eq!(state.history_log.entries.len(), HISTORY_LIMIT);
        assert_eq!(state.history_log.entries[0].track_uid, "track-104");
        assert_eq!(
            state.history_log.entries[HISTORY_LIMIT - 1].track_uid,
            "track-5"
        );
    }

    #[test]
    fn persisted_history_log_round_trips() {
        let path = temp_queue_path("history_round_trip");
        let mut state = RuntimeState::new();
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-a".to_string()));
        state.apply_command(ipc_proto::PlaybackCommand::Play("track-b".to_string()));

        persist_history_log(&path, &state.history_log).unwrap();
        let restored = load_history_log(&path).unwrap();

        assert_eq!(restored.entries.len(), 2);
        assert_eq!(restored.entries[0].track_uid, "track-b");
        assert_eq!(restored.entries[1].track_uid, "track-a");

        cleanup_temp_path(&path);
    }

    fn shuffled_snapshot() -> QueueSnapshot {
        QueueSnapshot {
            order_mode: OrderMode::Shuffle,
            repeat_mode: RepeatMode::All,
            current_order_index: Some(0),
            play_order: vec!["q2".to_string(), "q1".to_string()],
            tracks: vec![queue_entry("q1", "track-a"), queue_entry("q2", "track-b")],
        }
    }

    fn queue_entry(queue_entry_id: &str, track_uid: &str) -> QueueEntry {
        QueueEntry {
            queue_entry_id: queue_entry_id.to_string(),
            track_uid: track_uid.to_string(),
            volume_uuid: "vol-1".to_string(),
            relative_path: track_uid.to_string(),
            title: Some(track_uid.to_string()),
            duration_ms: None,
        }
    }

    fn resolved_track(
        track_uid: &str,
        volume_uuid: &str,
        relative_path: &str,
        title: Option<String>,
        duration_ms: Option<u64>,
        format: &str,
    ) -> ResolvedTrack {
        ResolvedTrack {
            track_uid: track_uid.to_string(),
            volume_uuid: volume_uuid.to_string(),
            relative_path: relative_path.to_string(),
            title,
            duration_ms,
            format: format.to_string(),
            absolute_path: PathBuf::from(format!("/tmp/{track_uid}.{format}")),
        }
    }

    fn prepare_test_library_db(path: &PathBuf) -> Connection {
        let _ = fs::remove_file(path);
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "
                CREATE TABLE volumes (
                    volume_uuid TEXT PRIMARY KEY,
                    label TEXT,
                    mount_path TEXT NOT NULL,
                    fs_type TEXT,
                    is_available INTEGER NOT NULL DEFAULT 1,
                    last_seen_at INTEGER NOT NULL
                );
                CREATE TABLE tracks (
                    track_uid TEXT PRIMARY KEY,
                    album_id INTEGER,
                    volume_uuid TEXT NOT NULL,
                    relative_path TEXT NOT NULL,
                    filename TEXT NOT NULL,
                    title TEXT,
                    artist TEXT,
                    album_artist TEXT,
                    track_no INTEGER,
                    disc_no INTEGER,
                    duration_ms INTEGER,
                    sample_rate INTEGER,
                    bit_depth INTEGER,
                    format TEXT,
                    cover_ref_id INTEGER,
                    musicbrainz_track_id TEXT,
                    file_mtime INTEGER,
                    indexed_at INTEGER NOT NULL,
                    UNIQUE(volume_uuid, relative_path)
                );
                ",
            )
            .unwrap();
        connection
    }

    fn queue_entry_id_by_track(state: &RuntimeState, track_uid: &str) -> String {
        state
            .snapshot
            .tracks
            .iter()
            .find(|entry| entry.track_uid == track_uid)
            .map(|entry| entry.queue_entry_id.clone())
            .unwrap()
    }

    fn temp_queue_path(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("lumelo-{label}-{suffix}.json"))
    }

    fn cleanup_temp_path(path: &PathBuf) {
        let _ = fs::remove_file(path);
    }
}
