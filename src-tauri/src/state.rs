use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub const LOG_BUFFER_CAPACITY: usize = 5000;

pub struct AppState {
    pub node: Arc<NodeState>,
    pub storage: Arc<StorageState>,
    pub shield: Arc<ShieldState>,
    pub wallet: Arc<WalletState>,
    pub default_data_dir: PathBuf,
    pub tray_status: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
}

impl AppState {
    pub fn new(data_dir: PathBuf, default_data_dir: PathBuf) -> Self {
        Self {
            node: Arc::new(NodeState::new(data_dir)),
            storage: Arc::new(StorageState::new()),
            shield: Arc::new(ShieldState::new()),
            wallet: Arc::new(WalletState::new()),
            default_data_dir,
            tray_status: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub is_removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageWarningLevel {
    None,
    Warning,
    Critical,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageInfo {
    pub data_dir: String,
    pub volume_name: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub is_external: bool,
    pub warning_level: StorageWarningLevel,
}

pub struct StorageState {
    pub monitor_task: Mutex<Option<JoinHandle<()>>>,
    pub paused_low_space: Mutex<bool>,
    pub drive_connected: Mutex<bool>,
}

impl StorageState {
    pub fn new() -> Self {
        Self {
            monitor_task: Mutex::new(None),
            paused_low_space: Mutex::new(false),
            drive_connected: Mutex::new(true),
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

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum ShieldStatus {
    Disabled,
    #[serde(rename_all = "camelCase")]
    Bootstrapping {
        progress: u8,
    },
    Active,
    Error {
        message: String,
    },
    Interrupted,
}

pub struct ShieldState {
    pub status: Mutex<ShieldStatus>,
    pub process: Mutex<Option<tokio::process::Child>>,
    pub bootstrap_task: Mutex<Option<JoinHandle<()>>>,
    pub kill_switch_task: Mutex<Option<JoinHandle<()>>>,
}

impl ShieldState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(ShieldStatus::Disabled),
            process: Mutex::new(None),
            bootstrap_task: Mutex::new(None),
            kill_switch_task: Mutex::new(None),
        }
    }

    pub async fn is_active(&self) -> bool {
        matches!(*self.status.lock().await, ShieldStatus::Active)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum WalletStatus {
    Stopped,
    Starting,
    #[serde(rename_all = "camelCase")]
    Running {
        endpoint: String,
    },
    Stopping,
    Error {
        message: String,
    },
}

impl WalletStatus {
    pub fn status_str(&self) -> &'static str {
        match self {
            WalletStatus::Stopped => "stopped",
            WalletStatus::Starting => "starting",
            WalletStatus::Running { .. } => "running",
            WalletStatus::Stopping => "stopping",
            WalletStatus::Error { .. } => "error",
        }
    }

    pub fn is_stopped_or_error(&self) -> bool {
        matches!(self, WalletStatus::Stopped | WalletStatus::Error { .. })
    }
}

pub struct WalletState {
    pub status: Mutex<WalletStatus>,
    pub process: Mutex<Option<tokio::process::Child>>,
    pub health_task: Mutex<Option<JoinHandle<()>>>,
    pub log_reader_tasks: Mutex<Vec<JoinHandle<()>>>,
    pub log_buffer: Mutex<VecDeque<String>>,
    pub backoff: Mutex<BackoffState>,
}

impl WalletState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(WalletStatus::Stopped),
            process: Mutex::new(None),
            health_task: Mutex::new(None),
            log_reader_tasks: Mutex::new(Vec::new()),
            log_buffer: Mutex::new(VecDeque::with_capacity(LOG_BUFFER_CAPACITY)),
            backoff: Mutex::new(BackoffState::default()),
        }
    }
}
