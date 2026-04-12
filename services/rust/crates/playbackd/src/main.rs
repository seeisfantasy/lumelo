use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use ipc_proto::{
    command_socket_path_from_env, event_socket_path_from_env, format_ack_line, format_error_line,
    format_event_line, format_queue_snapshot_line, format_status_line, history_state_path_from_env,
    parse_command_line, queue_state_path_from_env, PlaybackCommand, PlaybackEvent, PlaybackState,
    PlaybackStatusSnapshot, QueueSnapshotEntryView, QueueSnapshotView,
};
use media_model::{HistoryEntry, HistoryLog, OrderMode, QueueEntry, QueueSnapshot, RepeatMode};
use serde::{Deserialize, Serialize};

const QUEUE_FILE_VERSION: u32 = 1;
const HISTORY_FILE_VERSION: u32 = 1;
const HISTORY_LIMIT: usize = 100;

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

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&state);
                let event_hub = event_hub.clone();
                let queue_path = Arc::clone(&queue_path);
                let history_path = Arc::clone(&history_path);
                thread::spawn(move || {
                    if let Err(err) =
                        handle_client(stream, state, event_hub, queue_path, history_path)
                    {
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

    let (outcome, snapshot_to_persist, history_to_persist) = {
        let mut state = state
            .lock()
            .map_err(|_| "playback state lock poisoned".to_string())?;
        let outcome = state.apply_command(command);
        let snapshot = outcome.persist_queue.then(|| state.snapshot.clone());
        let history = outcome.persist_history.then(|| state.history_log.clone());
        (outcome, snapshot, history)
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

fn repeat_mode_label(mode: RepeatMode) -> &'static str {
    match mode {
        RepeatMode::Off => "off",
        RepeatMode::One => "one",
        RepeatMode::All => "all",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        load_history_log, load_runtime_state, normalize_snapshot, persist_history_log,
        persist_queue_snapshot, resolve_track_uid, RuntimeState, HISTORY_LIMIT,
    };
    use ipc_proto::parse_queue_snapshot_line;
    use media_model::{HistoryLog, OrderMode, QueueEntry, QueueSnapshot, RepeatMode};
    use std::fs;
    use std::path::PathBuf;
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
