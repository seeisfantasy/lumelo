use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const LUMELO_RUNTIME_DIR: &str = "/run/lumelo";
pub const LUMELO_STATE_DIR: &str = "/var/lib/lumelo";
pub const LUMELO_CACHE_DIR: &str = "/var/cache/lumelo";
pub const PLAYBACK_CMD_SOCKET: &str = "/run/lumelo/playback_cmd.sock";
pub const PLAYBACK_EVT_SOCKET: &str = "/run/lumelo/playback_evt.sock";
pub const QUIET_MODE_FLAG_PATH: &str = "/run/lumelo/quiet_mode";
pub const PLAYBACK_QUEUE_STATE_PATH: &str = "/var/lib/lumelo/queue.json";
pub const PLAYBACK_HISTORY_STATE_PATH: &str = "/var/lib/lumelo/history.json";

pub const PRODUCT_STATE_DIR: &str = LUMELO_STATE_DIR;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlaybackCommand {
    Ping,
    Status,
    QueueSnapshot,
    HistorySnapshot,
    Play(String),
    QueuePlay(Vec<String>),
    QueueAppend(String),
    QueueInsertNext(String),
    QueueRemove(String),
    QueueClear,
    QueueReplace(Vec<String>),
    Pause,
    Stop,
    Next,
    Prev,
    PlayHistory(String),
    SetOrderMode(String),
    SetRepeatMode(String),
}

impl PlaybackCommand {
    pub fn encode(&self) -> String {
        match self {
            Self::Ping => "PING".to_string(),
            Self::Status => "STATUS".to_string(),
            Self::QueueSnapshot => "QUEUE_SNAPSHOT".to_string(),
            Self::HistorySnapshot => "HISTORY_SNAPSHOT".to_string(),
            Self::Play(track_id) => format!("PLAY {track_id}"),
            Self::QueuePlay(track_ids) => format!(
                "QUEUE_PLAY {}",
                serde_json::to_string(track_ids)
                    .expect("queue play track ids must serialize as JSON")
            ),
            Self::QueueAppend(track_id) => format!("QUEUE_APPEND {track_id}"),
            Self::QueueInsertNext(track_id) => format!("QUEUE_INSERT_NEXT {track_id}"),
            Self::QueueRemove(queue_entry_id) => format!("QUEUE_REMOVE {queue_entry_id}"),
            Self::QueueClear => "QUEUE_CLEAR".to_string(),
            Self::QueueReplace(track_ids) => format!(
                "QUEUE_REPLACE {}",
                serde_json::to_string(track_ids)
                    .expect("queue replace track ids must serialize as JSON")
            ),
            Self::Pause => "PAUSE".to_string(),
            Self::Stop => "STOP".to_string(),
            Self::Next => "NEXT".to_string(),
            Self::Prev => "PREV".to_string(),
            Self::PlayHistory(track_id) => format!("PLAY_HISTORY {track_id}"),
            Self::SetOrderMode(mode) => format!("SET_ORDER_MODE {mode}"),
            Self::SetRepeatMode(mode) => format!("SET_REPEAT_MODE {mode}"),
        }
    }
}

pub fn runtime_dir_path() -> PathBuf {
    env_path(
        &["LUMELO_RUNTIME_DIR", "PRODUCT_RUNTIME_DIR"],
        PathBuf::from(LUMELO_RUNTIME_DIR),
    )
}

pub fn state_dir_path() -> PathBuf {
    env_path(
        &["LUMELO_STATE_DIR", "PRODUCT_STATE_DIR"],
        PathBuf::from(LUMELO_STATE_DIR),
    )
}

pub fn cache_dir_path() -> PathBuf {
    env_path(
        &["LUMELO_CACHE_DIR", "PRODUCT_CACHE_DIR"],
        PathBuf::from(LUMELO_CACHE_DIR),
    )
}

