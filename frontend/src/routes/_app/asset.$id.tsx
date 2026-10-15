import { useMemo } from "react";
import { Link, createFileRoute, notFound } from "@tanstack/react-router";
import { ArrowDownLeft, ArrowLeft, ArrowUpRight, CircleHelp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { TokenIcon } from "@/components/wallet/token-icon";
import { ASSETS, type AssetId } from "@/lib/wallet/assets";
import { usePrices } from "@/lib/wallet/prices";
import { useWallet } from "@/lib/wallet/store";
import { formatTokenAmount, formatUsd } from "@/lib/utils";

export const Route = createFileRoute("/_app/asset/$id")({ component: AssetPage });
const IDS = new Set(Object.keys(ASSETS));

function AssetPage() {
  const { id } = Route.useParams();
  if (!IDS.has(id)) throw notFound();
  const assetId = id as AssetId;
  const asset = ASSETS[assetId];
  const wallet = useWallet((s) => s.wallet);
  const balances = useWallet((s) => s.balances);
  const hide = useWallet((s) => s.hideBalances);
  const prices = usePrices((s) => s.prices);
  const priceStatus = usePrices((s) => s.status);
  const price = prices[assetId]?.usd ?? 0;
  const change = prices[assetId]?.usd_24h_change ?? 0;
  const amount = useMemo(() => {
    const humanAmount = (value: unknown, decimals?: number) => {
      const raw = String(value ?? "0").trim().replace(/,/g, "");
      const parsed = Number(raw);
      if (!Number.isFinite(parsed) || parsed < 0) return 0;
      return Number.isInteger(decimals) && Number(decimals) >= 0 && Number(decimals) <= 36
        ? parsed / 10 ** Number(decimals)
        : parsed;
    };
    let total = 0;
    for (const balance of balances) {
      const nativeSymbol = String(balance.native?.symbol ?? "").toUpperCase();
      if (nativeSymbol === asset.symbol.toUpperCase()) {
        total += humanAmount(balance.native?.balance, balance.native?.decimals);
      }
      for (const token of balance.tokens ?? []) {
        if (String(token.symbol ?? "").toUpperCase() === asset.symbol.toUpperCase()) {
          total += humanAmount(token.balance, token.decimals);
        }
      }
    }
    return total;
  }, [balances, asset.symbol]);
  const value = amount * price;
  const up = change >= 0;

  return <div className="space-y-5 pb-4">
    <div className="flex items-center gap-3"><TokenIcon id={assetId} className="size-12" /><div className="min-w-0"><h1 className="text-2xl font-semibold tracking-tight">{asset.name}</h1><p className="mt-1 text-sm text-muted-foreground">{asset.networkLabel}</p></div></div>
    <section className="rounded-3xl border border-border bg-card p-5">
      <p className="text-sm text-muted-foreground">Market price</p>
      <p className="mt-2 text-3xl font-semibold tracking-tight tabular-nums">{hide ? "••••••" : formatUsd(price, { digits: price < 2 ? 4 : 2 })}</p>
      <p className={`mt-2 text-sm tabular-nums ${up ? "text-gain" : "text-loss"}`}>{up ? "+" : ""}{change.toFixed(2)}% <span className="text-muted-foreground">24h</span></p>
      <p className="mt-3 text-xs text-muted-foreground">{priceStatus === "live" ? "Market data refreshed from the price provider." : "Market price may be a fallback estimate until live price data loads."}</p>
    </section>
    <section className="rounded-2xl border border-border bg-card p-4">
      <p className="text-sm text-muted-foreground">Your holdings</p>
      <p className="mt-2 text-2xl font-semibold tabular-nums">{hide ? "••••••" : `${formatTokenAmount(amount)} ${asset.symbol}`}</p>
      <p className="mt-1 text-sm text-muted-foreground">Estimated value: {hide ? "••••••" : formatUsd(value)}</p>
      {!balances.length && <div className="mt-4 flex items-start gap-2 rounded-xl bg-secondary/60 p-3"><CircleHelp className="mt-0.5 size-4 shrink-0 text-muted-foreground" /><p className="text-xs leading-relaxed text-muted-foreground">No live holdings have been loaded yet. Refresh the portfolio to request balances from NodeX.</p></div>}
    </section>
    <div className="grid grid-cols-2 gap-2"><Button size="lg" asChild><Link to="/send"><ArrowUpRight className="size-4" /> Send</Link></Button><Button size="lg" variant="secondary" asChild><Link to="/receive"><ArrowDownLeft className="size-4" /> Receive</Link></Button></div>
    <Button variant="outline" className="w-full" asChild><Link to="/blockchains"><ArrowLeft className="size-4" /> All blockchains</Link></Button>
    <p className="text-center text-xs text-muted-foreground">{wallet?.name ?? "Wallet"} · Testing integration</p>
  </div>;
}
