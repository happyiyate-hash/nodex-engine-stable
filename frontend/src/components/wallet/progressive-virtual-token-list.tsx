import { useEffect, useMemo, useRef, useState } from "react";

type TokenLike = {
  name?: string | null;
  symbol?: string | null;
  address?: string | null;
  balance?: unknown;
  decimals?: number | null;
};

type ProgressiveVirtualTokenListProps = {
  tokens: TokenLike[];
  hidden: boolean;
  formatBalance: (value: unknown, decimals?: number) => string;
};

const ROW_HEIGHT = 44;
const BATCH_SIZE = 8;
const BATCH_DELAY_MS = 40;
const OVERSCAN = 3;

export function ProgressiveVirtualTokenList({
  tokens,
  hidden,
  formatBalance,
}: ProgressiveVirtualTokenListProps) {
  const tokenCount = tokens.length;
  const scrollRef = useRef<HTMLDivElement>(null);
  const [renderedCount, setRenderedCount] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(280);

  useEffect(() => {
    let cancelled = false;
    let timer: number | undefined;
    let currentCount = 0;

    setRenderedCount(0);
    setScrollTop(0);
    if (scrollRef.current) scrollRef.current.scrollTop = 0;

    const revealNextBatch = () => {
      if (cancelled) return;
      currentCount = Math.min(currentCount + BATCH_SIZE, tokenCount);
      setRenderedCount(currentCount);
      if (currentCount < tokenCount) {
        timer = window.setTimeout(revealNextBatch, BATCH_DELAY_MS);
      }
    };

    timer = window.setTimeout(revealNextBatch, 24);
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [tokenCount]);

  const startIndex = Math.max(
    0,
    Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN,
  );
  const visibleRowCount = Math.ceil(viewportHeight / ROW_HEIGHT) + OVERSCAN * 2;
  const endIndex = Math.min(renderedCount, startIndex + visibleRowCount);
  const visibleTokens = useMemo(
    () => tokens.slice(startIndex, endIndex),
    [tokens, startIndex, endIndex],
  );
  const totalHeight = renderedCount * ROW_HEIGHT;

  return (
    <div className="relative">
      <div
        ref={scrollRef}
        aria-label="Token balances"
        aria-busy={renderedCount < tokenCount}
        className="max-h-[min(42vh,320px)] overflow-y-auto overscroll-contain rounded-lg scrollbar-thin"
        onScroll={(event) => {
          setScrollTop(event.currentTarget.scrollTop);
          setViewportHeight(event.currentTarget.clientHeight || 280);
        }}
      >
        {renderedCount > 0 ? (
          <div className="relative w-full" style={{ height: totalHeight }}>
            {visibleTokens.map((token, offset) => {
              const index = startIndex + offset;
              const name = String(token.name ?? token.symbol ?? "Unknown token").trim() || "Unknown token";
              const symbol = String(token.symbol ?? "").trim();
              const address = String(token.address ?? "").trim();

              return (
                <div
                  key={`${address || symbol || "token"}-${index}`}
                  className="token-float-row absolute inset-x-0 flex items-center gap-2 py-1"
                  style={{ top: index * ROW_HEIGHT, height: ROW_HEIGHT }}
                >
                  <div className="grid size-7 shrink-0 place-items-center rounded-full bg-secondary text-[9px] font-semibold">
                    {(symbol || name.slice(0, 3)).slice(0, 3).toUpperCase()}
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-[11px] font-medium">{name}</p>
                    <p className="truncate text-[10px] text-muted-foreground">{symbol || "Token asset"}</p>
                  </div>
                  <div className="min-w-0 text-right">
                    <p className="max-w-[38vw] truncate text-xs font-semibold tabular-nums">
                      {hidden ? "••••••" : formatBalance(token.balance, token.decimals ?? undefined)}
                    </p>
                    <p className="text-[10px] text-muted-foreground">{symbol || "Token balance"}</p>
                  </div>
                </div>
              );
            })}
          </div>
        ) : tokenCount > 0 ? (
          <div className="flex min-h-12 items-center justify-center gap-2 text-[10px] text-muted-foreground">
            <span className="size-1.5 animate-pulse rounded-full bg-muted-foreground/70" />
            Preparing token list…
          </div>
        ) : null}
      </div>
      {renderedCount < tokenCount && renderedCount > 0 && (
        <p className="mt-1.5 text-[10px] text-muted-foreground" aria-live="polite">
          Loading remaining tokens… {renderedCount.toLocaleString()} of {tokenCount.toLocaleString()}
        </p>
      )}
    </div>
  );
}
