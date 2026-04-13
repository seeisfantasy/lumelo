# AI Review Part 23

这是给外部 AI 做静态审查的代码分卷。每一卷都只包含仓库快照中的一部分文本文件内容，按当前工作树生成。

## `services/rust/crates/playbackd/src/main.rs` (1/3)

- bytes: 84527
- segment: 1/3

~~~rust
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
    format_event_line, format_queue_snapshot_line, format_status_line, history_state_path_from_env,
    parse_command_line, queue_state_path_from_env, state_dir_path, PlaybackCommand, PlaybackEvent,
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
const DEFAULT_AUDIO_DEVICE: &str = "default";

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
    let track_resolver = TrackResolver::new(state_dir_path().join(LIBRARY_DB_FILENAME));
    let output_controller = OutputController::new(
        std::env::var("LUMELO_AUDIO_DEVICE").unwrap_or_else(|_| DEFAULT_AUDIO_DEVICE.to_string()),
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
    println!("  audio device:   {}", output_controller.device_label());

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

#[derive(Debug, Clone)]
struct TrackResolver {
    db_path: PathBuf,
}

#[derive(Debug, Clone)]
struct OutputController {
    inner: Arc<Mutex<OutputState>>,
    device: Arc<String>,
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
        Self { db_path }
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
                    let mount_path: String = row.get(6)?;
                    let relative_path: String = row.get(2)?;
                    Ok(ResolvedTrack {
                        track_uid: row.get(0)?,
                        volume_uuid: row.get(1)?,
                        relative_path: relative_path.clone(),
                        title: row.get::<_, Option<String>>(3)?,
                        duration_ms,
                        format: row.get(5)?,
                        absolute_path: PathBuf::from(mount_path).join(&relative_path),
                    })
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

        row.ok_or_else(|| {
            PlaybackOperationError::content(
                "track_not_found",
                format!("track is not present in library.db: {track_id}"),
                true,
            )
        })
    }

    fn resolve_path_track(
        &self,
        track_id: &str,
    ) -> Result<Option<ResolvedTrack>, PlaybackOperationError> {
        let candidate = Path::new(track_id);
        if !candidate.is_absolute() || !candidate.exists() {
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

impl OutputController {
    fn new(device: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(OutputState::default())),
            device: Arc::new(device),
        }
    }

    fn device_label(&self) -> &str {
        self.device.as_str()
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

        let child = self.spawn_output_child(&track)?;

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

    fn spawn_output_child(&self, track: &ResolvedTrack) -> Result<Child, PlaybackOperationError> {
        if resolved_format(track) == "wav" {
            return self.spawn_aplay_file(track);
        }

        self.spawn_decoded_aplay(track)
    }

    fn spawn_aplay_file(&self, track: &ResolvedTrack) -> Result<Child, PlaybackOperationError> {
        Command::new("aplay")
            .arg("-D")
            .arg(self.device.as_str())
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
                        self.device.as_str()
                    ),
                )
            })
    }

    fn spawn_decoded_aplay(&self, track: &ResolvedTrack) -> Result<Child, PlaybackOperationError> {
        let pcm = inspect_decoded_pcm(track)?;
        let mut child = Command::new("aplay")
            .arg("-D")
            .arg(self.device.as_str())
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
                        self.device.as_str()
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
            PlaybackCommand::PlayHistory(track_id) => {
                self.enqueue_and_activate(track_id.clone(), "play_history");
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

    fn enqueue_and_activate(&mut self, track_id: String, action: &str) {
        let queue_entry_id = self.push_track_entry(track_id.clone());
        self.snapshot.play_order.push(queue_entry_id);
        self.snapshot.current_order_index = Some(self.snapshot.play_order.len().saturating_sub(1));
        self.snapshot.order_mode = OrderMode::Sequential;
        self.snapshot.repeat_mode = RepeatMode::Off;

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
~~~

