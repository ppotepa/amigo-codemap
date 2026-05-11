use serde::{Deserialize, Serialize};

use crate::model::CodeMap;

pub const DEFAULT_DAEMON_ADDR: &str = "127.0.0.1:47821";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonMapOptions {
    pub root: String,
    pub out: String,
    pub level: u8,
    pub pretty: bool,
    pub ai: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DaemonRequest {
    Status,
    GetMap { options: DaemonMapOptions },
    Refresh { options: DaemonMapOptions },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub root: String,
    pub out: String,
    pub level: u8,
    pub ai: bool,
    pub dirty: bool,
    pub files: usize,
    pub packages: usize,
    pub symbols: usize,
    pub dependencies: usize,
    pub areas: usize,
    pub generated_at_unix_ms: u64,
    pub generation: u64,
    pub dirty_paths: usize,
    pub last_refresh_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DaemonResponse {
    Ok,
    Status {
        status: DaemonStatus,
    },
    Map {
        map: CodeMap,
        stale_was_refreshed: bool,
    },
    Refreshed {
        wrote_output: bool,
        generated_at_unix_ms: u64,
    },
    Error {
        message: String,
    },
}
