use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderMode {
    Sequential,
    Shuffle,
}

impl Default for OrderMode {
    fn default() -> Self {
        Self::Sequential
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    Off,
    One,
    All,
}

impl Default for RepeatMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct QueueEntry {
    pub queue_entry_id: String,
    pub track_uid: String,
    pub volume_uuid: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub order_mode: OrderMode,
    pub repeat_mode: RepeatMode,
    pub current_order_index: Option<usize>,
    pub play_order: Vec<String>,
    pub tracks: Vec<QueueEntry>,
}

impl QueueSnapshot {
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub played_at: u64,
    pub track_uid: String,
    pub volume_uuid: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct HistoryLog {
    pub entries: Vec<HistoryEntry>,
}

impl HistoryLog {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn push_recent(&mut self, entry: HistoryEntry, limit: usize) {
        self.entries.insert(0, entry);
        self.entries.truncate(limit);
    }
}
