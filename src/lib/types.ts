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

export interface AppConfig {
  dataDir: string;
  firstRunComplete: boolean;
  shieldMode: boolean;
  walletServer: boolean;
}
