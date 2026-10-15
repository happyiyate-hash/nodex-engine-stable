import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { ArrowLeft, ArrowRight, Eye, EyeOff, Shield, LoaderCircle, KeyRound, KeySquare, Check } from "lucide-react";
import { toast } from "sonner";
import { SableMark } from "@/components/brand/sable-mark";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { normalizeMnemonic } from "@/lib/wallet/mnemonic";
import { fetchBlockchainCatalog, type SelectedChain } from "@/lib/wallet/engine";
import { useWallet } from "@/lib/wallet/store";

type Step = "welcome" | "create" | "restore-method" | "phrase" | "phrase-chains" | "key-chains" | "key-secret" | "pin" | "backup" | "complete";
type Mode = "create" | "phrase" | "private-key";

export function Onboarding() {
  const navigate = useNavigate();
  const createWallet = useWallet((s) => s.createWallet);
  const restoreWallet = useWallet((s) => s.restoreWallet);
  const importPrivateKey = useWallet((s) => s.importPrivateKey);
  const setPin = useWallet((s) => s.setPin);
  const completeSetup = useWallet((s) => s.completeSetup);
  const wallet = useWallet((s) => s.wallet);
  const [step, setStep] = useState<Step>("welcome");
  const [mode, setMode] = useState<Mode>("create");
  const [name, setName] = useState("Primary");
  const [restorePhrase, setRestorePhrase] = useState("");
  const [privateKey, setPrivateKey] = useState("");
  const [showSecret, setShowSecret] = useState(false);
  const [busy, setBusy] = useState(false);
  const [pin, setPinValue] = useState("");
  const [pin2, setPin2] = useState("");
  const [allPhraseChains, setAllPhraseChains] = useState(true);
  const [allEvmChains, setAllEvmChains] = useState(false);
  const [chainCatalog, setChainCatalog] = useState<SelectedChain[]>([]);
  const [catalogLoading, setCatalogLoading] = useState(true);
  const [catalogError, setCatalogError] = useState<string | null>(null);
  const [selectedPhrase, setSelectedPhrase] = useState<string[]>([]);
  const [selectedKey, setSelectedKey] = useState<string[]>([]);

  useEffect(() => {
    let cancelled = false;
    fetchBlockchainCatalog().then((chains) => {
      if (cancelled) return;
      setChainCatalog(chains);
      setCatalogError(chains.length ? null : "NodeX returned no supported mainnet blockchains.");
    }).catch((error) => {
      if (!cancelled) setCatalogError(error instanceof Error ? error.message : "Could not load supported blockchains from NodeX.");
    }).finally(() => { if (!cancelled) setCatalogLoading(false); });
    return () => { cancelled = true; };
  }, []);

  const evmChains = useMemo(() => chainCatalog.filter((chain) => chain.chain_type.toLowerCase() === "evm"), [chainCatalog]);
  const nonEvmChains = useMemo(() => chainCatalog.filter((chain) => chain.chain_type.toLowerCase() !== "evm"), [chainCatalog]);
  const phraseChains = allPhraseChains ? chainCatalog : chainCatalog.filter((chain) => selectedPhrase.includes(chain.chain_id));
  const keyChains = allEvmChains
    ? evmChains
    : chainCatalog.filter((chain) => selectedKey.includes(chain.chain_id));
  const selectedChains = mode === "create" || mode === "phrase" ? phraseChains : keyChains;

  function toggleChain(id: string, checked: boolean, forKey = false) {
    if (forKey) {
      setSelectedKey((previous) => checked ? [...new Set([...previous, id])] : previous.filter((value) => value !== id));
      setAllEvmChains(false);
    } else {
      setSelectedPhrase((previous) => checked ? [...new Set([...previous, id])] : previous.filter((value) => value !== id));
      setAllPhraseChains(false);
    }
  }

  async function onPin() {
    if (pin.length !== 6 || pin !== pin2) { toast.error("Enter matching 6-digit PINs."); return; }
    if (catalogLoading || catalogError || !selectedChains.length) { toast.error("Select at least one supported blockchain."); return; }
    if (mode === "private-key" && selectedChains.some((chain) => chain.chain_type.toLowerCase() !== "evm") && selectedChains.length > 1) {
      toast.error("Choose one non-EVM blockchain, or select EVM networks only."); return;
    }
    setBusy(true);
    try {
      let created: Awaited<ReturnType<typeof createWallet>>;
      if (mode === "create") created = await createWallet(name, pin, selectedChains);
      else if (mode === "phrase") created = await restoreWallet(restorePhrase, name, pin, selectedChains);
      else created = await importPrivateKey(privateKey, name, pin, selectedChains);
      await setPin(pin);
      if (mode === "private-key") { setStep("complete"); return; }
      setStep("backup");
      if (created.phrase) setRestorePhrase("");
    } catch (error) {
      const message = error instanceof Error ? error.message : "NodeX could not import this wallet.";
      toast.error(message, { action: { label: "View logs", onClick: () => { void navigate({ to: "/engine-logs" }); } } });
    } finally { setBusy(false); }
  }

  function continueFromPhrase() {
    const normalized = normalizeMnemonic(restorePhrase);
    if (![12, 15, 18, 21, 24].includes(normalized.split(" ").length)) {
      toast.error("Enter a valid 12, 15, 18, 21, or 24-word recovery phrase.");
      return;
    }
    setMode("phrase");
    setAllPhraseChains(true);
    setSelectedPhrase([]);
    setStep("phrase-chains");
  }

  function continueFromKeyChains() {
    if (!selectedChains.length) { toast.error("Choose at least one blockchain."); return; }
    if (!allEvmChains && selectedChains.some((chain) => chain.chain_type.toLowerCase() !== "evm") && selectedChains.length > 1) {
      toast.error("Non-EVM private-key imports must use one blockchain at a time."); return;
    }
    setPrivateKey("");
    setStep("key-secret");
  }

  function continueFromPrivateKey() {
    // Trim harmless whitespace; preserve Base58 case and characters for Solana.
    const raw = privateKey.trim().replace(/\s+/g, "");
    if (!raw) { toast.error("Paste your private key."); return; }

    const solanaSelected = selectedChains.length === 1
      && selectedChains[0]?.chain_type.toLowerCase() === "solana";

    if (solanaSelected) {
      const hexKey = raw.replace(/^0x/i, "");
      const looksHex = /^[0-9a-f]+$/i.test(hexKey) && (hexKey.length === 64 || hexKey.length === 128);
      const looksBase58 = /^[1-9A-HJ-NP-Za-km-z]+$/.test(raw);
      if (!looksHex && !looksBase58) {
        toast.error("Enter a Solana Base58 secret key or a 64/128-character hexadecimal key.");
        return;
      }
      setPrivateKey(looksHex ? hexKey : raw);
      setStep("pin");
      return;
    }

    const normalized = raw.replace(/^0x/i, "");
    if (!/^[0-9a-f]+$/i.test(normalized)) {
      toast.error("Private key must contain hexadecimal characters only (0–9 and A–F).");
      return;
    }
    if (normalized.length % 2 !== 0) {
      toast.error("This private key has an odd number of hex digits. Copy it again; do not add or remove digits manually.");
      return;
    }
    if (selectedChains.every((chain) => chain.chain_type.toLowerCase() === "evm") && normalized.length !== 64) {
      toast.error("For EVM networks such as Polygon, the private key must contain exactly 64 hexadecimal digits (32 bytes), with an optional 0x prefix.");
      return;
    }
    setPrivateKey(normalized);
    setStep("pin");
  }

  const back = () => {
    if (step === "restore-method") setStep("welcome");
    else if (step === "phrase") setStep("restore-method");
    else if (step === "phrase-chains") setStep("phrase");
    else if (step === "key-chains") setStep("restore-method");
    else if (step === "key-secret") setStep("key-chains");
    else if (step === "pin") setStep(mode === "create" ? "create" : mode === "phrase" ? "phrase-chains" : "key-chains");
    else setStep("welcome");
  };

  return (
    <div className="relative flex min-h-dvh flex-col bg-background">
      <div className="mx-auto flex w-full max-w-md flex-1 flex-col px-6 py-8">
        {step !== "welcome" && step !== "backup" && step !== "complete" && (
          <button type="button" onClick={back} className="mb-5 inline-flex w-fit items-center gap-2 text-sm text-muted-foreground hover:text-foreground"><ArrowLeft className="size-4" /> Back</button>
        )}

        {step === "welcome" && <div className="flex flex-1 flex-col">
          <div className="stagger-enter flex flex-1 flex-col justify-end pb-10">
            <SableMark className="mb-8 size-12" />
            <p className="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">Sable · NodeX Engine</p>
            <h1 className="mt-3 text-4xl font-medium tracking-tight text-foreground">Your wallet, your choice.</h1>
            <p className="mt-4 max-w-sm text-[15px] leading-relaxed text-muted-foreground">Create a new wallet or restore an existing one using your recovery phrase or private key.</p>
          </div>
          <div className="stagger-enter flex flex-col gap-3">
            <Button size="lg" onClick={() => { setMode("create"); setAllPhraseChains(true); setStep("create"); }}>Create wallet <ArrowRight className="size-4" /></Button>
            <Button size="lg" variant="secondary" onClick={() => { setMode("phrase"); setStep("restore-method"); }}>Restore wallet</Button>
          </div>
        </div>}

        {step === "restore-method" && <div className="flex flex-1 flex-col">
          <Header kicker="Restore wallet" title="How would you like to restore?" copy="Choose the secret you already have. Your secret is sent only to the configured NodeX engine for wallet import." />
          <div className="mt-8 space-y-3">
            <Choice icon={<KeyRound className="size-5" />} title="Recovery phrase" copy="Restore an HD wallet using 12–24 recovery words." onClick={() => { setMode("phrase"); setStep("phrase"); }} />
            <Choice icon={<KeySquare className="size-5" />} title="Private key" copy="Import one non-EVM chain or one or more EVM networks." onClick={() => { setMode("private-key"); setSelectedKey([]); setAllEvmChains(false); setStep("key-chains"); }} />
          </div>
        </div>}

        {step === "create" && <div className="flex flex-1 flex-col">
          <Header kicker="New wallet · Step 1" title="Choose blockchains" copy="Select all supported networks or choose only the networks you want this wallet to use." />
          <ChainSelection chains={chainCatalog} loading={catalogLoading} error={catalogError} allSelected={allPhraseChains} setAllSelected={setAllPhraseChains} selected={selectedPhrase} onToggle={(id, checked) => toggleChain(id, checked)} />
          <NameField name={name} setName={setName} />
          <div className="mt-auto pt-8"><Button size="lg" className="w-full" disabled={catalogLoading || !!catalogError || !selectedChains.length} onClick={() => setStep("pin")}>Continue · Set PIN</Button></div>
        </div>}

        {step === "phrase" && <div className="flex flex-1 flex-col">
          <Header kicker="Restore · Recovery phrase" title="Enter your recovery phrase" copy="Paste the words in order, separated by spaces. Check that no word is missing before continuing." />
          <div className="mt-7 space-y-2"><Label htmlFor="restore-phrase">Secret recovery phrase</Label><textarea id="restore-phrase" autoCapitalize="none" autoCorrect="off" spellCheck={false} value={restorePhrase} onChange={(e) => setRestorePhrase(e.target.value)} rows={5} className="w-full resize-none rounded-xl bg-secondary px-4 py-3 font-mono text-sm shadow-[var(--shadow-border)] outline-none focus-visible:ring-2 focus-visible:ring-ring/40" placeholder="Paste your recovery words here" /></div>
          <div className="mt-3 rounded-lg bg-card p-3 text-xs leading-relaxed text-muted-foreground"><Shield className="mr-1 inline size-3.5" /> Never share your recovery phrase with anyone, including support.</div>
          <div className="mt-auto pt-8"><Button size="lg" className="w-full" onClick={continueFromPhrase}>Continue to blockchains <ArrowRight className="size-4" /></Button></div>
        </div>}

        {step === "phrase-chains" && <div className="flex flex-1 flex-col">
          <Header kicker="Restore · Blockchains" title="Choose your networks" copy="Recommended: keep all supported blockchains selected. Turn this option off to start with an empty list and choose networks individually." />
          <ChainSelection chains={chainCatalog} loading={catalogLoading} error={catalogError} allSelected={allPhraseChains} setAllSelected={(value) => { setAllPhraseChains(value); if (!value) setSelectedPhrase([]); }} selected={selectedPhrase} onToggle={(id, checked) => toggleChain(id, checked)} />
          <NameField name={name} setName={setName} />
          <div className="mt-auto pt-8"><Button size="lg" className="w-full" disabled={catalogLoading || !!catalogError || !phraseChains.length} onClick={() => setStep("pin")}>Continue · Set PIN</Button></div>
        </div>}

        {step === "key-chains" && <div className="flex flex-1 flex-col">
          <Header kicker="Restore · Private key" title="Choose blockchains" copy="Nothing is selected automatically. A private key can be used across EVM networks; non-EVM networks are listed separately and must be imported one at a time." />
          <div className="mt-6 space-y-3">
            <label className="flex cursor-pointer items-start gap-3 rounded-xl bg-card p-4 shadow-[var(--shadow-border)]">
              <input type="checkbox" checked={allEvmChains} onChange={(e) => { setAllEvmChains(e.target.checked); setSelectedKey(e.target.checked ? [] : []); }} className="mt-1 size-4 accent-foreground" />
              <span className="min-w-0 flex-1"><span className="block text-sm font-semibold">Select all EVM blockchains</span><span className="mt-1 block text-xs text-muted-foreground">One EVM address can be used on {evmChains.length} EVM networks.</span></span>
            </label>
            <p className="px-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground">EVM networks</p>
            <div className="max-h-64 space-y-2 overflow-y-auto pr-1">{evmChains.map((chain) => <ChainRow key={chain.chain_id} chain={chain} checked={allEvmChains || selectedKey.includes(chain.chain_id)} disabled={allEvmChains} onChange={(checked) => toggleChain(chain.chain_id, checked, true)} />)}</div>
            <p className="px-1 pt-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">Non-EVM networks · choose one</p>
            <div className="max-h-64 space-y-2 overflow-y-auto pr-1">{nonEvmChains.map((chain) => <ChainRow key={chain.chain_id} chain={chain} checked={selectedKey.includes(chain.chain_id)} disabled={allEvmChains} onChange={(checked) => { if (checked) { setAllEvmChains(false); setSelectedKey([chain.chain_id]); } else toggleChain(chain.chain_id, false, true); }} />)}</div>
          </div>
          <NameField name={name} setName={setName} />
          <div className="mt-auto pt-8"><Button size="lg" className="w-full" disabled={catalogLoading || !!catalogError || !keyChains.length} onClick={continueFromKeyChains}>Continue <ArrowRight className="size-4" /></Button></div>
        </div>}

        {step === "key-secret" && <div className="flex flex-1 flex-col">
          <Header kicker="Restore · Private key" title="Enter your private key" copy="Paste the private key for the blockchain selection you just made. It stays hidden by default." />
          <div className="mt-7 space-y-2"><Label htmlFor="private-key">Private key</Label><div className="flex gap-2"><Input id="private-key" type={showSecret ? "text" : "password"} autoComplete="off" spellCheck={false} value={privateKey} onChange={(e) => setPrivateKey(e.target.value)} placeholder="Paste private key" className="font-mono" /><Button type="button" variant="secondary" size="icon" onClick={() => setShowSecret((value) => !value)} aria-label={showSecret ? "Hide private key" : "Show private key"}>{showSecret ? <EyeOff className="size-4" /> : <Eye className="size-4" />}</Button></div><p className="text-xs text-muted-foreground">Never share this key. NodeX request logs redact secret fields.</p></div>
          <div className="mt-auto pt-8"><Button size="lg" className="w-full" disabled={!privateKey.trim()} onClick={continueFromPrivateKey}>Continue · Set PIN <ArrowRight className="size-4" /></Button></div>
        </div>}

        {step === "pin" && <div className="flex flex-1 flex-col">
          <Header kicker="Security · Final step" title="Protect your wallet" copy="Set a six-digit PIN. The engine uses it to protect imported wallet secrets." />
          <div className="mt-8 space-y-4">
            <div className="space-y-2"><Label htmlFor="pin">6-digit PIN</Label><Input id="pin" inputMode="numeric" maxLength={6} value={pin} onChange={(e) => setPinValue(e.target.value.replace(/\D/g, "").slice(0, 6))} placeholder="••••••" className="font-mono tracking-[0.4em]" /></div>
            <div className="space-y-2"><Label htmlFor="pin2">Confirm PIN</Label><Input id="pin2" inputMode="numeric" maxLength={6} value={pin2} onChange={(e) => setPin2(e.target.value.replace(/\D/g, "").slice(0, 6))} placeholder="••••••" className="font-mono tracking-[0.4em]" /></div>
            <p className="flex items-start gap-2 text-xs leading-relaxed text-muted-foreground"><Shield className="mt-0.5 size-3.5 shrink-0" /> The PIN is required before the import is submitted to NodeX.</p>
          </div>
          <div className="mt-auto space-y-2 pt-8">
            <Button size="lg" className="w-full" disabled={busy || pin.length !== 6 || pin2.length !== 6} onClick={onPin}>{busy ? <><LoaderCircle className="size-4 animate-spin" /> Connecting to engine…</> : mode === "create" ? "Create wallet" : "Import wallet"}</Button>
          </div>
        </div>}

        {step === "backup" && wallet && <div className="flex flex-1 flex-col">
          <Header kicker="Recovery" title="Save your recovery phrase" copy="Keep these words somewhere private and secure. Anyone with this phrase can control the wallet." />
          <div className="mt-8 rounded-xl bg-secondary p-4 shadow-[var(--shadow-border)]"><ol className="grid grid-cols-2 gap-2 sm:grid-cols-3">{wallet.phrase.split(" ").map((word, i) => <li key={i} className="rounded-md bg-background/50 px-3 py-2 font-mono text-sm"><span className="mr-2 text-xs text-muted-foreground">{i + 1}</span>{word}</li>)}</ol></div>
          <div className="mt-auto pt-8"><Button size="lg" className="w-full" onClick={() => completeSetup()}>I saved my phrase · Open wallet <Check className="size-4" /></Button></div>
        </div>}

        {step === "complete" && wallet && <div className="flex flex-1 flex-col">
          <div className="my-auto py-10 text-center"><div className="mx-auto mb-5 flex size-16 items-center justify-center rounded-full bg-emerald-500/10 text-emerald-500"><Check className="size-8" /></div><Header kicker="Import complete" title="Wallet imported" copy="Your selected blockchain address records have been registered by NodeX. Keep your private key safe." /><p className="mt-5 break-all text-xs text-muted-foreground">Wallet ID: {wallet.engineWalletId}</p></div>
          <Button size="lg" className="w-full" onClick={() => completeSetup()}>Open wallet <ArrowRight className="size-4" /></Button>
        </div>}
      </div>
    </div>
  );
}

