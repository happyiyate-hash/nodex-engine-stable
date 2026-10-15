import { sha256Hex } from "@/lib/utils";

// Addresses are intentionally not derived in the browser. NodeX's registered
// wallet response is the only authoritative address source.
export type Addresses = {
  evm: string;
  sol: string;
  btc: string;
  byChain?: Record<string, string>;
};

export function normalizeMnemonic(phrase: string) {
  return phrase.trim().toLowerCase().split(/\s+/).filter(Boolean).join(" ");
}

export async function hashPin(pin: string, salt: string) {
  return sha256Hex(`${salt}:${pin}`);
}
