export type NodeStatus = "stopped" | "starting" | "running" | "stopping" | "error";

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
