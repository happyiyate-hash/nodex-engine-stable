import type { ReactNode } from "react";
import { cn } from "@/lib/utils";
import type { AssetId } from "@/lib/wallet/assets";

const icons: Record<AssetId, ReactNode> = {
  bitcoin: (
    <path
      d="M16 5.5A10.5 10.5 0 1 1 5.5 16 10.5 10.5 0 0 1 16 5.5Zm.4 4.2v1.6c1.6.1 2.7.8 2.7 2.3 0 1.2-.7 1.9-1.8 2.1 1.3.3 2.2 1.1 2.2 2.5 0 1.8-1.4 2.7-3.1 2.8v1.6h-1.6v-1.6h-1.1v1.6H12v-1.6h-2.1v-1.4h1.3c.5 0 .8-.3.8-.8v-7.1c0-.5-.3-.8-.8-.8H9.9V10h2.1V8.5h1.6V10h1.1V8.5h1.6ZM14.7 16.6h1.9c.9 0 1.5-.5 1.5-1.3s-.6-1.3-1.5-1.3h-1.9Zm0 1.4v2.5h2.1c1 0 1.7-.5 1.7-1.3s-.7-1.2-1.7-1.2Z"
      fill="currentColor"
    />
  ),
  ethereum: (
    <path
      d="M16 5.5 9.5 16.2 16 20l6.5-3.8Zm0 16.1-6.5-3.7L16 26.5l6.5-8.6Z"
      fill="currentColor"
    />
  ),
  solana: (
    <>
      <path d="M10.2 11.2h10.4l-2.1-2.1H8.1Z" fill="currentColor" />
      <path d="M8.1 16.9h10.4l2.1-2.1H10.2Z" fill="currentColor" />
      <path d="M10.2 22.9h10.4l-2.1-2.1H8.1Z" fill="currentColor" />
    </>
  ),
  "usd-coin": (
    <>
      <circle
        cx="16"
        cy="16"
        r="8.5"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
      />
      <path
        d="M16.7 12.2v-.9h-1.4v.9c-1.4.2-2.3 1-2.3 2.2 0 1.4 1.1 2 2.8 2.3 1.3.2 1.6.5 1.6 1s-.6.8-1.5.8c-.8 0-1.4-.3-1.6-1l-1.5.3c.3 1.3 1.3 2 3.1 2.2v.9h1.4v-.9c1.5-.2 2.4-1.1 2.4-2.3 0-1.4-1.1-2-2.9-2.3-1.2-.2-1.5-.5-1.5-1s.6-.8 1.4-.8c.8 0 1.2.3 1.4.8l1.5-.4c-.3-1.1-1.2-1.8-2.8-2Z"
        fill="currentColor"
      />
    </>
  ),
  chainlink: (
    <path
      d="M16 6.5 24 11v10l-8 4.5L8 21V11Zm0 2.3L10.2 12v8L16 23.2 21.8 20v-8Z"
      fill="currentColor"
    />
  ),
  uniswap: (
    <path
      d="M11.2 8.5c2.6 3.4 3.4 7.6 2.2 11.4-.5 1.7-1.4 3.2-1.1 4.1.3.8 1.4.7 2.4.1 2.5-1.4 4.8-4.6 5.8-8.2.8-3.1.4-6.1-1.3-7.6 1.9 1 3.2 3.6 3.2 6.7 0 4.8-3.4 8.9-7.8 9.6-1.8.3-3.2-.3-3.7-1.6-.5-1.3.1-3 1-4.6 1.4-2.5 1.6-5.3.3-7.6-.6-1-1.5-1.8-2.4-2.3Z"
      fill="currentColor"
    />
  ),
};

export function TokenIcon({
  id,
  className,
}: {
  id: AssetId;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex size-10 items-center justify-center rounded-full bg-secondary text-foreground shadow-[var(--shadow-border)]",
        className,
      )}
    >
      <svg viewBox="0 0 32 32" className="size-[22px]" aria-hidden>
        {icons[id]}
      </svg>
    </span>
  );
}
