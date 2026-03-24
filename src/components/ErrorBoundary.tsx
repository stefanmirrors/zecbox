import React from "react";

interface Props {
  children: React.ReactNode;
}

interface State {
  hasError: boolean;
  message: string;
}

export class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, message: "" };
  }

  static getDerivedStateFromError(error: Error): State {
    return {
      hasError: true,
      message: error.message || "An unexpected error occurred.",
    };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex min-h-screen items-center justify-center bg-zec-dark px-6">
          <div className="max-w-md text-center space-y-6">
            <h1 className="text-2xl font-bold text-zec-text">
              Something went wrong
            </h1>
            <p className="text-zec-muted">{this.state.message}</p>
            <button
              onClick={() => window.location.reload()}
              className="px-6 py-2.5 rounded-lg font-medium bg-zec-yellow text-zec-dark hover:brightness-110 transition-colors"
            >
              Restart zecbox
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
