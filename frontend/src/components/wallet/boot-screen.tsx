import { SableMark } from "@/components/brand/sable-mark";

export function BootScreen() {
  return (
    <div className="flex min-h-dvh flex-col items-center justify-center gap-4 bg-background">
      <SableMark className="size-12 animate-[sable-pulse_1.6s_ease-in-out_infinite]" />
      <p className="text-sm font-medium tracking-tight text-muted-foreground">Sable</p>
    </div>
  );
}
