interface Props {
  onContinue: () => void;
}

export function Welcome({ onContinue }: Props) {
  return (
    <div className="flex min-h-[90vh] items-center justify-center px-6">
      <div className="max-w-sm text-center space-y-10">
        <div className="space-y-4">
          <h1 className="text-4xl font-bold text-zec-yellow tracking-tight">
            zecbox
          </h1>
          <p className="text-zec-muted text-xs tracking-widest uppercase">
            Zcash Full Node
          </p>
        </div>

        <p className="text-zec-muted text-base leading-relaxed">
          Run your own Zcash node with zero configuration. zecbox downloads,
          verifies, and syncs the blockchain automatically.
        </p>

        <button
          onClick={onContinue}
          className="w-full py-3.5 rounded-xl font-semibold bg-zec-yellow text-zec-dark hover:brightness-110 transition-all"
        >
          Get Started
        </button>
      </div>
    </div>
  );
}
