import { useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { ArrowUpRight, ShieldAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useWallet } from "@/lib/wallet/store";

export const Route = createFileRoute("/_app/send")({ component: SendPage });

function SendPage() {
  const wallet = useWallet((s) => s.wallet);
  const balances = useWallet((s) => s.balances);
  const [networkId, setNetworkId] = useState("");
  const [recipient, setRecipient] = useState("");
  const [amount, setAmount] = useState("");
  const chains = wallet?.chainAddresses ?? [];
  const selected = chains.find((chain) => String(chain.chain_id ?? chain.name) === networkId) ?? chains[0];
  const balance = balances.find((item) => String(item.chain_id ?? "") === String(selected?.chain_id ?? ""));
  const symbol = String(selected?.symbol ?? balance?.native?.symbol ?? "Native asset");

  return <div className="space-y-5 pb-4">
    <section className="space-y-4 rounded-2xl border border-border bg-card p-4">
      <div className="space-y-2"><Label htmlFor="send-network">Network</Label><select id="send-network" value={networkId || String(selected?.chain_id ?? selected?.name ?? "")} onChange={(event) => setNetworkId(event.target.value)} className="h-12 w-full rounded-xl border border-border bg-background px-3 text-sm outline-none focus:ring-2 focus:ring-ring">{chains.map((chain, index) => <option key={String(chain.chain_id ?? index)} value={String(chain.chain_id ?? chain.name ?? "")}>{String(chain.name ?? chain.chain_type ?? "Blockchain")} · {String(chain.symbol ?? "Native")}</option>)}</select></div>
      <div className="space-y-2"><Label htmlFor="send-recipient">Recipient address</Label><Input id="send-recipient" value={recipient} onChange={(event) => setRecipient(event.target.value)} placeholder="Paste wallet address" autoComplete="off" /><p className="text-xs text-muted-foreground">Only send to an address compatible with the selected network.</p></div>
      <div className="space-y-2"><div className="flex items-center justify-between gap-2"><Label htmlFor="send-amount">Amount</Label><span className="text-xs text-muted-foreground">Available: {balance?.native?.balance ?? "—"} {symbol}</span></div><div className="flex gap-2"><Input id="send-amount" inputMode="decimal" value={amount} onChange={(event) => setAmount(event.target.value.replace(/[^0-9.]/g, ""))} placeholder="0.00" /><div className="flex h-10 shrink-0 items-center rounded-md bg-secondary px-3 text-sm font-semibold">{symbol}</div></div></div>
      <div className="rounded-xl border border-amber-500/20 bg-amber-500/5 p-3"><div className="flex items-start gap-2"><ShieldAlert className="mt-0.5 size-4 shrink-0 text-amber-500" /><div><p className="text-sm font-medium">Sending is not enabled yet</p><p className="mt-1 text-xs leading-relaxed text-muted-foreground">The engine's transaction signing and broadcast flow has not been connected. This form will not submit or move funds.</p></div></div></div>
      <Button className="w-full" size="lg" disabled>Send {symbol}</Button>
    </section>
    <p className="text-center text-xs leading-relaxed text-muted-foreground">Testing wallet · Never enter a real transfer expecting it to be broadcast.</p>
  </div>;
}
