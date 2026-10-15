import { ASSETS, type AssetId, type NetworkId } from "@/lib/wallet/assets";
import type { PricePoint } from "@/lib/wallet/prices";
import type { Transaction } from "@/lib/wallet/store";
import type { Addresses } from "@/lib/wallet/mnemonic";

export type HoldingRow = {
  id: AssetId;
  amount: number;
  price: number;
  change: number;
  value: number;
};

export function holdingRows(
  holdings: Record<AssetId, number>,
  prices: Record<AssetId, PricePoint>,
  network: NetworkId,
): HoldingRow[] {
  return (Object.keys(holdings) as AssetId[])
    .map((id) => {
      const amount = holdings[id] ?? 0;
      const price = prices[id]?.usd ?? 0;
      return {
        id,
        amount,
        price,
        change: prices[id]?.usd_24h_change ?? 0,
        value: amount * price,
      };
    })
    .filter((row) => {
      if (row.amount <= 0) return false;
      if (network === "all") return true;
      if (network === "base") return ASSETS[row.id].network === "ethereum";
      return ASSETS[row.id].network === network;
    })
    .sort((a, b) => b.value - a.value);
}

export function portfolioTotals(rows: HoldingRow[]) {
  const total = rows.reduce((s, r) => s + r.value, 0);
  const prev = rows.reduce((s, r) => {
    const denom = 1 + r.change / 100;
    return s + (denom === 0 ? r.value : r.value / denom);
  }, 0);
  const delta = total - prev;
  const pct = prev === 0 ? 0 : (delta / prev) * 100;
  return { total, delta, pct };
}

export function addressForNetwork(
  addresses: Addresses,
  network: NetworkId,
) {
  if (network === "solana") return addresses.sol;
  if (network === "bitcoin") return addresses.btc;
  return addresses.evm;
}

export function groupTxs(txs: Transaction[]) {
  const groups: { key: string; items: Transaction[] }[] = [];
  const map = new Map<string, Transaction[]>();
  for (const tx of txs) {
    const d = new Date(tx.timestamp);
    const key = `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
    const list = map.get(key) ?? [];
    list.push(tx);
    map.set(key, list);
  }
  for (const [key, items] of map) {
    groups.push({ key, items });
  }
  return groups;
}

export function isLikelyAddress(value: string, network: NetworkId) {
  const v = value.trim();
  if (network === "solana") return v.length >= 32 && v.length <= 44;
  if (network === "bitcoin") return /^(bc1|[13])[a-zA-HJ-NP-Z0-9]{24,}$/.test(v);
  return /^0x[a-fA-F0-9]{40}$/.test(v);
}