pub fn command_socket_path_from_env() -> PathBuf {
    std::env::var("PLAYBACK_CMD_SOCKET_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| runtime_dir_path().join(socket_filename(PLAYBACK_CMD_SOCKET)))
}

pub fn event_socket_path_from_env() -> PathBuf {
    std::env::var("PLAYBACK_EVT_SOCKET_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| runtime_dir_path().join(socket_filename(PLAYBACK_EVT_SOCKET)))
}

pub fn quiet_mode_flag_path_from_env() -> PathBuf {
    std::env::var("QUIET_MODE_FLAG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| runtime_dir_path().join(socket_filename(QUIET_MODE_FLAG_PATH)))
}

pub fn queue_state_path_from_env() -> PathBuf {
    std::env::var("PLAYBACK_QUEUE_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| state_dir_path().join(socket_filename(PLAYBACK_QUEUE_STATE_PATH)))
}

pub fn history_state_path_from_env() -> PathBuf {
    std::env::var("PLAYBACK_HISTORY_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| state_dir_path().join(socket_filename(PLAYBACK_HISTORY_STATE_PATH)))
}

fn env_path(keys: &[&str], fallback: PathBuf) -> PathBuf {
    for key in keys {
        if let Ok(value) = std::env::var(key) {
            if !value.is_empty() {
                return PathBuf::from(value);
            }
        }
    }

    fallback
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PlaybackState {
    Idle,
    PreQuiet,
    QuietActive,
    Paused,
    QuietErrorHold,
    Stopped,
}

impl PlaybackState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::PreQuiet => "pre_quiet",
            Self::QuietActive => "quiet_active",
            Self::Paused => "paused",
            Self::QuietErrorHold => "quiet_error_hold",
            Self::Stopped => "stopped",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "idle" => Some(Self::Idle),
            "pre_quiet" => Some(Self::PreQuiet),
            "quiet_active" => Some(Self::QuietActive),
            "paused" => Some(Self::Paused),
            "quiet_error_hold" => Some(Self::QuietErrorHold),
            "stopped" => Some(Self::Stopped),
            _ => None,
        }
    }
}

impl fmt::Display for PlaybackState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PlaybackStatusSnapshot {
    pub state: PlaybackState,
    pub order_mode: String,
    pub repeat_mode: String,
    pub current_track: Option<String>,
    pub last_command: Option<String>,
    pub queue_entries: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueueSnapshotView {
    pub order_mode: String,
    pub repeat_mode: String,
    pub current_order_index: Option<usize>,
    pub entries: Vec<QueueSnapshotEntryView>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueueSnapshotEntryView {
    pub order_index: usize,
    pub queue_entry_id: String,
    pub track_uid: String,
    pub volume_uuid: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub duration_ms: Option<u64>,
    pub is_current: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HistorySnapshotView {
    pub entries: Vec<HistorySnapshotEntryView>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HistorySnapshotEntryView {
    pub played_at: u64,
    pub track_uid: String,
    pub volume_uuid: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub duration_ms: Option<u64>,
}

impl Default for PlaybackStatusSnapshot {
    fn default() -> Self {
        Self {
            state: PlaybackState::Idle,
            order_mode: "sequential".to_string(),
            repeat_mode: "off".to_string(),
            current_track: None,
            last_command: None,
            queue_entries: 0,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlaybackEvent {
    PlayRequestAccepted {
        track_id: String,
    },
    PlaybackStarted {
        track_id: String,
    },
    PlaybackPaused,
    PlaybackResumed,
    PlaybackStopped {
        reason: String,
    },
    TrackChanged {
        track_id: String,
    },
    PlaybackFailed {
        reason: String,
        class: PlaybackFailureClass,
        recoverable: bool,
        keep_quiet: bool,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PlaybackFailureClass {
    Output,
    Content,
}

impl PlaybackFailureClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Output => "output",
            Self::Content => "content",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "output" => Some(Self::Output),
            "content" => Some(Self::Content),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProtocolError {
    pub code: &'static str,
    pub message: String,
}

impl ProtocolError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProtocolError {}

pub fn parse_command_line(line: &str) -> Result<PlaybackCommand, ProtocolError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(ProtocolError::new("empty_command", "command line is empty"));
    }

    let (verb, payload) = match trimmed.split_once(' ') {
        Some((verb, payload)) => (verb, Some(payload.trim())),
        None => (trimmed, None),
    };

    match verb {
        "PING" => Ok(PlaybackCommand::Ping),
        "STATUS" => Ok(PlaybackCommand::Status),
        "QUEUE_SNAPSHOT" => Ok(PlaybackCommand::QueueSnapshot),
        "HISTORY_SNAPSHOT" => Ok(PlaybackCommand::HistorySnapshot),
        "QUEUE_CLEAR" => Ok(PlaybackCommand::QueueClear),
        "PAUSE" => Ok(PlaybackCommand::Pause),
        "STOP" => Ok(PlaybackCommand::Stop),
        "NEXT" => Ok(PlaybackCommand::Next),
        "PREV" => Ok(PlaybackCommand::Prev),
        "PLAY" => parse_track_command(payload).map(PlaybackCommand::Play),
        "QUEUE_PLAY" => parse_track_list_command(payload).map(PlaybackCommand::QueuePlay),
        "QUEUE_APPEND" => parse_track_command(payload).map(PlaybackCommand::QueueAppend),
        "QUEUE_INSERT_NEXT" => parse_track_command(payload).map(PlaybackCommand::QueueInsertNext),
        "QUEUE_REMOVE" => parse_queue_entry_command(payload).map(PlaybackCommand::QueueRemove),
        "QUEUE_REPLACE" => parse_track_list_command(payload).map(PlaybackCommand::QueueReplace),
        "PLAY_HISTORY" => parse_track_command(payload).map(PlaybackCommand::PlayHistory),
        "SET_ORDER_MODE" => parse_order_mode_command(payload).map(PlaybackCommand::SetOrderMode),
        "SET_REPEAT_MODE" => parse_repeat_mode_command(payload).map(PlaybackCommand::SetRepeatMode),
        _ => Err(ProtocolError::new(
            "unknown_command",
            format!("unsupported command verb: {verb}"),
        )),
    }
}

fn parse_order_mode_command(payload: Option<&str>) -> Result<String, ProtocolError> {
    let mode = payload.unwrap_or_default().trim();
    if mode.is_empty() {
        return Err(ProtocolError::new(
            "missing_order_mode",
            "order mode is required",
        ));
    }
    match mode {
        "sequential" | "shuffle" => Ok(mode.to_string()),
        _ => Err(ProtocolError::new(
            "invalid_order_mode",
            format!("unsupported order mode: {mode}"),
        )),
    }
}

fn parse_repeat_mode_command(payload: Option<&str>) -> Result<String, ProtocolError> {
    let mode = payload.unwrap_or_default().trim();
    if mode.is_empty() {
        return Err(ProtocolError::new(
            "missing_repeat_mode",
            "repeat mode is required",
        ));
    }
    match mode {
        "off" | "one" | "all" => Ok(mode.to_string()),
        _ => Err(ProtocolError::new(
            "invalid_repeat_mode",
            format!("unsupported repeat mode: {mode}"),
        )),
    }
}

pub fn format_ack_line(action: &str, state: PlaybackState, current_track: Option<&str>) -> String {
    format!(
        "OK\tkind=ack\taction={}\tstate={}\tcurrent_track={}",
        sanitize_field_value(action),
        state,
        optional_field(current_track)
    )
}

pub fn format_status_line(status: &PlaybackStatusSnapshot) -> String {
    format!(
        "OK\tkind=status\tstate={}\torder_mode={}\trepeat_mode={}\tcurrent_track={}\tlast_command={}\tqueue_entries={}",
        status.state,
        sanitize_field_value(&status.order_mode),
        sanitize_field_value(&status.repeat_mode),
        optional_field(status.current_track.as_deref()),
        optional_field(status.last_command.as_deref()),
        status.queue_entries
    )
}

pub fn format_queue_snapshot_line(snapshot: &QueueSnapshotView) -> String {
    let payload = serde_json::to_string(snapshot).expect("queue snapshot must serialize");
    format!(
        "OK\tkind=queue_snapshot\tpayload={}",
        sanitize_field_value(&payload)
    )
}

pub fn format_history_snapshot_line(snapshot: &HistorySnapshotView) -> String {
    let payload = serde_json::to_string(snapshot).expect("history snapshot must serialize");
    format!(
        "OK\tkind=history_snapshot\tpayload={}",
        sanitize_field_value(&payload)
    )
}

pub fn format_error_line(code: &str, message: &str) -> String {
    format!(
        "ERR\tcode={}\tmessage={}",
        sanitize_field_value(code),
        sanitize_field_value(message)
    )
}

pub fn format_event_line(event: &PlaybackEvent) -> String {
    match event {
        PlaybackEvent::PlayRequestAccepted { track_id } => {
            format!(
                "EVENT\tname=PLAY_REQUEST_ACCEPTED\ttrack_id={}",
                sanitize_field_value(track_id)
            )
        }
        PlaybackEvent::PlaybackStarted { track_id } => {
            format!(
                "EVENT\tname=PLAYBACK_STARTED\ttrack_id={}",
                sanitize_field_value(track_id)
            )
        }
        PlaybackEvent::PlaybackPaused => "EVENT\tname=PLAYBACK_PAUSED".to_string(),
        PlaybackEvent::PlaybackResumed => "EVENT\tname=PLAYBACK_RESUMED".to_string(),
        PlaybackEvent::PlaybackStopped { reason } => {
            format!(
                "EVENT\tname=PLAYBACK_STOPPED\treason={}",
                sanitize_field_value(reason)
            )
        }
        PlaybackEvent::TrackChanged { track_id } => {
            format!(
                "EVENT\tname=TRACK_CHANGED\ttrack_id={}",
                sanitize_field_value(track_id)
            )
        }
        PlaybackEvent::PlaybackFailed {
            reason,
            class,
            recoverable,
            keep_quiet,
        } => format!(
            "EVENT\tname=PLAYBACK_FAILED\treason={}\tclass={}\trecoverable={}\tkeep_quiet={}",
            sanitize_field_value(reason),
            class.as_str(),
            recoverable,
            keep_quiet
        ),
    }
}

pub fn parse_status_line(line: &str) -> Result<PlaybackStatusSnapshot, ProtocolError> {
    let (kind, fields) = parse_response_line(line)?;
    if kind != "OK" {
        return Err(ProtocolError::new(
            "unexpected_response",
            "response is not OK",
        ));
    }

    let response_kind = fields
        .iter()
        .find_map(|(key, value)| (*key == "kind").then_some(*value))
        .ok_or_else(|| ProtocolError::new("missing_field", "missing kind field"))?;
    if response_kind != "status" {
        return Err(ProtocolError::new(
            "unexpected_response",
            format!("expected status response, got {response_kind}"),
        ));
    }

    let state = PlaybackState::parse(require_field(&fields, "state")?)
        .ok_or_else(|| ProtocolError::new("invalid_state", "state field is invalid"))?;

    let queue_entries = require_field(&fields, "queue_entries")?
        .parse::<usize>()
        .map_err(|_| {
            ProtocolError::new("invalid_queue_entries", "queue_entries is not a number")
        })?;

    Ok(PlaybackStatusSnapshot {
        state,
        order_mode: require_field(&fields, "order_mode")?.to_string(),
        repeat_mode: require_field(&fields, "repeat_mode")?.to_string(),
        current_track: parse_optional_field(require_field(&fields, "current_track")?),
        last_command: parse_optional_field(require_field(&fields, "last_command")?),
        queue_entries,
    })
}

pub fn parse_queue_snapshot_line(line: &str) -> Result<QueueSnapshotView, ProtocolError> {
    let (kind, fields) = parse_response_line(line)?;
    if kind != "OK" {
        return Err(ProtocolError::new(
            "unexpected_response",
            "response is not OK",
        ));
    }

    let response_kind = fields
        .iter()
        .find_map(|(key, value)| (*key == "kind").then_some(*value))
        .ok_or_else(|| ProtocolError::new("missing_field", "missing kind field"))?;
    if response_kind != "queue_snapshot" {
        return Err(ProtocolError::new(
            "unexpected_response",
            format!("expected queue_snapshot response, got {response_kind}"),
        ));
    }

    let payload = require_field(&fields, "payload")?;
    serde_json::from_str(payload).map_err(|err| {
        ProtocolError::new(
            "invalid_queue_snapshot",
            format!("queue snapshot payload is invalid JSON: {err}"),
        )
    })
}

pub fn parse_history_snapshot_line(line: &str) -> Result<HistorySnapshotView, ProtocolError> {
    let (kind, fields) = parse_response_line(line)?;
    if kind != "OK" {
        return Err(ProtocolError::new(
            "unexpected_response",
            "response is not OK",
        ));
    }

    let response_kind = fields
        .iter()
        .find_map(|(key, value)| (*key == "kind").then_some(*value))
        .ok_or_else(|| ProtocolError::new("missing_field", "missing kind field"))?;
    if response_kind != "history_snapshot" {
        return Err(ProtocolError::new(
            "unexpected_response",
            format!("expected history_snapshot response, got {response_kind}"),
        ));
    }

    let payload = require_field(&fields, "payload")?;
    serde_json::from_str(payload).map_err(|err| {
        ProtocolError::new(
            "invalid_history_snapshot",
            format!("history snapshot payload is invalid JSON: {err}"),
        )
    })
}

pub fn parse_event_line(line: &str) -> Result<PlaybackEvent, ProtocolError> {
    let fields = parse_prefixed_fields(line, "EVENT")?;
    let name = require_field(&fields, "name")?;

    match name {
        "PLAY_REQUEST_ACCEPTED" => Ok(PlaybackEvent::PlayRequestAccepted {
            track_id: require_field(&fields, "track_id")?.to_string(),
        }),
        "PLAYBACK_STARTED" => Ok(PlaybackEvent::PlaybackStarted {
            track_id: require_field(&fields, "track_id")?.to_string(),
        }),
        "PLAYBACK_PAUSED" => Ok(PlaybackEvent::PlaybackPaused),
        "PLAYBACK_RESUMED" => Ok(PlaybackEvent::PlaybackResumed),
        "PLAYBACK_STOPPED" => Ok(PlaybackEvent::PlaybackStopped {
            reason: require_field(&fields, "reason")?.to_string(),
        }),
        "TRACK_CHANGED" => Ok(PlaybackEvent::TrackChanged {
            track_id: require_field(&fields, "track_id")?.to_string(),
        }),
        "PLAYBACK_FAILED" => Ok(PlaybackEvent::PlaybackFailed {
            reason: require_field(&fields, "reason")?.to_string(),
            class: PlaybackFailureClass::parse(require_field(&fields, "class")?)
                .ok_or_else(|| ProtocolError::new("invalid_class", "invalid failure class"))?,
            recoverable: parse_bool_field(require_field(&fields, "recoverable")?)?,
            keep_quiet: parse_bool_field(require_field(&fields, "keep_quiet")?)?,
        }),
        _ => Err(ProtocolError::new(
            "unknown_event",
            format!("unsupported event name: {name}"),
        )),
    }
}

fn parse_response_line(line: &str) -> Result<(&str, Vec<(&str, &str)>), ProtocolError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(ProtocolError::new(
            "empty_response",
            "response line is empty",
        ));
    }

    let mut parts = trimmed.split('\t');
    let kind = parts
        .next()
        .ok_or_else(|| ProtocolError::new("empty_response", "response line is empty"))?;

    if kind != "OK" && kind != "ERR" {
        return Err(ProtocolError::new(
            "invalid_response",
            format!("unsupported response prefix: {kind}"),
        ));
    }

    let mut fields = Vec::new();
    for token in parts {
        let (key, value) = token.split_once('=').ok_or_else(|| {
            ProtocolError::new(
                "invalid_field",
                format!("response field is missing '=': {token}"),
            )
        })?;
        fields.push((key, value));
    }

    Ok((kind, fields))
}

fn parse_prefixed_fields<'a>(
    line: &'a str,
    expected_prefix: &str,
) -> Result<Vec<(&'a str, &'a str)>, ProtocolError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(ProtocolError::new("empty_line", "line is empty"));
    }

    let mut parts = trimmed.split('\t');
    let prefix = parts
        .next()
        .ok_or_else(|| ProtocolError::new("empty_line", "line is empty"))?;
    if prefix != expected_prefix {
        return Err(ProtocolError::new(
            "invalid_prefix",
            format!("expected prefix {expected_prefix}, got {prefix}"),
        ));
    }

    let mut fields = Vec::new();
    for token in parts {
        let (key, value) = token.split_once('=').ok_or_else(|| {
            ProtocolError::new("invalid_field", format!("field is missing '=': {token}"))
        })?;
        fields.push((key, value));
    }

    Ok(fields)
}

fn require_field<'a>(
    fields: &'a [(&'a str, &'a str)],
    name: &str,
) -> Result<&'a str, ProtocolError> {
    fields
        .iter()
        .find_map(|(key, value)| (*key == name).then_some(*value))
        .ok_or_else(|| ProtocolError::new("missing_field", format!("missing field: {name}")))
}

fn parse_track_command(payload: Option<&str>) -> Result<String, ProtocolError> {
    let value = payload.unwrap_or_default().trim();
    if value.is_empty() {
        return Err(ProtocolError::new(
            "missing_track_id",
            "track id is required",
        ));
    }

    Ok(value.to_string())
}

fn parse_queue_entry_command(payload: Option<&str>) -> Result<String, ProtocolError> {
    let value = payload.unwrap_or_default().trim();
    if value.is_empty() {
        return Err(ProtocolError::new(
            "missing_queue_entry_id",
            "queue entry id is required",
        ));
    }

    Ok(value.to_string())
}

fn parse_track_list_command(payload: Option<&str>) -> Result<Vec<String>, ProtocolError> {
    let value = payload.unwrap_or_default().trim();
    if value.is_empty() {
        return Err(ProtocolError::new(
            "missing_track_list",
            "track list is required",
        ));
    }

    let parsed: Vec<String> = serde_json::from_str(value).map_err(|err| {
        ProtocolError::new(
            "invalid_track_list",
            format!("track list must be a JSON array of strings: {err}"),
        )
    })?;
    if parsed.iter().any(|track_id| track_id.trim().is_empty()) {
        return Err(ProtocolError::new(
            "invalid_track_id",
            "track list contains an empty track id",
        ));
    }

    Ok(parsed)
}

fn parse_bool_field(raw: &str) -> Result<bool, ProtocolError> {
    match raw {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(ProtocolError::new(
            "invalid_bool",
            format!("invalid boolean value: {raw}"),
        )),
    }
}

fn optional_field(value: Option<&str>) -> String {
    match value {
        Some(raw) if !raw.is_empty() => sanitize_field_value(raw),
        _ => "-".to_string(),
    }
}

fn parse_optional_field(value: &str) -> Option<String> {
    (value != "-").then(|| value.to_string())
}

fn sanitize_field_value(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '\t' | '\r' | '\n' => ' ',
            _ => ch,
        })
        .collect()
}

fn socket_filename(default_path: &'static str) -> &'static str {
    Path::new(default_path)
        .file_name()
        .and_then(|value| value.to_str())
        .expect("default path must have a file name")
}

#[cfg(test)]
mod tests {
    use super::{
        format_event_line, format_history_snapshot_line, format_queue_snapshot_line,
        format_status_line, parse_command_line, parse_event_line, parse_history_snapshot_line,
        parse_queue_snapshot_line, parse_status_line, HistorySnapshotEntryView,
        HistorySnapshotView, PlaybackCommand, PlaybackEvent, PlaybackFailureClass, PlaybackState,
        PlaybackStatusSnapshot, QueueSnapshotEntryView, QueueSnapshotView,
    };

    #[test]
    fn parses_track_commands_with_spaces() {
        let parsed = parse_command_line("PLAY Album Side A / Track 01").unwrap();
        assert_eq!(
            parsed,
            PlaybackCommand::Play("Album Side A / Track 01".to_string())
        );
    }

    #[test]
    fn parses_queue_replace_json_payload() {
        let parsed = parse_command_line(
            r#"QUEUE_REPLACE ["Album Side A / Track 01","Album Side B / Track 02"]"#,
        )
        .unwrap();
        assert_eq!(
            parsed,
            PlaybackCommand::QueueReplace(vec![
                "Album Side A / Track 01".to_string(),
                "Album Side B / Track 02".to_string(),
            ])
        );
    }

    #[test]
    fn parses_queue_play_json_payload() {
        let parsed = parse_command_line(
            r#"QUEUE_PLAY ["Album Side A / Track 01","Album Side B / Track 02"]"#,
        )
        .unwrap();
        assert_eq!(
            parsed,
            PlaybackCommand::QueuePlay(vec![
                "Album Side A / Track 01".to_string(),
                "Album Side B / Track 02".to_string(),
            ])
        );
    }

    #[test]
    fn parses_history_snapshot_command() {
        let parsed = parse_command_line("HISTORY_SNAPSHOT").unwrap();

        assert_eq!(parsed, PlaybackCommand::HistorySnapshot);
    }

    #[test]
    fn parses_playback_mode_commands() {
        assert_eq!(
            parse_command_line("SET_ORDER_MODE shuffle").unwrap(),
            PlaybackCommand::SetOrderMode("shuffle".to_string())
        );
        assert_eq!(
            parse_command_line("SET_REPEAT_MODE all").unwrap(),
            PlaybackCommand::SetRepeatMode("all".to_string())
        );
    }

    #[test]
    fn rejects_unknown_playback_modes() {
        let order_err = parse_command_line("SET_ORDER_MODE random").unwrap_err();
        assert_eq!(order_err.code, "invalid_order_mode");

        let repeat_err = parse_command_line("SET_REPEAT_MODE forever").unwrap_err();
        assert_eq!(repeat_err.code, "invalid_repeat_mode");
    }

    #[test]
    fn queue_replace_encode_round_trips_json() {
        let original = PlaybackCommand::QueueReplace(vec![
            "Album Side A / Track 01".to_string(),
            "Album Side B / Track 02".to_string(),
        ]);

        let encoded = original.encode();
        let decoded = parse_command_line(&encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn queue_play_encode_round_trips_json() {
        let original = PlaybackCommand::QueuePlay(vec![
            "Album Side A / Track 01".to_string(),
            "Album Side B / Track 02".to_string(),
        ]);

        let encoded = original.encode();
        let decoded = parse_command_line(&encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn playback_mode_encode_round_trips() {
        for original in [
            PlaybackCommand::SetOrderMode("sequential".to_string()),
            PlaybackCommand::SetOrderMode("shuffle".to_string()),
            PlaybackCommand::SetRepeatMode("off".to_string()),
            PlaybackCommand::SetRepeatMode("one".to_string()),
            PlaybackCommand::SetRepeatMode("all".to_string()),
        ] {
            let encoded = original.encode();
            let decoded = parse_command_line(&encoded).unwrap();

            assert_eq!(decoded, original);
        }
    }

    #[test]
    fn status_round_trip_preserves_fields() {
        let original = PlaybackStatusSnapshot {
            state: PlaybackState::QuietActive,
            order_mode: "sequential".to_string(),
            repeat_mode: "off".to_string(),
            current_track: Some("Album Side A / Track 01".to_string()),
            last_command: Some("play_history:Album Side A / Track 01".to_string()),
            queue_entries: 3,
        };

        let encoded = format_status_line(&original);
        let decoded = parse_status_line(&encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn queue_snapshot_round_trip_preserves_fields() {
        let original = QueueSnapshotView {
            order_mode: "sequential".to_string(),
            repeat_mode: "off".to_string(),
            current_order_index: Some(1),
            entries: vec![
                QueueSnapshotEntryView {
                    order_index: 0,
                    queue_entry_id: "q1".to_string(),
                    track_uid: "track-a".to_string(),
                    volume_uuid: "vol-1".to_string(),
                    relative_path: "track-a".to_string(),
                    title: Some("track-a".to_string()),
                    duration_ms: None,
                    is_current: false,
                },
                QueueSnapshotEntryView {
                    order_index: 1,
                    queue_entry_id: "q2".to_string(),
                    track_uid: "track-b".to_string(),
                    volume_uuid: "vol-1".to_string(),
                    relative_path: "track-b".to_string(),
                    title: Some("track-b".to_string()),
                    duration_ms: None,
                    is_current: true,
                },
            ],
        };

        let encoded = format_queue_snapshot_line(&original);
        let decoded = parse_queue_snapshot_line(&encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn history_snapshot_round_trip_preserves_fields() {
        let original = HistorySnapshotView {
            entries: vec![HistorySnapshotEntryView {
                played_at: 123,
                track_uid: "track-a".to_string(),
                volume_uuid: "vol-1".to_string(),
                relative_path: "Album/track-a.flac".to_string(),
                title: Some("Track A".to_string()),
                duration_ms: Some(201_000),
            }],
        };

        let encoded = format_history_snapshot_line(&original);
        let decoded = parse_history_snapshot_line(&encoded).unwrap();

        assert_eq!(decoded, original);
    }

    #[test]
    fn event_round_trip_preserves_failure_payload() {
        let original = PlaybackEvent::PlaybackFailed {
            reason: "alsa_open_failed".to_string(),
            class: PlaybackFailureClass::Output,
            recoverable: false,
            keep_quiet: false,
        };

        let encoded = format_event_line(&original);
        let decoded = parse_event_line(&encoded).unwrap();

        assert_eq!(decoded, original);
    }
}
