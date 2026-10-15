import { cn } from "@/lib/utils";

export function SableMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 32 32"
      className={cn("text-foreground", className)}
      aria-hidden
    >
      <circle cx="16" cy="16" r="14" fill="currentColor" />
      <circle cx="21" cy="16" r="11" fill="var(--color-background)" />
    </svg>
  );
}

export function SableWordmark({ className }: { className?: string }) {
  return (
    <span className={cn("flex items-center gap-2.5", className)}>
      <SableMark className="size-6" />
      <span className="text-[15px] font-medium tracking-tight">Sable</span>
    </span>
  );
}
