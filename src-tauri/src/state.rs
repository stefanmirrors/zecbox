use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub const LOG_BUFFER_CAPACITY: usize = 5000;

pub struct AppState {
    pub node: Arc<NodeState>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            node: Arc::new(NodeState::new(data_dir)),
        }
    }
}

pub struct NodeState {
    pub status: Mutex<NodeStatus>,
    pub process: Mutex<Option<tokio::process::Child>>,
    pub health_task: Mutex<Option<JoinHandle<()>>>,
    pub log_reader_tasks: Mutex<Vec<JoinHandle<()>>>,
    pub log_buffer: Mutex<VecDeque<String>>,
    pub data_dir: Mutex<PathBuf>,
    pub backoff: Mutex<BackoffState>,
}

impl NodeState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            status: Mutex::new(NodeStatus::Stopped),
            process: Mutex::new(None),
            health_task: Mutex::new(None),
            log_reader_tasks: Mutex::new(Vec::new()),
            log_buffer: Mutex::new(VecDeque::with_capacity(LOG_BUFFER_CAPACITY)),
            data_dir: Mutex::new(data_dir),
            backoff: Mutex::new(BackoffState::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum NodeStatus {
    Stopped,
    Starting,
    #[serde(rename_all = "camelCase")]
    Running {
        block_height: u64,
        peer_count: u32,
    },
    Stopping,
    Error {
        message: String,
    },
}

impl NodeStatus {
    pub fn status_str(&self) -> &'static str {
        match self {
            NodeStatus::Stopped => "stopped",
            NodeStatus::Starting => "starting",
            NodeStatus::Running { .. } => "running",
            NodeStatus::Stopping => "stopping",
            NodeStatus::Error { .. } => "error",
        }
    }

    pub fn is_stopped_or_error(&self) -> bool {
        matches!(self, NodeStatus::Stopped | NodeStatus::Error { .. })
    }
}

#[derive(Debug)]
pub struct BackoffState {
    pub consecutive_failures: u32,
    pub current_delay_secs: u64,
    pub healthy_since: Option<Instant>,
}

impl Default for BackoffState {
    fn default() -> Self {
        Self {
            consecutive_failures: 0,
            current_delay_secs: 1,
            healthy_since: None,
        }
    }
}

impl BackoffState {
    pub fn next_delay(&mut self) -> u64 {
        let delay = self.current_delay_secs;
        self.consecutive_failures += 1;
        self.current_delay_secs = (self.current_delay_secs * 2).min(60);
        delay
    }

    pub fn reset(&mut self) {
        self.consecutive_failures = 0;
        self.current_delay_secs = 1;
        self.healthy_since = None;
    }

    pub fn mark_healthy(&mut self) {
        if self.healthy_since.is_none() {
            self.healthy_since = Some(Instant::now());
        }
        if let Some(since) = self.healthy_since {
            if since.elapsed().as_secs() >= 60 {
                self.reset();
            }
        }
    }
}
