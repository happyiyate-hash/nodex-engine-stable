import { create } from "zustand";
import { persist } from "zustand/middleware";
import { type AssetId, type NetworkId } from "@/lib/wallet/assets";
import { hashPin, normalizeMnemonic, type Addresses } from "@/lib/wallet/mnemonic";
import { randomHex } from "@/lib/utils";
import { chainRecords, engineRequest, walletContext, type SelectedChain } from "@/lib/wallet/engine";
import { generateLocalRecoveryPhrase, validateLocalRecoveryPhrase } from "@/lib/wallet/wasm";
import type { EngineStreamEvent } from "@/lib/wallet/event-stream";

export type TxType = "send" | "receive" | "swap";
export type TxStatus = "confirmed" | "pending";
export type Transaction = {
  id: string; type: TxType; status: TxStatus; assetId: AssetId; amount: number;
  toAssetId?: AssetId; toAmount?: number; counterparty?: string; hash: string; timestamp: number;
};
export type EngineBalance = {
  wallet_id?: string; chain_id?: string; chain_name?: string; name?: string; chain_type?: string; address?: string;
  native?: { symbol?: string; balance?: string; decimals?: number; name?: string; address?: string } | null;
  tokens?: Array<{ symbol?: string; balance?: string; decimals?: number; name?: string; address?: string; is_native?: boolean }>;
};
export type WalletRecord = {
  name: string;
  derivationErrors: Array<Record<string, unknown>>;
  phrase: string;
  addresses: Addresses;
  chainAddresses: Array<Record<string, unknown>>;
  engineWalletId: string;
  childId: string;
  childIndex: number;
  createdAt: number;
  pinHash: string | null;
  pinSalt: string | null;
};
type ChainFetchState = { status: "idle" | "loading" | "success" | "error"; message?: string; updatedAt?: string };
type WalletState = {
  wallet: WalletRecord | null;
  holdings: Record<AssetId, number>;
  balances: EngineBalance[];
  chainFetchState: Record<string, ChainFetchState>;
  txs: Transaction[];
  locked: boolean;
  hideBalances: boolean;
  network: NetworkId;
  hydrated: boolean;
  setupComplete: boolean;
  refreshingBalances: boolean;
  lastBalanceRefresh: string | null;
  lastEngineError: string | null;
  setHydrated: () => void;
  completeSetup: () => void;
  createWallet: (name: string, pin: string, chains?: SelectedChain[]) => Promise<WalletRecord>;
  restoreWallet: (phrase: string, name: string, pin: string, chains?: SelectedChain[]) => Promise<WalletRecord>;
  importPrivateKey: (privateKey: string, name: string, pin: string, chains: SelectedChain[]) => Promise<WalletRecord>;
  refreshBalances: () => Promise<void>;
  applyEngineEvent: (event: EngineStreamEvent) => void;
  setPin: (pin: string) => Promise<void>;
  unlock: (pin: string) => Promise<boolean>;
  lock: () => void;
  send: (input: { assetId: AssetId; amount: number; to: string }) => Transaction;
  swap: (input: { fromId: AssetId; toId: AssetId; fromAmount: number; toAmount: number }) => Transaction;
  toggleHideBalances: () => void;
  setNetwork: (network: NetworkId) => void;
  rename: (name: string) => void;
  reset: () => void;
};
const EMPTY_HOLDINGS: Record<AssetId, number> = {
  ethereum: 0, bitcoin: 0, "usd-coin": 0, solana: 0, chainlink: 0, uniswap: 0,
};
const selectedDefaults = (): SelectedChain[] => [];
let balanceRefreshInFlight = false;

function makeAddresses(chains: Array<Record<string, unknown>>): Addresses {
  const find = (type: string, name?: string) => {
    const match = chains.find((c) => String(c.chain_type ?? "").toLowerCase() === type || String(c.name ?? "").toLowerCase() === name);
    return String(match?.address ?? "");
  };
  return {
    evm: find("evm", "ethereum"),
    sol: find("solana", "solana"),
    btc: find("btc", "bitcoin"),
    byChain: Object.fromEntries(chains.map((c) => [String(c.chain_id ?? c.name ?? "unknown"), String(c.address ?? "")])),
  } as Addresses;
}

