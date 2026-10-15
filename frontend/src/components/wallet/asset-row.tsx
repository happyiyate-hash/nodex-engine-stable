import { Link } from "@tanstack/react-router";
import { TokenIcon } from "@/components/wallet/token-icon";
import { Sparkline } from "@/components/wallet/sparkline";
import { ASSETS } from "@/lib/wallet/assets";
import { formatTokenAmount, formatUsd } from "@/lib/utils";
import type { HoldingRow } from "@/lib/wallet/select";
import { cn } from "@/lib/utils";

export function AssetRow({
  row,
  hidden,
}: {
  row: HoldingRow;
  hidden: boolean;
}) {
  const asset = ASSETS[row.id];
  const up = row.change >= 0;
  return (
    <Link
      to="/asset/$id"
      params={{ id: row.id }}
      className="flex items-center gap-3 rounded-lg px-2 py-3 transition-colors duration-150 hover:bg-accent"
    >
      <TokenIcon id={row.id} />
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline justify-between gap-3">
          <p className="truncate text-sm font-medium">{asset.name}</p>
          <p className="tabular text-sm font-medium">
            {hidden ? "••••" : formatUsd(row.value)}
          </p>
        </div>
        <div className="mt-0.5 flex items-center justify-between gap-3">
          <p className="text-xs text-muted-foreground">
            {hidden ? "••••" : formatTokenAmount(row.amount)} {asset.symbol}
          </p>
          <p
            className={cn(
              "tabular text-xs",
              up ? "text-gain" : "text-loss",
            )}
          >
            {up ? "+" : ""}
            {row.change.toFixed(2)}%
          </p>
        </div>
      </div>
      <Sparkline
        seed={row.id}
        end={row.price}
        changePct={row.change}
        className="hidden h-8 w-16 sm:block"
      />
    </Link>
  );
}
