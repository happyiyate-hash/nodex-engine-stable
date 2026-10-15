import { useMemo, useState } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowDownLeft, ArrowUpRight, ArrowLeftRight, Blocks, CheckCircle2, ChevronDown, ChevronUp, CircleAlert, Eye, EyeOff, LoaderCircle, RefreshCw, ShieldCheck } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { ProgressiveVirtualTokenList } from "@/components/wallet/progressive-virtual-token-list";
import { useWallet } from "@/lib/wallet/store";
import { usePrices } from "@/lib/wallet/prices";
import { formatUsd } from "@/lib/utils";
import type { AssetId } from "@/lib/wallet/assets";

export const Route = createFileRoute("/_app/")({ component: HomePage });
const PRICE_ID: Record<string, AssetId> = { ETH: "ethereum", WETH: "ethereum", BTC: "bitcoin", WBTC: "bitcoin", USDC: "usd-coin", SOL: "solana", LINK: "chainlink", UNI: "uniswap" };
function readableName(chain: Record<string, unknown>) { return String(chain.name ?? chain.chain_name ?? chain.chain_type ?? "Blockchain").replace(/[-_]/g, " ").replace(/\b\w/g, (letter) => letter.toUpperCase()); }
function shortAddress(address: string) { return address.length > 26 ? `${address.slice(0, 12)}…${address.slice(-8)}` : address || "Address unavailable"; }
function amountOf(value: unknown, decimals?: number) {
  const raw = String(value ?? "0").trim().replace(/,/g, "");
  if (!raw) return 0;
  const parsed = Number(raw);
  if (!Number.isFinite(parsed) || parsed <= 0) return 0;
  // Engine balances are raw base units; use the asset's own decimals rather
  // than assuming every token has 18 decimals.
  return Number.isInteger(decimals) && Number(decimals) >= 0 && Number(decimals) <= 255
    ? parsed / 10 ** Number(decimals)
    : parsed;
}
function formatBalance(value: unknown, assetDecimals?: number) {
  const raw = String(value ?? "").trim();
  if (!raw) return "—";
  if (/^-?\\d+$/.test(raw) && Number.isInteger(assetDecimals) && Number(assetDecimals) >= 0 && Number(assetDecimals) <= 255) {
    try {
      const negative = raw.startsWith("-");
      const integer = BigInt(negative ? raw.slice(1) : raw);
      const scale = 10n ** BigInt(Number(assetDecimals));
      const whole = integer / scale;
      const fraction = (integer % scale).toString().padStart(Number(assetDecimals), "0").slice(0, 8).replace(/0+$/, "");
      const grouped = whole.toString().replace(/\\B(?=(\\d{3})+(?!\\d))/g, ",");
      return `${negative ? "-" : ""}${grouped}${fraction ? `.${fraction}` : ""}`;
    } catch { /* Fall through to the best-effort legacy formatter. */ }
  }
  const parsed = Number(raw.replace(/,/g, ""));
  if (!Number.isFinite(parsed)) return raw;
  if (parsed === 0) return "0";
  const places = Math.abs(parsed) >= 0.0001 ? 4 : Math.min(14, Math.max(4, -Math.floor(Math.log10(Math.abs(parsed))) + 4));
  return parsed.toLocaleString("en-US", { maximumFractionDigits: places, minimumFractionDigits: 0, useGrouping: true });
}
function HomePage() {
  const wallet = useWallet((s) => s.wallet);
  const balances = useWallet((s) => s.balances);
  const chainFetchState = useWallet((s) => s.chainFetchState);
  const refreshing = useWallet((s) => s.refreshingBalances);
  const lastRefresh = useWallet((s) => s.lastBalanceRefresh);
  const lastError = useWallet((s) => s.lastEngineError);
  const refreshBalances = useWallet((s) => s.refreshBalances);
  const hide = useWallet((s) => s.hideBalances);
  const toggleHide = useWallet((s) => s.toggleHideBalances);
  const prices = usePrices((s) => s.prices);
  const [expandedErrors, setExpandedErrors] = useState<Record<string, boolean>>({});
  const chains = wallet?.chainAddresses ?? [];
  const derivationErrors = wallet?.derivationErrors ?? [];
  const successCount = chains.filter((chain) => chainFetchState[String(chain.chain_id ?? chain.name ?? "")]?.status === "success").length;
  const portfolio = useMemo(() => {
    let value = 0;
    let pricedAssets = 0;
    for (const balance of balances) {
      const nativeAsset = PRICE_ID[String(balance.native?.symbol ?? "").toUpperCase()];
      if (nativeAsset) { value += amountOf(balance.native?.balance, balance.native?.decimals) * (prices[nativeAsset]?.usd ?? 0); pricedAssets += 1; }
      for (const token of balance.tokens ?? []) {
        const tokenAsset = PRICE_ID[String(token.symbol ?? "").toUpperCase()];
        if (tokenAsset) { value += amountOf(token.balance, token.decimals) * (prices[tokenAsset]?.usd ?? 0); pricedAssets += 1; }
      }
    }
    return { value, pricedAssets };
  }, [balances, prices]);

  async function refresh() {
    try { await refreshBalances(); toast("Balance refresh finished. Check each network's status."); }
    catch (error) { toast(error instanceof Error ? error.message : "Engine balance request failed. Open request logs for details."); }
  }

  return <div className="flex h-dvh flex-col overflow-hidden pb-0">
    <section className="relative flex shrink-0 flex-col overflow-hidden rounded-b-[1.5rem] bg-secondary/65 px-5 pb-3 pt-[max(0.85rem,env(safe-area-inset-top))] text-white shadow-[0_8px_28px_rgba(0,0,0,0.28)] backdrop-blur-2xl after:pointer-events-none after:absolute after:inset-x-0 after:bottom-0 after:h-px after:bg-gradient-to-r after:from-transparent after:via-white/40 after:to-transparent after:shadow-[0_0_4px_rgba(255,255,255,0.14)]">
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2.5">
          <div className="grid size-9 shrink-0 place-items-center rounded-xl border border-white/10 bg-white/5"><ShieldCheck className="size-[18px]" /></div>
          <div className="min-w-0"><p className="truncate text-xs text-white/65">{wallet?.name ?? "My wallet"}</p><h1 className="text-base font-semibold tracking-tight">Wallet overview</h1></div>
        </div>
        <Button variant="secondary" size="sm" disabled={refreshing || !wallet} onClick={refresh} className="h-9 shrink-0 rounded-xl border border-white/10 bg-white/5 text-white hover:bg-white/10">{refreshing ? <LoaderCircle className="size-4 animate-spin" /> : <RefreshCw className="size-4" />}{refreshing ? "Updating" : "Refresh"}</Button>
      </div>
      <div className="mt-3">
        <div className="flex items-center gap-2"><p className="text-xs font-medium text-white/65">Total estimated balance</p><button className="rounded-md p-1 text-white/75 transition hover:bg-white/10 hover:text-white" onClick={toggleHide} aria-label={hide ? "Show balances" : "Hide balances"}>{hide ? <EyeOff className="size-4" /> : <Eye className="size-4" />}</button></div>
        <p className="mt-1 text-[clamp(2rem,8vw,2.65rem)] font-semibold leading-none tracking-tight tabular-nums">{hide ? "••••••••" : portfolio.pricedAssets ? formatUsd(portfolio.value) : "—"}</p>
        <p className="mt-2 text-[11px] text-white/60">{!balances.length ? "Refresh to load on-chain balances" : portfolio.pricedAssets ? "Estimated value · supported assets only" : "Balances received · price estimate unavailable"}</p>
        <div className="mt-3 grid grid-cols-3 gap-2">
          <Button variant="secondary" className="relative h-[54px] flex-col gap-1 rounded-full border border-white/[0.08] bg-[#000000] text-white shadow-[0_0_0_0.5px_rgba(255,255,255,0.035),0_3px_8px_rgba(0,0,0,0.34)] transition-all hover:border-white/[0.12] hover:bg-[#030303] active:translate-y-px active:shadow-[inset_0_1px_0_rgba(255,255,255,0.035)]" asChild><Link to="/send"><ArrowUpRight className="size-4" /><span className="text-[11px]">Send</span></Link></Button>
          <Button variant="secondary" className="relative h-[54px] flex-col gap-1 rounded-full border border-white/[0.08] bg-[#000000] text-white shadow-[0_0_0_0.5px_rgba(255,255,255,0.035),0_3px_8px_rgba(0,0,0,0.34)] transition-all hover:border-white/[0.12] hover:bg-[#030303] active:translate-y-px active:shadow-[inset_0_1px_0_rgba(255,255,255,0.035)]" asChild><Link to="/receive"><ArrowDownLeft className="size-4" /><span className="text-[11px]">Receive</span></Link></Button>
          <Button variant="secondary" className="relative h-[54px] flex-col gap-1 rounded-full border border-white/[0.08] bg-[#000000] text-white shadow-[0_0_0_0.5px_rgba(255,255,255,0.035),0_3px_8px_rgba(0,0,0,0.34)] transition-all hover:border-white/[0.12] hover:bg-[#030303] active:translate-y-px active:shadow-[inset_0_1px_0_rgba(255,255,255,0.035)]" asChild><Link to="/swap"><ArrowLeftRight className="size-4" /><span className="text-[11px]">Swap</span></Link></Button>
        </div>
      </div>
    </section>

    <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain pb-28 pt-0">
    {lastError && <details className="group border-b border-destructive/20 px-1 py-2"><summary className="flex cursor-pointer list-none items-start gap-2"><CircleAlert className="mt-0.5 size-4 shrink-0 text-destructive" /><span className="min-w-0 flex-1"><span className="block text-xs font-semibold">Latest engine warning</span><span className="mt-1 block truncate text-[11px] text-muted-foreground">{lastError}</span></span><ChevronDown className="mt-0.5 size-4 shrink-0 text-muted-foreground transition group-open:rotate-180" /></summary><p className="mt-3 break-words border-t border-border pt-3 text-xs leading-relaxed text-destructive">{lastError}</p><Link to="/engine-logs" className="mt-2 inline-block text-xs font-medium underline underline-offset-4">Open request and response logs</Link></details>}

    {derivationErrors.length > 0 && <section className="space-y-2 border-b border-destructive/20 px-1 py-2"><div className="flex items-center gap-2"><CircleAlert className="size-4 text-destructive" /><h2 className="text-xs font-semibold">Address issues <span className="text-muted-foreground">({derivationErrors.length})</span></h2></div>{derivationErrors.map((failure, index) => <details key={String(failure.chain_id ?? failure.chain ?? index)} className="rounded-lg border border-border px-3 py-2"><summary className="cursor-pointer text-xs font-medium">{String(failure.chain ?? failure.chain_name ?? failure.name ?? "Unknown blockchain")} <span className="ml-1 font-normal text-destructive">· Error details</span></summary><p className="mt-2 break-words text-[11px] leading-relaxed text-muted-foreground">{String(failure.error ?? failure.message ?? "No error details were returned.")}</p></details>)}<Link to="/engine-logs" className="inline-block text-xs underline underline-offset-4">Full engine logs</Link></section>}

    <section className="space-y-0">
      <div className="flex items-center justify-between gap-3 px-4 pb-2 pt-3"><div><h2 className="text-sm font-semibold">Your networks</h2></div><Link to="/blockchains" className="text-xs font-medium text-muted-foreground underline underline-offset-4">View all</Link></div>
      {!chains.length ? <div className="rounded-xl border border-border bg-card px-4 py-9 text-center"><div className="mx-auto grid size-10 place-items-center rounded-xl bg-secondary"><Blocks className="size-5" /></div><p className="mt-3 text-sm font-semibold">No networks connected</p><p className="mx-auto mt-1 max-w-xs text-xs text-muted-foreground">Create or restore a wallet and select the blockchains you want to use.</p></div> : <div>{chains.map((chain, index) => {
        const chainId = String(chain.chain_id ?? "");
        const key = chainId || String(chain.name ?? index);
        const name = readableName(chain);
        const balance = balances.find((item) =>
          String(item.chain_id ?? "").trim().toLowerCase() === key.trim().toLowerCase());
        const native = balance?.native;
        const state = chainFetchState[key] ?? { status: "idle" as const };
        const expanded = Boolean(expandedErrors[key]);
        return <article key={key} className="px-4 py-2.5">
          <div className="flex items-center gap-2.5"><div className="grid size-9 shrink-0 place-items-center rounded-full bg-secondary text-[10px] font-semibold">{String(chain.symbol ?? native?.symbol ?? name.slice(0, 3)).slice(0, 3).toUpperCase()}</div><div className="min-w-0 flex-1"><h3 className="truncate text-xs font-semibold">{name}</h3><p className="mt-0.5 truncate text-[10px] text-muted-foreground">{String(chain.symbol ?? native?.symbol ?? "Native asset")}</p></div><div className="min-w-0 text-right"><p className="max-w-[44vw] truncate text-sm font-semibold tabular-nums">{hide ? "••••••" : formatBalance(native?.balance, native?.decimals)}</p><p className="mt-0.5 text-[10px] text-muted-foreground">{native?.symbol ?? "Native balance"}</p></div><div className="grid size-6 shrink-0 place-items-center">{state.status === "loading" ? <LoaderCircle className="size-3.5 animate-spin text-muted-foreground" /> : state.status === "error" ? <CircleAlert className="size-4 text-destructive" /> : state.status === "success" ? <CheckCircle2 className="size-4 text-emerald-500" /> : <span className="size-1.5 rounded-full bg-muted-foreground/40" />}</div></div>
          {(balance?.tokens?.length ?? 0) > 0 && <div className="ml-[2.875rem] mt-2 space-y-1.5">
            <p className="text-[10px] font-medium uppercase tracking-wider text-muted-foreground">Tokens · {balance!.tokens!.length}</p>
            <ProgressiveVirtualTokenList tokens={balance!.tokens!} hidden={hide} formatBalance={formatBalance} />
          </div>}
          {state.status === "error" && <div className="mt-2 border-l-2 border-destructive/40 pl-2"><button onClick={() => setExpandedErrors((s) => ({ ...s, [key]: !s[key] }))} className="flex w-full items-center gap-1.5 text-left text-[10px] font-medium text-destructive"><CircleAlert className="size-3" /><span className="flex-1 truncate">{state.message ?? "Balance request failed"}</span>{expanded ? <ChevronUp className="size-3.5" /> : <ChevronDown className="size-3.5" />}</button>{expanded && <div className="mt-2 px-1 py-1"><p className="whitespace-pre-wrap break-words font-mono text-[10px] leading-relaxed text-muted-foreground">{state.message ?? "No detailed error response was returned."}</p><Link to="/engine-logs" className="mt-2 inline-block text-[10px] font-medium underline underline-offset-4">Open full logs</Link></div>}</div>}
        </article>;
      })}</div>}
    </section>
    </div>
  </div>;
}