async function createOrImport(name: string, pin: string, chains: SelectedChain[], phrase?: string): Promise<WalletRecord> {
  // Generate new recovery material in the local Rust/WASM core. The engine still
  // registers the wallet, derives authoritative addresses, and encrypts its secrets.
  const normalizedPhrase = phrase ? normalizeMnemonic(phrase) : await generateLocalRecoveryPhrase();
  if (!(await validateLocalRecoveryPhrase(normalizedPhrase))) {
    throw new Error("The local NodeX runtime rejected the recovery phrase. No wallet was registered.");
  }
  const body = await engineRequest<Record<string, unknown>>("import_wallet", {
    name: name.trim() || (phrase ? "Restored" : "Primary"),
    pin,
    word_count: 12,
    mnemonic: normalizedPhrase,
    chains: chains.map((chain) => ({ ...chain })),
  });
  const raw = walletContext(body.data);
  const engineWalletId = String(raw.wallet_group_id ?? raw.wallet_id ?? raw.id ?? "");
  // The current engine response returns addresses as a name -> address object,
  // not always as a chain/blockchain array. Normalize both formats for the UI.
  const chainAddressesFromEngine = chainRecords(raw);
  const addressMapValue = raw.addresses ?? body.data?.addresses;
  const addressMap = addressMapValue && typeof addressMapValue === "object" && !Array.isArray(addressMapValue)
    ? addressMapValue as Record<string, unknown>
    : {};
  const chainAddresses = chainAddressesFromEngine.length
    ? chainAddressesFromEngine
    : chains.map((chain) => ({
        ...chain,
        address: String(addressMap[chain.name] ?? addressMap[chain.chain_type] ?? ""),
      })).filter((chain) => chain.address.length > 0);
  const derivationErrorValue = raw.derivation_errors ?? body.data?.derivation_errors;
  const derivationErrors = Array.isArray(derivationErrorValue)
    ? derivationErrorValue.filter((item): item is Record<string, unknown> => !!item && typeof item === "object" && !Array.isArray(item))
    : [];
  // child_id is the registered derivation child (e.g. wallet_2_child_0);
  // wallet_id identifies the parent wallet and must not be substituted for it.
  const childId = String(raw.child_id ?? raw.childId ?? raw.wallet_child_id ?? raw.id ?? engineWalletId);
  if (!engineWalletId || !childId || chainAddresses.some((chain) => !chain.address) || (!chainAddresses.length && !derivationErrors.length)) {
    throw new Error("Engine response was successful but the wallet ID, child ID, or chain addresses could not be normalized. Open Engine request logs for the redacted response.");
  }
  let returnedPhrase = normalizedPhrase;
  if (!returnedPhrase) {
    const phraseResponse = await engineRequest<Record<string, unknown>>("get_mnemonic", {
      wallet_id: engineWalletId, child_id: childId, pin, include_secret_phrase: true,
    });
    returnedPhrase = String(phraseResponse.data?.secret_phrase ?? walletContext(phraseResponse.data).mnemonic ?? "");
  }
  if (!returnedPhrase) {
    throw new Error("Wallet was created by the engine, but it did not return the recovery phrase. See Settings/Engine logs; do not invent or derive a phrase in the frontend.");
  }
  const now = Date.now();
  return {
    name: name.trim() || "Primary",
    derivationErrors,
    phrase: returnedPhrase,
    addresses: makeAddresses(chainAddresses),
    chainAddresses,
    engineWalletId,
    childId,
    childIndex: Number(raw.child_index ?? 0),
    createdAt: now,
    pinHash: null,
    pinSalt: null,
  };
}

