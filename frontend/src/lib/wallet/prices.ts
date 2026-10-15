import { create } from "zustand";
import {
  ASSET_LIST,
  FALLBACK_PRICES,
  type AssetId,
} from "@/lib/wallet/assets";

export type PricePoint = { usd: number; usd_24h_change: number };

type PriceState = {
  prices: Record<AssetId, PricePoint>;
  updatedAt: number;
  status: "idle" | "live" | "fallback";
  refresh: () => Promise<void>;
};

const IDS = ASSET_LIST.map((a) => a.coingeckoId).join(",");

export const usePrices = create<PriceState>((set, get) => ({
  prices: FALLBACK_PRICES,
  updatedAt: 0,
  status: "idle",
  refresh: async () => {
    try {
      const res = await fetch(
        `https://api.coingecko.com/api/v3/simple/price?ids=${IDS}&vs_currencies=usd&include_24hr_change=true`,
      );
      if (!res.ok) throw new Error("price http");
      const json = (await res.json()) as Record<
        string,
        { usd: number; usd_24h_change?: number }
      >;
      const next = { ...FALLBACK_PRICES };
      for (const asset of ASSET_LIST) {
        const row = json[asset.coingeckoId];
        if (row && typeof row.usd === "number") {
          next[asset.id] = {
            usd: row.usd,
            usd_24h_change: row.usd_24h_change ?? 0,
          };
        }
      }
      set({ prices: next, updatedAt: Date.now(), status: "live" });
    } catch {
      if (get().status === "idle") {
        set({ status: "fallback", updatedAt: Date.now() });
      }
    }
  },
}));

export function sparklinePath(
  seed: string,
  end: number,
  changePct: number,
  points = 24,
) {
  const start = end / (1 + changePct / 100);
  let hash = 0;
  for (let i = 0; i < seed.length; i++) {
    hash = (hash * 31 + seed.charCodeAt(i)) | 0;
  }
  const values: number[] = [];
  for (let i = 0; i < points; i++) {
    hash = (hash * 1664525 + 1013904223) | 0;
    const t = i / (points - 1);
    const drift = start + (end - start) * t;
    const noise = ((hash % 1000) / 1000 - 0.5) * (end * 0.018);
    values.push(Math.max(0, drift + noise));
  }
  values[values.length - 1] = end;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const coords = values.map((v, i) => {
    const x = (i / (points - 1)) * 100;
    const y = 32 - ((v - min) / span) * 28 - 2;
    return [x, y] as const;
  });
  return coords
    .map(([x, y], i) => `${i === 0 ? "M" : "L"}${x.toFixed(2)} ${y.toFixed(2)}`)
    .join(" ");
}