function ChainSelection({ chains, loading, error, allSelected, setAllSelected, selected, onToggle }: { chains: SelectedChain[]; loading: boolean; error: string | null; allSelected: boolean; setAllSelected: (value: boolean) => void; selected: string[]; onToggle: (id: string, checked: boolean) => void }) {
  return <div className="mt-6">
    <label className="flex cursor-pointer items-start gap-3 rounded-xl bg-card p-4 shadow-[var(--shadow-border)]">
      <input type="checkbox" checked={allSelected} onChange={(e) => { setAllSelected(e.target.checked); }} className="mt-1 size-4 accent-foreground" />
      <span className="min-w-0 flex-1"><span className="block text-sm font-semibold">Select all blockchains · Recommended</span><span className="mt-1 block text-xs text-muted-foreground">All {chains.length} supported mainnet networks</span></span>
    </label>
    {loading && <p className="mt-3 text-sm text-muted-foreground">Loading supported blockchains from NodeX…</p>}
    {error && <p role="alert" className="mt-3 rounded-lg border border-destructive/30 p-3 text-sm text-destructive">{error}</p>}
    <div className="mt-3 max-h-[38dvh] space-y-2 overflow-y-auto pr-1">{chains.map((chain) => <ChainRow key={chain.chain_id} chain={chain} checked={!allSelected && selected.includes(chain.chain_id)} disabled={allSelected} onChange={(checked) => onToggle(chain.chain_id, checked)} />)}</div>
  </div>;
}
function ChainRow({ chain, checked, disabled, onChange }: { chain: SelectedChain; checked: boolean; disabled?: boolean; onChange: (checked: boolean) => void }) {
  return <label className="flex cursor-pointer items-center gap-3 rounded-lg bg-secondary/60 px-3 py-3">
    <input type="checkbox" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} className="size-4 accent-foreground" />
    <span className="min-w-0 flex-1"><span className="block text-sm font-medium">{chain.name}</span><span className="block text-xs text-muted-foreground">{chain.symbol} · {chain.chain_type} · {chain.chain_id}</span></span>
  </label>;
}
function NameField({ name, setName }: { name: string; setName: (name: string) => void }) {
  return <div className="mt-5 space-y-2"><Label htmlFor="wallet-name">Wallet name (optional)</Label><Input id="wallet-name" value={name} onChange={(e) => setName(e.target.value)} placeholder="My wallet" /></div>;
}
function Choice({ icon, title, copy, onClick }: { icon: React.ReactNode; title: string; copy: string; onClick: () => void }) {
  return <button type="button" onClick={onClick} className="flex w-full items-center gap-4 rounded-xl bg-card p-4 text-left shadow-[var(--shadow-border)] transition-colors hover:bg-secondary"><span className="flex size-11 shrink-0 items-center justify-center rounded-full bg-secondary">{icon}</span><span className="min-w-0 flex-1"><span className="block font-semibold">{title}</span><span className="mt-1 block text-sm leading-relaxed text-muted-foreground">{copy}</span></span><ArrowRight className="size-4 shrink-0 text-muted-foreground" /></button>;
}
function Header({ kicker, title, copy }: { kicker: string; title: string; copy: string }) {
  return <div className="stagger-enter pt-2"><p className="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">{kicker}</p><h1 className="mt-3 text-3xl font-medium tracking-tight">{title}</h1><p className="mt-3 text-[15px] leading-relaxed text-muted-foreground">{copy}</p></div>;
}
