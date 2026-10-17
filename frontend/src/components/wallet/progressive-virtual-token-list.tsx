import { useEffect, useMemo, useRef, useState } from "react";

type TokenLike = {
  name?: string | null;
  symbol?: string | null;
  address?: string | null;
  balance?: unknown;
  decimals?: number | null;
  logo_url?: unknown;
  image_url?: unknown;
  icon_url?: unknown;
  chainName?: string;
  chainSymbol?: string;
  isNative?: boolean;
};

type ProgressiveVirtualTokenListProps = {
  tokens: TokenLike[];
  hidden: boolean;
  formatBalance: (value: unknown, decimals?: number) => string;
};

const ROW_HEIGHT = 40;
const BATCH_SIZE = 8;
const BATCH_DELAY_MS = 40;
const OVERSCAN = 3;

function AssetLogo({
  token,
  name,
  symbol,
}: {
  token: TokenLike;
  name: string;
  symbol: string;
}) {
  const imageUrl = String(token.logo_url ?? token.image_url ?? token.icon_url ?? "").trim();
  const [imageFailed, setImageFailed] = useState(false);
  useEffect(() => setImageFailed(false), [imageUrl]);
  const showImage = Boolean(imageUrl) && !imageFailed;
  return (
    <div className="relative size-7 shrink-0">
      {showImage ? (
        <img
          src={imageUrl}
          alt=""
          loading="lazy"
          className="size-7 rounded-md object-contain"
          onError={() => setImageFailed(true)}
        />
      ) : (
        <span className="flex size-7 items-center justify-center rounded-md bg-secondary text-[9px] font-semibold text-foreground">
          {(symbol || name.slice(0, 3)).slice(0, 3).toUpperCase()}
        </span>
      )}
      {token.chainSymbol && (
        <span
          title={token.chainName ?? "Blockchain"}
          className="absolute -bottom-1 -right-1 flex size-3.5 items-center justify-center rounded-full bg-background px-0.5 text-[6px] font-bold leading-none text-foreground shadow-sm"
        >
          {token.chainSymbol.slice(0, 2)}
        </span>
      )}
    </div>
  );
}

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
      if (currentCount < tokenCount) timer = window.setTimeout(revealNextBatch, BATCH_DELAY_MS);
    };
    timer = window.setTimeout(revealNextBatch, 24);
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [tokenCount]);

  const startIndex = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
  const visibleRowCount = Math.ceil(viewportHeight / ROW_HEIGHT) + OVERSCAN * 2;
  const endIndex = Math.min(renderedCount, startIndex + visibleRowCount);
  const visibleTokens = useMemo(() => tokens.slice(startIndex, endIndex), [tokens, startIndex, endIndex]);
  const totalHeight = renderedCount * ROW_HEIGHT;

  return (
    <div className="relative">
      <div
        ref={scrollRef}
        aria-label="Network and token balances"
        aria-busy={renderedCount < tokenCount}
        className="max-h-[min(48vh,380px)] overflow-y-auto overscroll-contain"
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
              const chainName = String(token.chainName ?? "Blockchain");
              return (
                <div
                  key={`${String(token.address ?? (symbol || "asset"))}-${chainName}-${index}`}
                  className="token-float-row absolute inset-x-0 flex items-center gap-2 py-0.5"
                  style={{ top: index * ROW_HEIGHT, height: ROW_HEIGHT }}
                >
                  <AssetLogo token={token} name={name} symbol={symbol} />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-xs font-medium">{name}</p>
                    <p className="truncate text-[10px] text-muted-foreground">{symbol || "Asset"} <span className="px-0.5">·</span> {chainName}</p>
                  </div>
                  <div className="min-w-0 text-right">
                    <p className="max-w-[38vw] truncate text-xs font-semibold tabular-nums">
                      {hidden ? "••••••" : formatBalance(token.balance, token.decimals ?? undefined)}
                    </p>
                    <p className="text-[10px] text-muted-foreground">{symbol || "Balance"}</p>
                  </div>
                </div>
              );
            })}
          </div>
        ) : tokenCount > 0 ? (
          <div className="flex min-h-10 items-center justify-center gap-2 text-[10px] text-muted-foreground">
            <span className="size-1.5 animate-pulse rounded-full bg-muted-foreground/70" />
            Preparing asset list…
          </div>
        ) : null}
      </div>
      {renderedCount < tokenCount && renderedCount > 0 && (
        <p className="mt-1 text-[10px] text-muted-foreground" aria-live="polite">
          Loading remaining assets… {renderedCount.toLocaleString()} of {tokenCount.toLocaleString()}
        </p>
      )}
    </div>
  );
}
