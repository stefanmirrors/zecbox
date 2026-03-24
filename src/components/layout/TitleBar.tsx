interface Props {
  title: string;
}

export function TitleBar({ title }: Props) {
  return (
    <div
      data-tauri-drag-region
      className="h-12 flex items-center px-6 border-b border-zec-border shrink-0 select-none"
    >
      <h2
        data-tauri-drag-region
        className="text-sm font-medium text-zec-muted"
      >
        {title}
      </h2>
    </div>
  );
}
