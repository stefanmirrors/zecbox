export type NodeStatusTag = "stopped" | "starting" | "running" | "stopping" | "error";

export interface NodeStatusInfo {
  status: NodeStatusTag;
  blockHeight?: number;
  peerCount?: number;
  estimatedHeight?: number;
  bestBlockHash?: string;
  syncPercentage?: number;
  chain?: string;
  message?: string;
  progress?: number;
}

export interface Volume {
  name: string;
  mountPoint: string;
  totalBytes: number;
  availableBytes: number;
  isRemovable: boolean;
}

export type StorageWarningLevel = "none" | "warning" | "critical" | "paused";

export interface StorageInfo {
  dataDir: string;
  volumeName: string;
  totalBytes: number;
  availableBytes: number;
  isExternal: boolean;
  warningLevel: StorageWarningLevel;
}

export type PrivacyMode = "standard" | "stealth" | "proxy" | "shield";

export interface AppConfig {
  dataDir: string;
  firstRunComplete: boolean;
  privacyMode: PrivacyMode;
  walletServer: boolean;
  autoStart: boolean;
  serveNetwork: boolean;
}

// --- Stealth Mode (Tor) types ---

export type StealthStatusTag = "disabled" | "bootstrapping" | "active" | "error" | "interrupted";

export interface StealthStatusInfo {
  enabled: boolean;
  status: StealthStatusTag;
  bootstrapProgress?: number;
  message?: string;
}

// --- Proxy Mode (VPS relay) types ---

export type ProxyStatusTag = "disabled" | "setup" | "connecting" | "active" | "error" | "interrupted";

export interface ProxyStatusInfo {
  enabled: boolean;
  status: ProxyStatusTag;
  vpsIp?: string;
  lastHandshakeSecs?: number;
  relayReachable?: boolean;
  message?: string;
  step?: string;
}

export interface ProxySetupConfig {
  vpsWgConf: string;
  dockerCompose: string;
  installCommand: string;
  homeWgConf: string;
}

export interface VpsProvider {
  name: string;
  url: string;
  acceptsZec: boolean;
  noKyc: boolean;
  locations: string[];
  description: string;
  tiers: VpsTier[];
}

export interface VpsTier {
  useCase: string;
  minRamMb: number;
  minStorageGb: number;
  estimatedCost: string;
}

// --- Wallet Server types ---

export type WalletStatusTag = "stopped" | "starting" | "running" | "stopping" | "error";

export interface WalletStatusInfo {
  enabled: boolean;
  status: WalletStatusTag;
  endpoint?: string;
  message?: string;
}

// --- Update types ---

export type UpdateStatusTag =
  | "idle"
  | "checking"
  | "updateAvailable"
  | "downloading"
  | "installing"
  | "rollingBack"
  | "error"
  | "complete";

export interface UpdateStatusInfo {
  status: UpdateStatusTag;
  binary?: string;
  progress?: number;
  message?: string;
}

export interface BinaryUpdateInfo {
  name: string;
  currentVersion: string;
  newVersion: string;
  downloadUrl: string;
  sha256: string;
  sizeBytes: number;
}

export interface VersionInfo {
  app: string;
  zebrad: string;
  zaino: string;
  arti: string;
}

// --- Network Serve types ---

export type NetworkServeStatusTag = "disabled" | "enabling" | "active" | "error";

export interface NetworkServeStatusInfo {
  enabled: boolean;
  status: NetworkServeStatusTag;
  publicIp?: string;
  reachable?: boolean;
  inboundPeers?: number;
  outboundPeers?: number;
  upnpActive?: boolean;
  localIp?: string;
  cgnatDetected?: boolean;
  message?: string;
}

export interface NodeStats {
  totalUptimeSecs: number;
  blocksValidated: number;
  walletsServed: number;
  currentStreakDays: number;
  bestStreakDays: number;
  lastOnlineDate: string | null;
  firstStarted: string | null;
}
