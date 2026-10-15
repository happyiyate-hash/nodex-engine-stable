export type EngineLog = {
  id: string;
  at: string;
  action: string;
  ok: boolean;
  durationMs: number;
  request: { action: string; params: Record<string, unknown> };
  response?: unknown;
  error?: string;
};

export type EngineEnvelope<T = Record<string, unknown>> = {
  success: boolean;
  data?: {
    success?: boolean;
    message?: string;
    secret_phrase?: string | null;
    addresses?: unknown;
    raw?: T;
    [key: string]: unknown;
  } | null;
  error?: string | null;
};

const LOG_KEY = "sable-engine-logs";
const LOG_LIMIT = 150;

export function getEngineBaseUrl(): string {
  return (import.meta.env.VITE_NODEX_ENGINE_URL as string | undefined)?.trim().replace(/\/$/, "") || "https://nodex-engine-stable.onrender.com";
}

function safeParams(params: Record<string, unknown>) {
  const copy = { ...params };
  for (const key of ["pin", "password", "mnemonic", "secret_phrase", "private_key", "seed"]) {
    if (key in copy) copy[key] = "[REDACTED]";
  }
  return copy;
}

export function readEngineLogs(): EngineLog[] {
  if (typeof window === "undefined") return [];
  // Merge both stores so a restricted/cleared localStorage does not make the
  // diagnostics page appear empty when this tab still has its request history.
  const byId = new Map<string, EngineLog>();
  for (const storage of [window.localStorage, window.sessionStorage]) {
    try {
      const raw = storage.getItem(LOG_KEY);
      if (!raw) continue;
      const parsed = JSON.parse(raw) as unknown;
      if (Array.isArray(parsed)) {
        for (const item of parsed) {
          if (item && typeof item === "object" && typeof item.id === "string" && typeof item.action === "string") {
            byId.set(item.id, item as EngineLog);
          }
        }
      }
    } catch { /* Continue with the other store if one is unavailable or malformed. */ }
  }
  return [...byId.values()].sort((a, b) => b.at.localeCompare(a.at)).slice(0, LOG_LIMIT);
}

function redactResponse(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(redactResponse);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(Object.entries(value as Record<string, unknown>).map(([key, item]) => [
    key, ["mnemonic", "secret_phrase", "encrypted_mnemonic", "private_key", "seed", "password", "pin", "api_key", "token"].includes(key.toLowerCase())
      ? "[REDACTED]" : redactResponse(item),
  ]));
}

function writeLog(log: EngineLog) {
  if (typeof window === "undefined") return;
  const logs = [log, ...readEngineLogs().filter((entry) => entry.id !== log.id)]
    .sort((a, b) => b.at.localeCompare(a.at)).slice(0, LOG_LIMIT);
  const serialized = JSON.stringify(logs);
  let saved = false;
  // Persist across reloads where possible, and retain a per-tab fallback.
  try { window.localStorage.setItem(LOG_KEY, serialized); saved = true; } catch { /* Storage can be blocked or full. */ }
  try { window.sessionStorage.setItem(LOG_KEY, serialized); saved = true; } catch { /* Keep logging best-effort. */ }
  // Always notify the current page even when browser storage is unavailable.
  window.dispatchEvent(new CustomEvent("sable-engine-log", { detail: logs }));
  if (!saved) console.warn("NodeX request log could not be persisted; it is available only in memory for this page.");
}

export async function engineRequest<T = Record<string, unknown>>(
  action: string,
  params: Record<string, unknown> = {},
): Promise<EngineEnvelope<T>> {
  const started = Date.now();
  let logged = false;
  const base = getEngineBaseUrl();
  const request = { action, params };
  try {
    
    let response: Response;
    try {
      response = await fetch(`${base}/api/request`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(request),
      });
    } catch (error) {
      if (error instanceof TypeError) throw new Error(`Could not reach the NodeX engine at ${base}. Check that the engine service is running and allows requests from this frontend origin (CORS).`);
      throw error;
    }
    let body: EngineEnvelope<T>;
    try { body = await response.json() as EngineEnvelope<T>; }
    catch { throw new Error(`NodeX engine returned a non-JSON response (HTTP ${response.status}). Check the engine URL and server logs.`); }
    // A refresh can return useful per-chain results while the overall envelope
    // is unsuccessful because other chains failed. Accept both current
    // { data: { balances, errors } } and older { data: { raw: { balances, errors } } } shapes.
    const dataObject = body.data && typeof body.data === "object"
      ? body.data as Record<string, unknown>
      : {};
    const rawBalanceData = dataObject.raw && typeof dataObject.raw === "object"
      ? dataObject.raw as Record<string, unknown>
      : {};
    const balancePayload = Array.isArray(dataObject.balances)
      ? dataObject
      : rawBalanceData;
    const partialBalanceResponse = action === "refresh_balance"
      && Array.isArray(balancePayload.balances);
    const balanceErrorCount = Array.isArray(balancePayload.errors) ? balancePayload.errors.length : 0;
    const ok = response.ok && (body.success === true && body.data?.success !== false || partialBalanceResponse);
    const logError = balanceErrorCount > 0
      ? `Balance refresh returned ${Array.isArray(balancePayload.balances) ? balancePayload.balances.length : 0} successful chain result(s) and ${balanceErrorCount} chain error(s)`
      : ok ? undefined : body.error ?? body.data?.message ?? `HTTP ${response.status}`;
    writeLog({ id: `${started}-${Math.random().toString(16).slice(2)}`, at: new Date().toISOString(), action, ok, durationMs: Date.now() - started, request: { action, params: safeParams(params) }, response: redactResponse(body), error: logError });
    logged = true;
    // Do not discard successful chain balances just because another chain failed.
    // The caller will update successful chains and mark only failed chains as errors.
    if (!response.ok || (!partialBalanceResponse && (body.success !== true || body.data?.success === false))) {
      throw new Error(body.error ?? body.data?.message ?? `Engine request failed (HTTP ${response.status})`);
    }
    return body;
  } catch (error) {
    const message = error instanceof Error ? error.message : "Unknown engine request error";
    if (!logged) {
      writeLog({ id: `${started}-${Math.random().toString(16).slice(2)}`, at: new Date().toISOString(), action, ok: false, durationMs: Date.now() - started, request: { action, params: safeParams(params) }, error: message });
    }
    throw error;
  }
}

