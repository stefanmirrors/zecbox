interface Props {
  title: string;
}

export function TitleBar({ title }: Props) {
  return (
    <div
      data-tauri-drag-region
      className="h-10 flex items-center px-6 shrink-0 select-none"
    >
      <h2
        data-tauri-drag-region
        className="text-xs font-medium text-zec-muted/50"
      >
        {title}
      </h2>
    </div>
  );
}
