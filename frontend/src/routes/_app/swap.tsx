import { useState } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowDownUp, ArrowLeft, ArrowLeftRight, ShieldAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useWallet } from "@/lib/wallet/store";

export const Route = createFileRoute("/_app/swap")({ component: SwapPage });
const SWAP_ASSETS = ["ETH", "BTC", "SOL", "USDC", "LINK", "UNI"];

function SwapPage() {
  const balances = useWallet((s) => s.balances);
  const [from, setFrom] = useState("ETH");
  const [to, setTo] = useState("USDC");
  const [amount, setAmount] = useState("");
  const available = balances.flatMap((balance) => [balance.native?.symbol, ...(balance.tokens ?? []).map((token) => token.symbol)]).filter((symbol): symbol is string => !!symbol);
  const assets = [...new Set([...SWAP_ASSETS, ...available])];
  const chooseReverse = () => { setFrom(to); setTo(from); };

  return <div className="space-y-5 pb-4">
    <section className="space-y-3 rounded-2xl border border-border bg-card p-4">
      <div className="space-y-2 rounded-2xl bg-secondary/50 p-4"><div className="flex items-center justify-between gap-2"><Label htmlFor="swap-from">You pay</Label><span className="text-xs text-muted-foreground">From</span></div><div className="flex gap-3"><Input id="swap-from" inputMode="decimal" value={amount} onChange={(event) => setAmount(event.target.value.replace(/[^0-9.]/g, ""))} placeholder="0.00" className="h-12 border-0 bg-transparent px-0 text-2xl shadow-none focus-visible:ring-0" /><select aria-label="Asset to swap from" value={from} onChange={(event) => { const next = event.target.value; setFrom(next); if (next === to) setTo(from); }} className="h-10 max-w-28 rounded-xl border border-border bg-background px-3 text-sm font-semibold">{assets.map((asset) => <option key={asset} value={asset}>{asset}</option>)}</select></div></div>
      <div className="flex justify-center"><Button size="icon" variant="secondary" className="rounded-full" onClick={chooseReverse} aria-label="Reverse swap assets"><ArrowDownUp className="size-4" /></Button></div>
      <div className="space-y-2 rounded-2xl bg-secondary/50 p-4"><div className="flex items-center justify-between gap-2"><Label htmlFor="swap-to">You receive</Label><span className="text-xs text-muted-foreground">To</span></div><div className="flex gap-3"><div id="swap-to" className="flex h-12 min-w-0 flex-1 items-center text-2xl font-semibold text-muted-foreground">—</div><select aria-label="Asset to swap to" value={to} onChange={(event) => { const next = event.target.value; setTo(next); if (next === from) setFrom(to); }} className="h-10 max-w-28 rounded-xl border border-border bg-background px-3 text-sm font-semibold">{assets.map((asset) => <option key={asset} value={asset}>{asset}</option>)}</select></div></div>
      <div className="rounded-xl border border-amber-500/20 bg-amber-500/5 p-3"><div className="flex items-start gap-2"><ShieldAlert className="mt-0.5 size-4 shrink-0 text-amber-500" /><div><p className="text-sm font-medium">Live quotes are not connected</p><p className="mt-1 text-xs leading-relaxed text-muted-foreground">No exchange rate, route, or transaction will be fabricated. Swap becomes available after a quote provider and engine execution flow are integrated.</p></div></div></div>
      <Button className="w-full" size="lg" disabled>Get quote</Button>
    </section>
  </div>;
}
