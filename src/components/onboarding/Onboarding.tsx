import { useState } from "react";
import { Welcome } from "./Welcome";
import { StorageSelect } from "./StorageSelect";
import { Ready } from "./Ready";

type Step = "welcome" | "storage" | "ready";

const steps: Step[] = ["welcome", "storage", "ready"];

interface Props {
  onComplete: () => void;
}

export function Onboarding({ onComplete }: Props) {
  const [step, setStep] = useState<Step>("welcome");
  const [selectedPath, setSelectedPath] = useState<string>("");

  const handleStorageSelect = (path: string) => {
    setSelectedPath(path);
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
        {step === "ready" && (
          <Ready selectedPath={selectedPath} onComplete={onComplete} />
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
