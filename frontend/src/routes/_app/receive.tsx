import { useMemo, useState } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowDownLeft, ArrowLeft, Check, ChevronRight, QrCode } from "lucide-react";
import { Button } from "@/components/ui/button";
import { AddressQr } from "@/components/wallet/qr-code";
import { CopyButton } from "@/components/wallet/copy-button";
import { useWallet } from "@/lib/wallet/store";

export const Route = createFileRoute("/_app/receive")({ component: ReceivePage });

function ReceivePage() {
  const wallet = useWallet((s) => s.wallet)!;
  const [selectedId, setSelectedId] = useState("");
  const chains = wallet.chainAddresses;
  const selected = chains.find((chain) => String(chain.chain_id ?? chain.name) === selectedId) ?? chains[0];
  const address = String(selected?.address ?? "");
  const chainName = String(selected?.name ?? selected?.chain_type ?? "Blockchain").replace(/[-_]/g, " ");
  const chainId = String(selected?.chain_id ?? "");
  const qrValue = useMemo(() => address, [address]);

  return <div className="space-y-5 pb-4">
    <header><Link to="/" className="inline-flex items-center gap-2 text-sm text-muted-foreground"><ArrowLeft className="size-4" /> Portfolio</Link><div className="mt-5 flex items-center gap-3"><div className="grid size-11 place-items-center rounded-2xl bg-secondary"><ArrowDownLeft className="size-5" /></div><div><p className="text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">Transfer</p><h1 className="text-2xl font-semibold tracking-tight">Receive crypto</h1></div></div><p className="mt-2 text-sm text-muted-foreground">Select a network to show its registered receiving address and QR code.</p></header>
    {!chains.length ? <section className="rounded-2xl border border-border bg-card px-4 py-12 text-center"><p className="font-semibold">No receiving addresses</p><p className="mt-2 text-sm text-muted-foreground">The engine has not returned any registered blockchain addresses for this wallet.</p></section> : <>
      <section className="space-y-2"><h2 className="px-1 text-sm font-semibold">Choose network</h2>{chains.map((chain, index) => {
        const id = String(chain.chain_id ?? chain.name ?? index);
        const isSelected = id === String(selected?.chain_id ?? selected?.name ?? "");
        return <button type="button" key={id} onClick={() => setSelectedId(id)} className={`flex w-full items-center gap-3 rounded-2xl border p-3 text-left transition-colors ${isSelected ? "border-foreground/30 bg-secondary/70" : "border-border bg-card hover:bg-secondary/40"}`}><div className="grid size-10 shrink-0 place-items-center rounded-xl bg-secondary text-xs font-bold">{String(chain.symbol ?? chain.name ?? "NET").slice(0, 3).toUpperCase()}</div><div className="min-w-0 flex-1"><p className="truncate text-sm font-semibold">{String(chain.name ?? chain.chain_type ?? id)}</p><p className="mt-1 text-xs text-muted-foreground">{String(chain.symbol ?? "Native asset")} {chain.chain_id ? `· Chain ID ${String(chain.chain_id)}` : ""}</p></div>{isSelected ? <Check className="size-4 shrink-0" /> : <ChevronRight className="size-4 shrink-0 text-muted-foreground" />}</button>;
      })}</section>
      {selected && <section className="space-y-4 rounded-2xl border border-border bg-card p-4">
        <div className="flex items-center gap-2"><QrCode className="size-4 text-muted-foreground" /><h2 className="text-sm font-semibold">{wallet.name} · {chainName}</h2></div>
        {address ? <div className="mx-auto max-w-[240px] rounded-2xl bg-white p-3"><AddressQr value={qrValue} className="w-full" /></div> : <div className="rounded-xl bg-secondary/60 px-4 py-8 text-center text-sm text-muted-foreground">No address was returned for this network.</div>}
        <div className="rounded-xl bg-secondary/50 p-3"><p className="text-xs text-muted-foreground">Receiving address</p><p className="mt-2 break-all font-mono text-sm leading-relaxed">{address || "Address unavailable"}</p>{address && <CopyButton value={address} label="Copy address" className="mt-3 w-full" />}</div>
        <div className="grid grid-cols-2 gap-3"><div className="rounded-xl bg-secondary/50 p-3"><p className="text-xs text-muted-foreground">Chain ID</p><p className="mt-1 break-all text-sm font-medium">{chainId || "Not returned"}</p></div><div className="rounded-xl bg-secondary/50 p-3"><p className="text-xs text-muted-foreground">Derivation path</p><p className="mt-1 break-all font-mono text-xs">{String(selected.derivation_path ?? "Not returned")}</p></div></div>
      </section>}
    </>}
    <p className="text-center text-xs leading-relaxed text-muted-foreground">Only use an address on its matching blockchain and network. This testing build is not intended for real funds.</p>
    <Button variant="secondary" className="w-full" asChild><Link to="/blockchains">View all blockchain addresses</Link></Button>
  </div>;
}
