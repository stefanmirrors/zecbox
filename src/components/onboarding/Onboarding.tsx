import { useState } from "react";
import type { PrivacyMode } from "../../lib/types";
import { Welcome } from "./Welcome";
import { StorageSelect } from "./StorageSelect";
import { ShieldSelect } from "./ShieldSelect";
import { Ready } from "./Ready";

type Step = "welcome" | "storage" | "privacy" | "ready";

const steps: Step[] = ["welcome", "storage", "privacy", "ready"];

interface Props {
  onComplete: () => void;
}

export function Onboarding({ onComplete }: Props) {
  const [step, setStep] = useState<Step>("welcome");
  const [selectedPath, setSelectedPath] = useState<string>("");
  const [privacyMode, setPrivacyMode] = useState<PrivacyMode>("standard");

  const handleStorageSelect = (path: string) => {
    setSelectedPath(path);
    setStep("privacy");
  };

  const handlePrivacySelect = (mode: PrivacyMode) => {
    setPrivacyMode(mode);
    setStep("ready");
  };

  const goBack = () => {
    const idx = steps.indexOf(step);
    if (idx > 0) setStep(steps[idx - 1]);
  };

  const currentIndex = steps.indexOf(step);

  return (
    <div className="min-h-screen bg-zec-dark flex flex-col">
      {/* Back button */}
      {step !== "welcome" && (
        <div className="px-6 pt-5">
          <button
            onClick={goBack}
            className="text-xs text-zec-muted hover:text-zec-text transition-colors flex items-center gap-1"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M19 12H5" />
              <path d="M12 19l-7-7 7-7" />
            </svg>
            Back
          </button>
        </div>
      )}

      <div className="flex-1">
        {step === "welcome" && (
          <Welcome onContinue={() => setStep("storage")} />
        )}
        {step === "storage" && (
          <StorageSelect onSelect={handleStorageSelect} />
        )}
        {step === "privacy" && (
          <ShieldSelect onSelect={handlePrivacySelect} />
        )}
        {step === "ready" && (
          <Ready selectedPath={selectedPath} privacyMode={privacyMode} onComplete={onComplete} />
        )}
      </div>

      {/* Step indicator */}
      <div className="flex justify-center gap-2 pb-8">
        {steps.map((s, i) => (
          <button
            key={s}
            onClick={() => i < currentIndex && setStep(steps[i])}
            disabled={i >= currentIndex}
            className={`h-1 rounded-full transition-all duration-300 ${
              i <= currentIndex ? "w-6 bg-zec-yellow" : "w-2 bg-zec-border"
            } ${i < currentIndex ? "cursor-pointer hover:brightness-125" : "cursor-default"}`}
          />
        ))}
      </div>
    </div>
  );
}
