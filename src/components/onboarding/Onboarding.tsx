import { useState } from "react";
import { Welcome } from "./Welcome";
import { StorageSelect } from "./StorageSelect";
import { Ready } from "./Ready";

type Step = "welcome" | "storage" | "ready";

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

  return (
    <div className="min-h-screen bg-zec-dark">
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
  );
}
