import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useNodeStatus } from "../../hooks/useNodeStatus";
import { useStorage } from "../../hooks/useStorage";
import { useStealthMode } from "../../hooks/useStealthMode";
import { useUpdates } from "../../hooks/useUpdates";
import { formatBytes } from "../../lib/format";
import { getAutoStartEnabled, setAutoStart, rebuildDatabase, resetOnboarding } from "../../lib/tauri";

interface SettingsProps {
  onResetToOnboarding: () => void;
}

export function Settings({ onResetToOnboarding }: SettingsProps) {
  const nodeStatus = useNodeStatus();
  const { storageInfo } = useStorage();
  const { status: stealthStatus } = useStealthMode();
  const [autoStartEnabled, setAutoStartEnabled] = useState(false);
  const [autoStartLoading, setAutoStartLoading] = useState(false);
  const [rebuildConfirm, setRebuildConfirm] = useState(false);
  const [rebuilding, setRebuilding] = useState(false);
  const [resetting, setResetting] = useState(false);
  const [recoveryNeeded, setRecoveryNeeded] = useState(false);
  const [autoStartError, setAutoStartError] = useState<string | null>(null);
  const {
    versions, updateStatus, availableUpdates, checking, error,
    checkNow, applyOne, applyAll, dismiss, clearError,
  } = useUpdates();

  const isUpdating =
    updateStatus.status === "downloading" ||
    updateStatus.status === "installing" ||
    updateStatus.status === "rollingBack";

  useEffect(() => {
    getAutoStartEnabled().then(setAutoStartEnabled).catch(() => {});
  }, []);

  useEffect(() => {
    const unlisten = listen("node_recovery_needed", () => setRecoveryNeeded(true));
    return () => { unlisten.then((f) => f()); };
  }, []);

  const handleAutoStartToggle = async () => {
    setAutoStartLoading(true);
    setAutoStartError(null);
    try {
      const newValue = !autoStartEnabled;
      await setAutoStart(newValue);
      setAutoStartEnabled(newValue);
    } catch {
      setAutoStartError("Failed to change auto-start setting.");
    } finally {
      setAutoStartLoading(false);
    }
  };

  const handleRebuild = async () => {
    setRebuilding(true);
    try {
      await rebuildDatabase();
      setRecoveryNeeded(false);
      setRebuildConfirm(false);
    } catch {
      // handled by node status
    } finally {
      setRebuilding(false);
    }
  };

  return (
    <div className="space-y-8">
      {/* About */}
      <Section title="About">
        <Row label="Version" value={versions?.app ?? "..."} />
        <Row label="Node" value={capitalize(nodeStatus.status)} />
        <Row label="Privacy" value={stealthStatus.enabled ? "Stealth (Tor)" : capitalize(stealthStatus.status)} />
        {storageInfo && <Row label="Data" value={storageInfo.dataDir} mono />}
      </Section>

      {/* System */}
      <Section title="System">
        <div className="flex items-center justify-between">
          <span className="text-sm text-zec-muted">Launch at Login</span>
          <Toggle
            enabled={autoStartEnabled}
            loading={autoStartLoading}
            onToggle={handleAutoStartToggle}
          />
        </div>
        {autoStartError && <p className="text-xs text-red-400/80">{autoStartError}</p>}
      </Section>

      {/* Binaries */}
      <Section
        title="Binaries"
        action={
          <button
            onClick={checkNow}
            disabled={checking || isUpdating}
            className="text-[11px] px-2.5 py-1 rounded-lg border border-zec-border text-zec-muted hover:text-zec-text disabled:opacity-40 transition-colors"
          >
            {checking ? "Checking..." : "Check for Updates"}
          </button>
        }
      >
        <Row label="zebrad" value={versions?.zebrad ?? "..."} mono />
        <Row label="zaino" value={versions?.zaino ?? "..."} mono />
        <Row label="arti" value={versions?.arti ?? "..."} mono />
      </Section>

      {/* Updates available */}
      {availableUpdates.length > 0 && (
        <div className="border border-zec-yellow/20 rounded-xl p-5 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-xs font-medium text-zec-yellow">Updates Available</h3>
            <div className="flex gap-2">
              <button onClick={dismiss} disabled={isUpdating} className="text-[11px] text-zec-muted hover:text-zec-text">
                Dismiss
              </button>
              {availableUpdates.length > 1 && (
                <button onClick={applyAll} disabled={isUpdating} className="text-[11px] px-2.5 py-1 rounded-lg bg-zec-yellow/10 text-zec-yellow hover:bg-zec-yellow/20 transition-colors">
                  Update All
                </button>
              )}
            </div>
          </div>
          {availableUpdates.map((u) => (
            <div key={u.name} className="flex items-center justify-between">
              <div>
                <span className="text-sm text-zec-text">{u.name}</span>
                <span className="text-xs text-zec-muted ml-2">{u.currentVersion} → {u.newVersion}</span>
                <span className="text-xs text-zec-muted ml-2">({formatBytes(u.sizeBytes)})</span>
              </div>
              <button
                onClick={() => applyOne(u.name)}
                disabled={isUpdating}
                className="text-[11px] px-2.5 py-1 rounded-lg bg-zec-yellow/10 text-zec-yellow hover:bg-zec-yellow/20 disabled:opacity-40 transition-colors"
              >
                Update
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Update status */}
      {isUpdating && (
        <div className="border border-zec-border rounded-xl p-4 flex items-center gap-3">
          <div className="w-3.5 h-3.5 border-2 border-zec-muted border-t-zec-text rounded-full animate-spin" />
          <p className="text-sm text-zec-muted">
            {updateStatus.status === "downloading" && `Downloading ${updateStatus.binary}...${updateStatus.progress !== undefined ? ` ${updateStatus.progress}%` : ""}`}
            {updateStatus.status === "installing" && `Installing ${updateStatus.binary}...`}
            {updateStatus.status === "rollingBack" && `Rolling back ${updateStatus.binary}...`}
          </p>
        </div>
      )}

      {updateStatus.status === "complete" && (
        <p className="text-xs text-emerald-400">Updates applied successfully.</p>
      )}

      {error && (
        <div className="flex items-center justify-between">
          <p className="text-xs text-red-400/80">{error}</p>
          <button onClick={clearError} className="text-[11px] text-zec-muted hover:text-zec-text">Dismiss</button>
        </div>
      )}

      {recoveryNeeded && (
        <div className="border border-red-400/20 rounded-xl p-4">
          <p className="text-sm text-red-400/80">
            Node failed to start repeatedly. The database may need to be rebuilt.
          </p>
        </div>
      )}

      {/* Advanced */}
      <Section title="Advanced">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-zec-text">Rebuild Database</p>
            <p className="text-[11px] text-zec-muted">Deletes chain data and re-syncs from scratch.</p>
          </div>
          {!rebuildConfirm ? (
            <button
              onClick={() => setRebuildConfirm(true)}
              disabled={rebuilding}
              className="text-[11px] px-2.5 py-1 rounded-lg border border-red-400/20 text-red-400/80 hover:bg-red-400/5 transition-colors"
            >
              Rebuild
            </button>
          ) : (
            <div className="flex gap-2">
              <button onClick={() => setRebuildConfirm(false)} className="text-[11px] text-zec-muted hover:text-zec-text">Cancel</button>
              <button
                onClick={handleRebuild}
                disabled={rebuilding}
                className="text-[11px] px-2.5 py-1 rounded-lg bg-red-500 text-white hover:bg-red-600 disabled:opacity-50 transition-colors"
              >
                {rebuilding ? "Rebuilding..." : "Confirm"}
              </button>
            </div>
          )}
        </div>
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-zec-text">Reset to Onboarding</p>
            <p className="text-[11px] text-zec-muted">Returns to the first-run setup screen.</p>
          </div>
          <button
            onClick={async () => {
              setResetting(true);
              try { await resetOnboarding(); onResetToOnboarding(); } catch { setResetting(false); }
            }}
            disabled={resetting}
            className="text-[11px] px-2.5 py-1 rounded-lg border border-zec-border text-zec-muted hover:text-zec-text disabled:opacity-40 transition-colors"
          >
            {resetting ? "Resetting..." : "Reset"}
          </button>
        </div>
      </Section>
    </div>
  );
}

function Section({ title, action, children }: { title: string; action?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-medium text-zec-muted">{title}</h3>
        {action}
      </div>
      <div className="border border-zec-border rounded-xl p-5 space-y-3">
        {children}
      </div>
    </div>
  );
}

function Row({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-zec-muted">{label}</span>
      <span className={`text-sm text-zec-text ${mono ? "font-mono text-xs" : ""} truncate max-w-64`} title={value}>
        {value}
      </span>
    </div>
  );
}

function Toggle({ enabled, loading, onToggle }: { enabled: boolean; loading: boolean; onToggle: () => void }) {
  return (
    <button
      onClick={onToggle}
      role="switch"
      aria-checked={enabled}
      disabled={loading}
      className={`relative w-9 h-5 rounded-full transition-colors ${
        enabled ? "bg-zec-yellow" : "bg-zec-border"
      } ${loading ? "opacity-40 cursor-not-allowed" : ""}`}
    >
      <span className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200 ${enabled ? "translate-x-4" : ""}`} />
    </button>
  );
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
