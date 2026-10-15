import { cn } from "@/lib/utils";
import { sparklinePath } from "@/lib/wallet/prices";

export function Sparkline({
  seed,
  end,
  changePct,
  className,
}: {
  seed: string;
  end: number;
  changePct: number;
  className?: string;
}) {
  const d = sparklinePath(seed, end, changePct);
  const up = changePct >= 0;
  return (
    <svg
      viewBox="0 0 100 32"
      className={cn("overflow-visible", className)}
      aria-hidden
    >
      <path
        d={d}
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
        className={up ? "text-gain" : "text-loss"}
      />
    </svg>
  );
}
