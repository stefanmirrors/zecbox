export type NodeStatusTag = "stopped" | "starting" | "running" | "stopping" | "error";

export interface NodeStatusInfo {
  status: NodeStatusTag;
  blockHeight?: number;
  peerCount?: number;
  message?: string;
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

export interface AppConfig {
  dataDir: string;
  firstRunComplete: boolean;
  shieldMode: boolean;
  walletServer: boolean;
}

export type ShieldStatusTag = "disabled" | "bootstrapping" | "active" | "error" | "interrupted";

export interface ShieldStatusInfo {
  enabled: boolean;
  status: ShieldStatusTag;
  bootstrapProgress?: number;
  message?: string;
}

export type WalletStatusTag = "stopped" | "starting" | "running" | "stopping" | "error";

export interface WalletStatusInfo {
  enabled: boolean;
  status: WalletStatusTag;
  endpoint?: string;
  message?: string;
}

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
