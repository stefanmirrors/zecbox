import { useState } from "react";
import { Welcome } from "./Welcome";
import { StorageSelect } from "./StorageSelect";
import { ShieldSelect } from "./ShieldSelect";
import { Ready } from "./Ready";

type Step = "welcome" | "storage" | "shield" | "ready";

const steps: Step[] = ["welcome", "storage", "shield", "ready"];

interface Props {
  onComplete: () => void;
}

export function Onboarding({ onComplete }: Props) {
  const [step, setStep] = useState<Step>("welcome");
  const [selectedPath, setSelectedPath] = useState<string>("");
  const [shieldMode, setShieldMode] = useState(false);

  const handleStorageSelect = (path: string) => {
    setSelectedPath(path);
    setStep("shield");
  };

  const handleShieldSelect = (shield: boolean) => {
    setShieldMode(shield);
    setStep("ready");
  };

  const currentIndex = steps.indexOf(step);

  return (
    <div className="min-h-screen bg-zec-dark flex flex-col">
      <div className="flex-1">
        {step === "welcome" && (
          <Welcome onContinue={() => setStep("storage")} />
        )}
        {step === "storage" && (
          <StorageSelect onSelect={handleStorageSelect} />
        )}
        {step === "shield" && (
          <ShieldSelect onSelect={handleShieldSelect} />
        )}
        {step === "ready" && (
          <Ready selectedPath={selectedPath} shieldMode={shieldMode} onComplete={onComplete} />
        )}
      </div>

      {/* Step indicator */}
      <div className="flex justify-center gap-2 pb-8">
        {steps.map((s, i) => (
          <div
            key={s}
            className={`h-1 rounded-full transition-all duration-300 ${
              i <= currentIndex ? "w-6 bg-zec-yellow" : "w-2 bg-zec-border"
            }`}
          />
        ))}
      </div>
    </div>
  );
}
