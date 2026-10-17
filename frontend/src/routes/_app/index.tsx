import { useMemo } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowDownLeft, ArrowUpRight, ArrowLeftRight, Blocks, CheckCircle2, Eye, EyeOff, LoaderCircle, RefreshCw, ShieldCheck } from "lucide-react";
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
  const refreshing = useWallet((s) => s.refreshingBalances);
  const lastRefresh = useWallet((s) => s.lastBalanceRefresh);
  const refreshBalances = useWallet((s) => s.refreshBalances);
  const hide = useWallet((s) => s.hideBalances);
  const toggleHide = useWallet((s) => s.toggleHideBalances);
  const prices = usePrices((s) => s.prices);
  const chains = wallet?.chainAddresses ?? [];
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

  const displayItems = useMemo(() => {
    type DisplayItem = {
      name: string;
      symbol: string;
      balance: unknown;
      decimals?: number;
      address?: string;
      logo_url?: string;
      image_url?: string;
      icon_url?: string;
      logoUrl?: string;
      imageUrl?: string;
      iconUrl?: string;
      logo?: string;
      chainName: string;
      chainSymbol: string;
      isNative: boolean;
    };
    const entries = chains.map((chain, index) => {
      const chainId = String(chain.chain_id ?? "");
      const key = chainId || String(chain.name ?? index);
      const name = readableName(chain);
      const balance = balances.find((item) =>
        String(item.chain_id ?? "").trim().toLowerCase() === key.trim().toLowerCase());
      const native = balance?.native;
      const nativeMeta = native as (typeof native & { logo_url?: string; image_url?: string; icon_url?: string; logoUrl?: string; imageUrl?: string; iconUrl?: string; logo?: string }) | undefined;
      const chainSymbol = String(chain.symbol ?? native?.symbol ?? name.slice(0, 3)).slice(0, 8).toUpperCase();
      const tokens = (balance?.tokens ?? []).map((token) => {
        const tokenMeta = token as typeof token & { logo_url?: string; image_url?: string; icon_url?: string };
        return {
          name: String(token.name ?? token.symbol ?? "Unknown token"),
          symbol: String(token.symbol ?? ""),
          balance: token.balance,
          decimals: token.decimals,
          address: token.address,
          logo_url: tokenMeta.logo_url,
          image_url: tokenMeta.image_url,
          icon_url: tokenMeta.icon_url,
          logoUrl: tokenMeta.logoUrl,
          imageUrl: tokenMeta.imageUrl,
          iconUrl: tokenMeta.iconUrl,
          logo: tokenMeta.logo,
          chainName: name,
          chainSymbol,
          isNative: false,
        } satisfies DisplayItem;
      });
      const nativeItem: DisplayItem = {
        name: String(native?.name ?? name),
        symbol: String(native?.symbol ?? chain.symbol ?? "Native"),
        balance: native?.balance,
        decimals: native?.decimals,
        address: String(native?.address ?? chain.address ?? ""),
        logo_url: nativeMeta?.logo_url ?? (chain.logo_url as string | undefined),
        image_url: nativeMeta?.image_url ?? (chain.image_url as string | undefined),
        icon_url: nativeMeta?.icon_url ?? (chain.icon_url as string | undefined),
        logoUrl: nativeMeta?.logoUrl ?? (chain.logoUrl as string | undefined),
        imageUrl: nativeMeta?.imageUrl ?? (chain.imageUrl as string | undefined),
        iconUrl: nativeMeta?.iconUrl ?? (chain.iconUrl as string | undefined),
        logo: nativeMeta?.logo ?? (chain.logo as string | undefined),
        chainName: name,
        chainSymbol,
        isNative: true,
      };
      return { key, nativeItem, tokens };
    });
    const maxTokens = Math.max(0, ...entries.map((entry) => entry.tokens.length));
    const items: DisplayItem[] = [];
    // Keep the order stable while interleaving each network's tokens so one
    // chain's assets do not form a visually separate group.
    for (let slot = 0; slot <= maxTokens; slot += 1) {
      for (const entry of entries) {
        if (slot === 0) items.push(entry.nativeItem);
        else if (entry.tokens[slot - 1]) items.push(entry.tokens[slot - 1]!);
      }
    }
    return items;
  }, [chains, balances]);

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
    <section className="space-y-0">
      <div className="flex items-center justify-between gap-3 px-4 pb-2 pt-3"><div><h2 className="text-sm font-semibold">Assets & networks</h2></div><Link to="/blockchains" className="text-xs font-medium text-muted-foreground underline underline-offset-4">View all</Link></div>
      {!chains.length ? <div className="px-4 py-8 text-center"><div className="mx-auto grid size-9 place-items-center rounded-lg bg-secondary"><Blocks className="size-4" /></div><p className="mt-3 text-sm font-semibold">No networks connected</p><p className="mx-auto mt-1 max-w-xs text-xs text-muted-foreground">Create or restore a wallet and select the blockchains you want to use.</p></div> : <ProgressiveVirtualTokenList tokens={displayItems} hidden={hide} formatBalance={formatBalance} />}
    </section>
    </div>
  </div>;
}
