import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowDownLeft, ArrowLeftRight, ArrowUpRight, Clock3, FileSearch } from "lucide-react";
import { TokenIcon } from "@/components/wallet/token-icon";
import { ASSETS } from "@/lib/wallet/assets";
import { useWallet, type Transaction } from "@/lib/wallet/store";
import { groupTxs } from "@/lib/wallet/select";
import { formatAmount, formatDateHeading, formatRelative, truncateAddress } from "@/lib/utils";

export const Route = createFileRoute("/_app/activity")({ component: ActivityPage });

function ActivityPage() {
  const txs = useWallet((s) => s.txs);
  const groups = groupTxs(txs);
  return <div className="space-y-5 pb-4">
    {txs.length === 0 ? <section className="rounded-2xl border border-border bg-card px-5 py-12 text-center"><div className="mx-auto grid size-12 place-items-center rounded-2xl bg-secondary"><Clock3 className="size-5" /></div><h2 className="mt-4 font-semibold">No transactions yet</h2><p className="mx-auto mt-2 max-w-xs text-sm leading-relaxed text-muted-foreground">Confirmed transfers and swaps will appear here once engine-backed transaction flows are available. Engine diagnostics are kept separately.</p><div className="mt-5 flex flex-col gap-2 sm:flex-row sm:justify-center"><Link to="/receive" className="inline-flex h-10 items-center justify-center rounded-xl bg-secondary px-4 text-sm font-medium">Receive crypto</Link><Link to="/engine-logs" className="inline-flex h-10 items-center justify-center gap-2 rounded-xl border border-border px-4 text-sm font-medium"><FileSearch className="size-4" /> Request logs</Link></div></section> : <div className="space-y-6">{groups.map((group) => <section key={group.key}><h2 className="mb-2 px-1 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">{formatDateHeading(group.items[0]!.timestamp)}</h2><div className="divide-y divide-border overflow-hidden rounded-2xl border border-border bg-card">{group.items.map((tx) => <TxRow key={tx.id} tx={tx} />)}</div></section>)}</div>}
  </div>;
}

function TxRow({ tx }: { tx: Transaction }) {
  const asset = ASSETS[tx.assetId];
  const toAsset = tx.toAssetId ? ASSETS[tx.toAssetId] : null;
  const Icon = tx.type === "send" ? ArrowUpRight : tx.type === "receive" ? ArrowDownLeft : ArrowLeftRight;
  const title = tx.type === "swap" ? `Swap ${asset.symbol} → ${toAsset?.symbol ?? ""}` : tx.type === "send" ? `Sent ${asset.symbol}` : `Received ${asset.symbol}`;
  const amount = tx.type === "swap" && tx.toAmount ? `+${formatAmount(tx.toAmount, toAsset?.symbol ?? "")}` : `${tx.type === "send" ? "−" : "+"}${formatAmount(tx.amount, asset.symbol)}`;
  return <article className="flex items-center gap-3 px-4 py-4"><div className="relative"><TokenIcon id={tx.toAssetId ?? tx.assetId} /><span className="absolute -bottom-0.5 -right-0.5 grid size-4 place-items-center rounded-full bg-background"><Icon className="size-2.5" /></span></div><div className="min-w-0 flex-1"><div className="flex items-baseline justify-between gap-2"><p className="truncate text-sm font-semibold">{title}</p><p className="shrink-0 tabular-nums text-sm font-semibold">{amount}</p></div><div className="mt-1 flex items-center justify-between gap-2 text-xs text-muted-foreground"><p className="truncate font-mono">{tx.counterparty ? truncateAddress(tx.counterparty, 4) : truncateAddress(tx.hash, 4)}</p><p className="shrink-0">{formatRelative(tx.timestamp)}</p></div></div></article>;
}
