import { cn } from "@/lib/utils";

function mulberry32(seed: number) {
  return () => {
    let t = (seed += 0x6d2b79f5);
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function CollectibleArt({
  seed,
  className,
}: {
  seed: number;
  className?: string;
}) {
  const rand = mulberry32(seed * 97);
  const shapes = Array.from({ length: 7 }, (_, i) => {
    const kind = rand();
    const x = rand() * 80 + 10;
    const y = rand() * 80 + 10;
    const s = rand() * 28 + 8;
    const o = 0.18 + rand() * 0.7;
    return { i, kind, x, y, s, o };
  });

  return (
    <svg
      viewBox="0 0 100 100"
      className={cn("bg-secondary text-foreground", className)}
      aria-hidden
    >
      <rect width="100" height="100" fill="var(--color-secondary)" />
      {shapes.map((sh) =>
        sh.kind > 0.55 ? (
          <rect
            key={sh.i}
            x={sh.x - sh.s / 2}
            y={sh.y - sh.s / 2}
            width={sh.s}
            height={sh.s}
            fill="currentColor"
            opacity={sh.o}
          />
        ) : sh.kind > 0.25 ? (
          <circle
            key={sh.i}
            cx={sh.x}
            cy={sh.y}
            r={sh.s / 2}
            fill="currentColor"
            opacity={sh.o}
          />
        ) : (
          <line
            key={sh.i}
            x1={sh.x - sh.s / 2}
            y1={sh.y}
            x2={sh.x + sh.s / 2}
            y2={sh.y + (sh.kind > 0.12 ? sh.s / 3 : -sh.s / 3)}
            stroke="currentColor"
            strokeWidth="1.2"
            opacity={sh.o}
          />
        ),
      )}
    </svg>
  );
}