export function clearEngineLogs() {
  if (typeof window === "undefined") return;
  try { window.localStorage.removeItem(LOG_KEY); } catch { /* storage may be blocked */ }
  try { window.sessionStorage.removeItem(LOG_KEY); } catch { /* storage may be blocked */ }
  window.dispatchEvent(new CustomEvent("sable-engine-log", { detail: [] }));
}

export type SelectedChain = {
  name: string;
  chain_id: string;
  chain_type: string;
  symbol: string;
  network: string;
  derivation_path?: string;
  rpc_provider?: string;
  rpc_url?: string;
};

/** Chain metadata is loaded from the NodeX engine's published catalog. */
export async function fetchBlockchainCatalog(): Promise<SelectedChain[]> {
  const base = getEngineBaseUrl();
  
  let response: Response;
  try { response = await fetch(`${base}/static/blockchains.json`, { headers: { accept: "application/json" } }); }
  catch (error) {
    if (error instanceof TypeError) throw new Error(`Could not load the NodeX blockchain catalog from ${base}. Check engine health and CORS.`);
    throw error;
  }
  if (!response.ok) throw new Error(`NodeX blockchain catalog request failed (HTTP ${response.status}).`);
  const payload = await response.json() as { networks?: unknown };
  if (!Array.isArray(payload.networks)) throw new Error("NodeX blockchain catalog did not contain a networks array.");
  return payload.networks.filter((item): item is SelectedChain & { adapter_supported?: boolean; mainnet?: boolean } =>
    !!item && typeof item === "object"
      && (item as Record<string, unknown>).adapter_supported === true
      && (item as Record<string, unknown>).mainnet === true
      && typeof (item as Record<string, unknown>).name === "string"
      && typeof (item as Record<string, unknown>).chain_id === "string"
      && typeof (item as Record<string, unknown>).chain_type === "string"
      && typeof (item as Record<string, unknown>).symbol === "string"
      && typeof (item as Record<string, unknown>).derivation_path === "string"
      && typeof (item as Record<string, unknown>).rpc_provider === "string"
      && !!String((item as Record<string, unknown>).rpc_provider).trim()
      && typeof (item as Record<string, unknown>).rpc_url === "string"
      && (String((item as Record<string, unknown>).rpc_url).trim().startsWith("https://") || String((item as Record<string, unknown>).rpc_url).trim().startsWith("http://"))
  ).map((item) => ({ name: item.name, chain_id: item.chain_id, chain_type: item.chain_type, symbol: item.symbol, network: "mainnet", derivation_path: item.derivation_path, rpc_provider: item.rpc_provider, rpc_url: item.rpc_url }));
}

export function walletContext(raw: unknown): Record<string, unknown> {
  if (!raw || typeof raw !== "object") return {};
  const value = raw as Record<string, unknown>;
  for (const key of ["raw", "wallet", "record"]) {
    const nested = value[key];
    if (nested && typeof nested === "object" && !Array.isArray(nested)) {
      const candidate = nested as Record<string, unknown>;
      if (candidate.wallet_id || candidate.wallet_group_id || candidate.blockchains || candidate.chains || candidate.balances || candidate.errors || candidate.status === "partial_success") return candidate;
      const unwrapped = walletContext(candidate);
      if (Object.keys(unwrapped).length) return unwrapped;
    }
  }
  return value;
}

export function chainRecords(raw: unknown): Array<Record<string, unknown>> {
  const wallet = walletContext(raw);
  const candidates = wallet.blockchains ?? wallet.chains ?? wallet.matching_blockchains;
  if (!Array.isArray(candidates)) return [];
  return candidates.filter((item): item is Record<string, unknown> => !!item && typeof item === "object");
}
