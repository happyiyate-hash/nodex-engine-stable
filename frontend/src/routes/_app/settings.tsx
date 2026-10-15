import { useState } from "react";
import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";
import { ArrowRight, BookOpen, Blocks, LockKeyhole, RadioTower, ShieldCheck, Wallet, Wrench } from "lucide-react";
import { AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle, AlertDialogTrigger } from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { CopyButton } from "@/components/wallet/copy-button";
import { useWallet } from "@/lib/wallet/store";
import { truncateAddress } from "@/lib/utils";

export const Route = createFileRoute("/_app/settings")({ component: SettingsPage });

function SettingsPage() {
  const wallet = useWallet((s) => s.wallet)!;
  const hide = useWallet((s) => s.hideBalances);
  const toggleHide = useWallet((s) => s.toggleHideBalances);
  const rename = useWallet((s) => s.rename);
  const lock = useWallet((s) => s.lock);
  const reset = useWallet((s) => s.reset);
  const setPin = useWallet((s) => s.setPin);
  const navigate = useNavigate();
  const [name, setName] = useState(wallet.name);
  const [pin, setPinValue] = useState("");
  const [showPhrase, setShowPhrase] = useState(false);

  return (
    <div className="space-y-6 pb-4">
      <header>
        <p className="text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">Your wallet</p>
        <h1 className="mt-1 text-2xl font-semibold tracking-tight">Settings</h1>
        <p className="mt-1 text-sm text-muted-foreground">Manage your wallet, privacy, and connections.</p>
      </header>

      <section className="rounded-2xl border border-border bg-card p-4">
        <div className="flex items-center gap-3">
          <div className="grid size-11 place-items-center rounded-2xl bg-secondary"><Wallet className="size-5" /></div>
          <div className="min-w-0 flex-1"><h2 className="font-semibold">{wallet.name}</h2><p className="text-xs text-muted-foreground">{wallet.chainAddresses.length} network{wallet.chainAddresses.length === 1 ? "" : "s"} connected</p></div>
          <ShieldCheck className="size-5 text-muted-foreground" />
        </div>
        <div className="mt-4 space-y-2">
          <Label htmlFor="wallet-name">Wallet name</Label>
          <div className="flex gap-2">
            <Input id="wallet-name" value={name} onChange={(event) => setName(event.target.value)} />
            <Button onClick={() => { rename(name); toast("Wallet name saved"); }}>Save</Button>
          </div>
        </div>
        <div className="mt-4 grid gap-3 rounded-xl bg-secondary/50 p-3">
          <div><p className="text-xs text-muted-foreground">Wallet ID</p><p className="mt-1 break-all font-mono text-xs">{wallet.engineWalletId}</p></div>
          <div><p className="text-xs text-muted-foreground">Wallet account</p><p className="mt-1 break-all font-mono text-xs">{wallet.childId}</p></div>
        </div>
      </section>

      <section className="overflow-hidden rounded-2xl border border-border bg-card">
        <div className="border-b border-border px-4 py-3"><h2 className="font-semibold">Wallet tools</h2><p className="mt-1 text-xs text-muted-foreground">Each tool has its own page.</p></div>
        <ToolLink to="/blockchains" icon={Blocks} title="Blockchains and addresses" description="View networks, balances, and receiving addresses" />
        <ToolLink to="/connection" icon={RadioTower} title="Engine connection" description="Configure and check live event streaming" />
        <ToolLink to="/engine-logs" icon={BookOpen} title="Request and response logs" description="Inspect redacted engine requests, responses, and live events" />
      </section>

      <section className="space-y-4 rounded-2xl border border-border bg-card p-4">
        <div><h2 className="font-semibold">Privacy</h2><p className="mt-1 text-sm text-muted-foreground">Hide balances across the wallet.</p></div>
        <div className="flex items-center justify-between gap-4"><div><p className="text-sm font-medium">Hide balances</p><p className="text-xs text-muted-foreground">Mask portfolio and network balances.</p></div><Switch checked={hide} onCheckedChange={() => toggleHide()} /></div>
      </section>

      <section className="space-y-4 rounded-2xl border border-border bg-card p-4">
        <div className="flex items-center gap-3"><div className="grid size-10 place-items-center rounded-xl bg-secondary"><LockKeyhole className="size-5" /></div><div><h2 className="font-semibold">Security</h2><p className="text-xs text-muted-foreground">Local wallet lock and recovery phrase</p></div></div>
        {wallet.pinHash ? <Button variant="secondary" className="w-full" onClick={() => { lock(); toast("Wallet locked"); }}>Lock wallet</Button> : <div className="space-y-2"><Label htmlFor="new-pin">Set a 6-digit wallet PIN</Label><div className="flex gap-2"><Input id="new-pin" inputMode="numeric" maxLength={6} value={pin} onChange={(event) => setPinValue(event.target.value.replace(/\D/g, "").slice(0, 6))} className="font-mono tracking-[0.3em]" /><Button disabled={pin.length !== 6} onClick={async () => { await setPin(pin); setPinValue(""); toast("Wallet PIN set"); }}>Set PIN</Button></div></div>}
        <div className="border-t border-border pt-4">
          <p className="text-sm font-medium">Recovery phrase</p>
          <p className="mt-1 text-xs leading-relaxed text-muted-foreground">Anyone with these words can control this wallet. This is a testing build; never use it with real funds or share your phrase.</p>
          <p className="mt-3 rounded-xl bg-secondary/60 px-3 py-3 font-mono text-sm">{showPhrase ? wallet.phrase : truncateAddress(wallet.phrase.replace(/\s/g, ""), 8)}</p>
          <div className="mt-2 flex gap-2"><Button variant="secondary" onClick={() => setShowPhrase((value) => !value)}>{showPhrase ? "Hide phrase" : "Reveal phrase"}</Button>{showPhrase && <CopyButton value={wallet.phrase} label="Copy phrase" />}</div>
        </div>
      </section>

      <section className="space-y-3 rounded-2xl border border-destructive/20 bg-card p-4">
        <div><h2 className="font-semibold">Remove wallet</h2><p className="mt-1 text-sm text-muted-foreground">Erase this wallet and its locally saved balances from this browser.</p></div>
        <AlertDialog>
          <AlertDialogTrigger asChild><Button variant="outline" className="w-full text-destructive">Remove wallet</Button></AlertDialogTrigger>
          <AlertDialogContent><AlertDialogHeader><AlertDialogTitle>Remove this wallet?</AlertDialogTitle><AlertDialogDescription>This cannot be undone unless you saved your recovery phrase securely.</AlertDialogDescription></AlertDialogHeader><AlertDialogFooter><AlertDialogCancel>Cancel</AlertDialogCancel><AlertDialogAction onClick={() => { reset(); void navigate({ to: "/" }); }}>Remove wallet</AlertDialogAction></AlertDialogFooter></AlertDialogContent>
        </AlertDialog>
      </section>

      <p className="flex items-start gap-2 px-1 text-xs leading-relaxed text-muted-foreground"><Wrench className="mt-0.5 size-4 shrink-0" /> NodeX is still a testing integration. Sending and swaps stay unavailable until their engine-backed transaction flows are implemented.</p>
    </div>
  );
}

function ToolLink({ to, icon: Icon, title, description }: { to: "/blockchains" | "/connection" | "/engine-logs"; icon: typeof Blocks; title: string; description: string }) {
  return <Link to={to} className="flex items-center gap-3 border-b border-border px-4 py-4 last:border-b-0 transition-colors hover:bg-secondary/40">
    <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-secondary"><Icon className="size-5" /></div>
    <div className="min-w-0 flex-1"><p className="text-sm font-medium">{title}</p><p className="mt-1 text-xs leading-relaxed text-muted-foreground">{description}</p></div><ArrowRight className="size-4 shrink-0 text-muted-foreground" />
  </Link>;
}
