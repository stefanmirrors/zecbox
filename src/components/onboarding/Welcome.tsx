interface Props {
  onContinue: () => void;
}

export function Welcome({ onContinue }: Props) {
  return (
    <div className="flex min-h-screen items-center justify-center px-6">
      <div className="max-w-md text-center space-y-8">
        <div className="space-y-2">
          <div className="text-6xl mb-4 select-none" aria-hidden>
            &#x1F6E1;
          </div>
          <h1 className="text-5xl font-bold text-zec-yellow tracking-tight">
            ZecBox
          </h1>
          <p className="text-zec-muted text-sm tracking-wide uppercase">
            Zcash Full Node
          </p>
        </div>

        <p className="text-zec-text text-lg leading-relaxed">
          Run your own Zcash full node with zero configuration. ZecBox
          downloads, verifies, and syncs the Zcash blockchain automatically.
          Your node, your privacy.
        </p>

        <button
          onClick={onContinue}
          className="w-full py-3 rounded-lg font-semibold text-lg bg-zec-yellow text-zec-dark hover:brightness-110 transition-all"
        >
          Continue
        </button>
      </div>
    </div>
  );
}
