use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub const LOG_BUFFER_CAPACITY: usize = 5000;

pub struct AppState {
    pub node: Arc<NodeState>,
    pub storage: Arc<StorageState>,
    pub shield: Arc<ShieldState>,
    pub wallet: Arc<WalletState>,
    pub update: Arc<UpdateState>,
    pub network: Arc<NetworkServeState>,
    pub default_data_dir: PathBuf,
    pub tray_status: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    pub power_thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    pub power_wake_task: Mutex<Option<JoinHandle<()>>>,
}

impl AppState {
    pub fn new(data_dir: PathBuf, default_data_dir: PathBuf) -> Self {
        Self {
            node: Arc::new(NodeState::new(data_dir)),
            storage: Arc::new(StorageState::new()),
            shield: Arc::new(ShieldState::new()),
            wallet: Arc::new(WalletState::new()),
            update: Arc::new(UpdateState::new()),
            network: Arc::new(NetworkServeState::new()),
            default_data_dir,
            tray_status: Mutex::new(None),
            power_thread: Mutex::new(None),
            power_wake_task: Mutex::new(None),
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
    pub stats: Mutex<NodeStats>,
    pub last_block_height: Mutex<u64>,
}

impl NodeState {
    pub fn new(data_dir: PathBuf) -> Self {
        let mut stats = NodeStats::load(&data_dir);

        // Reset stats if the chain database doesn't exist (fresh install or rebuild)
        let zebra_dir = data_dir.join("zebra");
        let is_fresh = !zebra_dir.exists() || zebra_dir.read_dir().map_or(true, |mut d| d.next().is_none());
        if is_fresh {
            stats = NodeStats::default();
            stats.save(&data_dir);
        }

        Self {
            status: Mutex::new(NodeStatus::Stopped),
            process: Mutex::new(None),
            health_task: Mutex::new(None),
            log_reader_tasks: Mutex::new(Vec::new()),
            log_buffer: Mutex::new(VecDeque::with_capacity(LOG_BUFFER_CAPACITY)),
            stats: Mutex::new(stats),
            last_block_height: Mutex::new(0),
            data_dir: Mutex::new(data_dir),
            backoff: Mutex::new(BackoffState::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum NodeStatus {
    Stopped,
    Starting {
        message: Option<String>,
        progress: Option<f64>,
    },
    #[serde(rename_all = "camelCase")]
    Running {
        block_height: u64,
        peer_count: u32,
        estimated_height: Option<u64>,
        best_block_hash: Option<String>,
        sync_percentage: Option<f64>,
        chain: Option<String>,
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
            NodeStatus::Starting { .. } => "starting",
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
    pub onion_address: Mutex<Option<String>>,
}

impl ShieldState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(ShieldStatus::Disabled),
            process: Mutex::new(None),
            bootstrap_task: Mutex::new(None),
            kill_switch_task: Mutex::new(None),
            onion_address: Mutex::new(None),
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

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpdateAvailable,
    #[serde(rename_all = "camelCase")]
    Downloading {
        binary: String,
        progress: u8,
    },
    Installing {
        binary: String,
    },
    RollingBack {
        binary: String,
    },
    Error {
        message: String,
    },
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryUpdateInfo {
    pub name: String,
    pub current_version: String,
    pub new_version: String,
    pub download_url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionInfo {
    pub app: String,
    pub zebrad: String,
    pub zaino: String,
    pub arti: String,
}

pub struct UpdateState {
    pub status: Mutex<UpdateStatus>,
    pub available_updates: Mutex<Vec<BinaryUpdateInfo>>,
    pub check_task: Mutex<Option<JoinHandle<()>>>,
}

impl UpdateState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(UpdateStatus::Idle),
            available_updates: Mutex::new(Vec::new()),
            check_task: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum NetworkServeStatus {
    Disabled,
    Enabling,
    #[serde(rename_all = "camelCase")]
    Active {
        public_ip: Option<String>,
        reachable: Option<bool>,
        inbound_peers: Option<u32>,
        outbound_peers: Option<u32>,
        upnp_active: bool,
        local_ip: Option<String>,
        cgnat_detected: bool,
    },
    Error {
        message: String,
    },
}

pub struct NetworkServeState {
    pub status: Mutex<NetworkServeStatus>,
    pub monitor_task: Mutex<Option<JoinHandle<()>>>,
}

impl NetworkServeState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(NetworkServeStatus::Disabled),
            monitor_task: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeStats {
    pub total_uptime_secs: u64,
    pub blocks_validated: u64,
    pub wallets_served: u64,
    pub current_streak_days: u32,
    pub best_streak_days: u32,
    pub last_online_date: Option<String>,
    pub first_started: Option<String>,
}

impl Default for NodeStats {
    fn default() -> Self {
        Self {
            total_uptime_secs: 0,
            blocks_validated: 0,
            wallets_served: 0,
            current_streak_days: 0,
            best_streak_days: 0,
            last_online_date: None,
            first_started: None,
        }
    }
}

impl NodeStats {
    pub fn load(data_dir: &PathBuf) -> Self {
        let path = data_dir.join("config").join("node_stats.json");
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, data_dir: &PathBuf) {
        let config_dir = data_dir.join("config");
        let _ = std::fs::create_dir_all(&config_dir);
        let path = config_dir.join("node_stats.json");
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn record_uptime_tick(&mut self, secs: u64) {
        self.total_uptime_secs += secs;
    }

    pub fn record_blocks(&mut self, new_height: u64, prev_height: u64) {
        if new_height > prev_height {
            self.blocks_validated += new_height - prev_height;
        }
    }

    pub fn update_streak(&mut self) {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        match &self.last_online_date {
            Some(last) if *last == today => {}
            Some(last) => {
                if let Ok(last_date) = NaiveDate::parse_from_str(last, "%Y-%m-%d") {
                    let today_date = Utc::now().date_naive();
                    let diff = (today_date - last_date).num_days();
                    if diff == 1 {
                        self.current_streak_days += 1;
                    } else if diff > 1 {
                        self.current_streak_days = 1;
                    }
                }
                self.last_online_date = Some(today);
            }
            None => {
                self.current_streak_days = 1;
                self.last_online_date = Some(today);
                if self.first_started.is_none() {
                    self.first_started = Some(Utc::now().to_rfc3339());
                }
            }
        }
        if self.current_streak_days > self.best_streak_days {
            self.best_streak_days = self.current_streak_days;
        }
    }
}