export const useWallet = create<WalletState>()(persist((set, get) => ({
  wallet: null, holdings: { ...EMPTY_HOLDINGS }, balances: [], chainFetchState: {}, txs: [], locked: false,
  hideBalances: false, network: "all", hydrated: false, setupComplete: true,
  refreshingBalances: false, lastBalanceRefresh: null, lastEngineError: null,
  setHydrated: () => set({ hydrated: true }),
  completeSetup: () => set({ setupComplete: true }),
  createWallet: async (name, pin, chains = selectedDefaults()) => {
    const wallet = await createOrImport(name, pin, chains);
    set({ wallet, holdings: { ...EMPTY_HOLDINGS }, balances: [], chainFetchState: {}, txs: [], locked: false, setupComplete: false, lastEngineError: null });
    return wallet;
  },
  restoreWallet: async (phrase, name, pin, chains = selectedDefaults()) => {
    const wallet = await createOrImport(name, pin, chains, phrase);
    set({ wallet, holdings: { ...EMPTY_HOLDINGS }, balances: [], txs: [], locked: false, setupComplete: false, lastEngineError: null });
    return wallet;
  },
  importPrivateKey: async (privateKey, name, pin, chains) => {
    if (!chains.length) throw new Error("Select at least one blockchain.");
    const selectedEvm = chains.every((chain) => chain.chain_type.toLowerCase() === "evm");
    if (chains.length > 1 && !selectedEvm) throw new Error("A private key can be imported to multiple EVM networks, or one non-EVM network at a time.");
    const body = await engineRequest<Record<string, unknown>>("import_private_key", {
      name: name.trim() || "Imported wallet",
      pin,
      private_key: privateKey.trim(),
      chains: chains.map((chain) => ({ ...chain })),
    });
    const raw = walletContext(body.data);
    const engineWalletId = String(raw.wallet_group_id ?? raw.wallet_id ?? raw.id ?? "");
    const chainAddresses = chainRecords(raw);
    const derivationErrorValue = raw.derivation_errors ?? body.data?.derivation_errors;
    const derivationErrors = Array.isArray(derivationErrorValue)
      ? derivationErrorValue.filter((item): item is Record<string, unknown> => !!item && typeof item === "object" && !Array.isArray(item))
      : [];
    const childId = String(raw.wallet_id ?? raw.child_id ?? raw.childId ?? "");
    if (!engineWalletId || !childId || chainAddresses.some((chain) => !chain.address) || (!chainAddresses.length && !derivationErrors.length)) {
      throw new Error("The engine did not return a complete private-key wallet record. Check the redacted Engine request log.");
    }
    const wallet: WalletRecord = {
      name: String(raw.name ?? name.trim() ?? "Imported wallet"),
      derivationErrors,
      phrase: "",
      addresses: makeAddresses(chainAddresses),
      chainAddresses,
      engineWalletId,
      childId,
      childIndex: Number(raw.child_index ?? 0),
      createdAt: Date.now(),
      pinHash: null,
      pinSalt: null,
    };
    set({ wallet, holdings: { ...EMPTY_HOLDINGS }, balances: [], chainFetchState: {}, txs: [], locked: false, setupComplete: false, lastEngineError: null });
    return wallet;
  },
  refreshBalances: async () => {
    const wallet = get().wallet;
    if (!wallet) throw new Error("Create or restore a wallet first.");
    // Do not launch a second full-wallet refresh while one is already active.
    if (balanceRefreshInFlight || get().refreshingBalances) return;
    balanceRefreshInFlight = true;
    const initialStates = Object.fromEntries(wallet.chainAddresses.map((chain, index) => [
      String(chain.chain_id ?? chain.name ?? ""),
      { status: (index === 0 ? "loading" : "idle") as "loading" | "idle" },
    ]));
    set({ refreshingBalances: true, lastEngineError: null, chainFetchState: { ...get().chainFetchState, ...initialStates } });
    try {
      const result = await engineRequest<Record<string, unknown>>("refresh_balance", {
        wallet_id: wallet.engineWalletId,
        child_id: wallet.childId,
      });
      // The engine has returned several response shapes across versions:
      // { balances, errors }, { raw: { balances, errors } }, and chain results
      // nested under data. Normalize these before touching UI state.
      const envelopeData = result.data && typeof result.data === "object" ? result.data as Record<string, unknown> : {};
      const payload = walletContext(envelopeData);
      const raw = envelopeData.raw && typeof envelopeData.raw === "object" ? envelopeData.raw as Record<string, unknown> : {};
      const candidates = [payload.balances, raw.balances, envelopeData.balances, payload.results, raw.results];
      const balances = candidates.find(Array.isArray) as EngineBalance[] | undefined ?? [];
      const errorCandidates = [payload.errors, raw.errors, envelopeData.errors, payload.derivation_errors, raw.derivation_errors];
      const errors = errorCandidates.find(Array.isArray) as Array<Record<string, unknown>> | undefined ?? [];
      const now = new Date().toISOString();
      const nextStates = { ...get().chainFetchState };
      const mergedBalances = [...get().balances];
      const findChainKey = (...values: unknown[]) => {
        const ids = new Set(values.map((value) => String(value ?? "").trim().toLowerCase()).filter(Boolean));
        if (!ids.size) return "";
        const exact = wallet.chainAddresses.filter((chain) =>
          [chain.chain_id, chain.name, chain.chain_name].some((value) => ids.has(String(value ?? "").trim().toLowerCase())));
        const exactKeys = [...new Set(exact.map((chain) => String(chain.chain_id ?? chain.name ?? "")))];
        // If identifiers disagree (e.g. chain_id says Ethereum but chain_name
        // says Polygon), reject the response instead of choosing the first match.
        if (exactKeys.length === 1) return exactKeys[0]!;
        if (exactKeys.length > 1) return "";
        // Adapter family names and symbols are safe only when exactly one
        // selected network matches. A generic "evm" must never pick a card.
        const aliases = wallet.chainAddresses.filter((chain) =>
          [chain.chain_type, chain.symbol].some((value) => ids.has(String(value ?? "").trim().toLowerCase())));
        const aliasKeys = [...new Set(aliases.map((chain) => String(chain.chain_id ?? chain.name ?? "")))];
        return aliasKeys.length === 1 ? aliasKeys[0]! : "";
      };
      for (const balance of balances) {
        const key = findChainKey(balance.chain_id, balance.chain_name, balance.name, balance.chain_type);
        // Never identify a blockchain by address: EVM networks commonly share
        // the same address, so address-based merging can move Polygon assets into Ethereum.
        if (!key) continue;
        nextStates[key] = { status: "success", updatedAt: now };
        const normalizedBalance = { ...balance, chain_id: key };
        const index = mergedBalances.findIndex((item) =>
          String(item.chain_id ?? "").trim().toLowerCase() === String(normalizedBalance.chain_id ?? "").trim().toLowerCase());
        if (index >= 0) mergedBalances[index] = { ...mergedBalances[index], ...normalizedBalance };
        else mergedBalances.push(normalizedBalance);
      }
      for (const error of errors) {
        const key = findChainKey(error.chain_id, error.chain, error.chain_name, error.name);
        if (key) nextStates[key] = { status: "error", message: String(error.error ?? error.message ?? "Balance unavailable"), updatedAt: now };
      }
      // Every chain included in this refresh must leave "loading" when the
      // response is received, even if an older engine omitted a matching error
      // entry or returned an unfamiliar response shape for that chain.
      for (const chain of wallet.chainAddresses) {
        const key = String(chain.chain_id ?? chain.name ?? "");
        if (nextStates[key]?.status === "loading" || nextStates[key]?.status === "idle") {
          nextStates[key] = {
            status: "error",
            message: "The engine returned no balance result for this blockchain.",
            updatedAt: now,
          };
        }
      }
      // A returned balance object without a numeric amount is still a completed
      // response; render its real payload rather than leaving a spinner forever.
      set({ balances: mergedBalances, chainFetchState: nextStates, lastBalanceRefresh: now });
      if (errors.length) {
        set({ lastEngineError: errors.map((e) => String(e.error ?? e.message ?? JSON.stringify(e))).join("; ") });
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Balance refresh failed";
      const nextStates = { ...get().chainFetchState };
      for (const chain of wallet.chainAddresses) {
        const key = String(chain.chain_id ?? chain.name ?? "");
        if (nextStates[key]?.status === "loading" || nextStates[key]?.status === "idle") nextStates[key] = { status: "error", message };
      }
      set({ lastEngineError: message, chainFetchState: nextStates });
      throw error;
    } finally {
      balanceRefreshInFlight = false;
      set({ refreshingBalances: false });
    }
  },
  applyEngineEvent: (event) => {
    const data = event.data ?? {};
    const chainId = String(event.chain_id ?? data.chain_id ?? data.chain_name ?? data.blockchain ?? (data.balance && typeof data.balance === "object" ? (data.balance as Record<string, unknown>).chain_id : "") ?? "");
    const now = new Date(Number(event.timestamp_ms ?? Date.now())).toISOString();
    const eventType = String(event.event_type ?? "").toLowerCase();
    if (eventType === "balance.fetch.started" && !chainId) {
      set((s) => ({ refreshingBalances: true, chainFetchState: Object.fromEntries(Object.entries(s.chainFetchState).map(([id, state]) => [id, { ...state, status: "loading" as const }])) }));
      return;
    }
    if ((eventType === "balance.refresh.completed" || eventType === "balance.fetch.completed" || eventType === "balance.refresh.finished") && !chainId) {
      set({ refreshingBalances: balanceRefreshInFlight, lastBalanceRefresh: now });
      return;
    }
    if ((eventType === "balance.refresh.failed" || eventType === "balance.fetch.failed") && !chainId) {
      const message = String(data.message ?? "Balance refresh failed");
      set((s) => ({ refreshingBalances: balanceRefreshInFlight, lastEngineError: message, chainFetchState: Object.fromEntries(Object.entries(s.chainFetchState).map(([id, state]) => [id, state.status === "loading" ? { status: "error" as const, message, updatedAt: now } : state])) }));
      return;
    }
    if (!chainId) return;

    // Ignore balance updates for a different wallet if the engine provides a
    // wallet/child identifier. This prevents one wallet's events updating another.
    const activeWallet = get().wallet;
    const eventWalletId = String(event.wallet_id ?? data.wallet_id ?? data.child_id ?? "");
    if (eventWalletId && activeWallet
      && eventWalletId !== activeWallet.engineWalletId
      && eventWalletId !== activeWallet.childId) return;

    const resolvedChainKey = (() => {
      const wanted = chainId.trim().toLowerCase();
      const exact = activeWallet?.chainAddresses.find((chain) =>
        [chain.chain_id, chain.name, chain.chain_name].some((value) => String(value ?? "").trim().toLowerCase() === wanted));
      if (exact) return String(exact.chain_id ?? exact.name ?? "");
      const aliases = activeWallet?.chainAddresses.filter((chain) =>
        [chain.chain_type, chain.symbol].some((value) => String(value ?? "").trim().toLowerCase() === wanted)) ?? [];
      return aliases.length === 1 ? String(aliases[0]!.chain_id ?? aliases[0]!.name ?? "") : "";
    })();
    // Unknown or ambiguous identifiers must never mutate another network's card.
    if (!resolvedChainKey) return;

    if (eventType === "balance.fetch.started") {
      set((s) => ({ chainFetchState: { ...s.chainFetchState, [resolvedChainKey]: { status: "loading" } } }));
      return;
    }
    if (eventType === "balance.fetch.failed") {
      const message = String(data.message ?? "Balance fetch failed");
      set((s) => {
        const nextStates = { ...s.chainFetchState, [resolvedChainKey]: { status: "error" as const, message, updatedAt: now } };
        return {
          chainFetchState: nextStates,
          // The global refresh stays active between sequential chain requests.
          // Only balance.refresh.completed (or the HTTP response) ends it.
          lastEngineError: message,
        };
      });
      return;
    }
    // balance.result is emitted as soon as an individual chain adapter returns,
    // so the UI can update progressively before the whole refresh completes.
    if (eventType === "balance.result" || eventType === "balance.updated") {
      const incomingValue = (data.balance && typeof data.balance === "object" ? data.balance : data) as unknown as EngineBalance;
      // If the event envelope and its payload disagree about the chain, reject it.
      // A malformed event must never overwrite another network's balance card.
      const payloadIds = [incomingValue.chain_id, incomingValue.chain_name, incomingValue.name]
        .map((value) => String(value ?? "").trim().toLowerCase()).filter(Boolean);
      if (payloadIds.length) {
        const payloadMatches = activeWallet?.chainAddresses.filter((chain) =>
          [chain.chain_id, chain.name, chain.chain_name].some((value) =>
            payloadIds.includes(String(value ?? "").trim().toLowerCase()))) ?? [];
        const payloadKeys = [...new Set(payloadMatches.map((chain) => String(chain.chain_id ?? chain.name ?? "")))];
        if (payloadKeys.length !== 1 || payloadKeys[0]!.trim().toLowerCase() !== resolvedChainKey.trim().toLowerCase()) return;
      }
      const normalizedKey = (() => {
        const wanted = chainId.toLowerCase();
        const exact = activeWallet?.chainAddresses.find((chain) =>
          [chain.chain_id, chain.name, chain.chain_name].some((value) => String(value ?? "").trim().toLowerCase() === wanted));
        if (exact) return String(exact.chain_id ?? exact.name ?? "");
        const aliases = activeWallet?.chainAddresses.filter((chain) =>
          [chain.chain_type, chain.symbol].some((value) => String(value ?? "").trim().toLowerCase() === wanted)) ?? [];
        return aliases.length === 1 ? String(aliases[0]!.chain_id ?? aliases[0]!.name ?? "") : "";
      })();
      set((s) => {
        const incoming = { ...incomingValue, chain_id: normalizedKey };
        const existing = s.balances.filter((item) =>
          String(item.chain_id ?? "").trim().toLowerCase() !== String(incoming.chain_id ?? "").trim().toLowerCase());
        const nextStates = { ...s.chainFetchState, [normalizedKey]: { status: "success" as const, updatedAt: now } };
        return {
          balances: [...existing, incoming],
          // Keep the refresh lock while the engine moves to the next chain.
          refreshingBalances: s.refreshingBalances,
          chainFetchState: nextStates,
          lastBalanceRefresh: now,
        };
      });
      return;
    }
    if (eventType === "balance.fetch.completed") {
      set((s) => {
        const nextStates = { ...s.chainFetchState, [resolvedChainKey]: { status: "success" as const, updatedAt: now } };
        return { chainFetchState: nextStates, lastBalanceRefresh: now };
      });
      return;
    }
    if (eventType === "balance.result.failed" || eventType === "balance.updated.failed") {
      const message = String(data.message ?? data.error ?? "Balance fetch failed");
      set((s) => {
        const nextStates = { ...s.chainFetchState, [resolvedChainKey]: { status: "error" as const, message, updatedAt: now } };
        return { chainFetchState: nextStates, lastEngineError: message };
      });
      return;
    }
  },
  setPin: async (pin) => {
    const wallet = get().wallet; if (!wallet) return;
    const pinSalt = randomHex(16); const pinHash = await hashPin(pin, pinSalt);
    set({ wallet: { ...wallet, pinHash, pinSalt } });
  },
  unlock: async (pin) => {
    const wallet = get().wallet;
    if (!wallet?.pinHash || !wallet.pinSalt) { set({ locked: false }); return true; }
    const next = await hashPin(pin, wallet.pinSalt);
    if (next !== wallet.pinHash) return false;
    set({ locked: false }); return true;
  },
  lock: () => { if (get().wallet?.pinHash) set({ locked: true }); },
  send: () => { throw new Error("Sending is disabled until an engine-backed transaction/broadcast flow is implemented."); },
  swap: () => { throw new Error("Swaps are disabled until an engine-backed swap flow is implemented."); },
  toggleHideBalances: () => set((s) => ({ hideBalances: !s.hideBalances })),
  setNetwork: (network) => set({ network }),
  rename: (name) => { const wallet = get().wallet; if (wallet) set({ wallet: { ...wallet, name: name.trim() || wallet.name } }); },
  reset: () => set({ wallet: null, holdings: { ...EMPTY_HOLDINGS }, balances: [], chainFetchState: {}, txs: [], locked: false, hideBalances: false, network: "all", setupComplete: true, lastBalanceRefresh: null, lastEngineError: null }),
}), {
  name: "sable-wallet",
  skipHydration: true,
  partialize: (s) => ({
    wallet: s.wallet, holdings: s.holdings, balances: s.balances, chainFetchState: s.chainFetchState, txs: s.txs,
    hideBalances: s.hideBalances, network: s.network,
    locked: s.wallet?.pinHash ? true : false, setupComplete: s.setupComplete,
    lastBalanceRefresh: s.lastBalanceRefresh,
  }),
  onRehydrateStorage: () => (state) => {
    state?.setHydrated();
    // Discard pre-integration demo wallets so fabricated addresses/balances
    // can never be mistaken for engine-registered wallet data.
    if (state?.wallet && !state.wallet.engineWalletId) state.reset();
    if (state?.wallet?.pinHash) state.lock();
  },
}));
