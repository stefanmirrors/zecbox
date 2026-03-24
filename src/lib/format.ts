export function formatBytes(bytes: number): string {
  if (bytes >= 1_000_000_000_000) {
    return `${(bytes / 1_000_000_000_000).toFixed(1)} TB`;
  }
  return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
}
