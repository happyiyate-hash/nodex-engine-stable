import { n as create } from "../_libs/zustand.mjs";
import { i as FALLBACK_PRICES, n as ASSET_LIST } from "./store-CTJXUQRt.mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/prices-Dmpyucw2.js
var IDS = ASSET_LIST.map((a) => a.coingeckoId).join(",");
var usePrices = create((set, get) => ({
	prices: FALLBACK_PRICES,
	updatedAt: 0,
	status: "idle",
	refresh: async () => {
		try {
			const res = await fetch(`https://api.coingecko.com/api/v3/simple/price?ids=${IDS}&vs_currencies=usd&include_24hr_change=true`);
			if (!res.ok) throw new Error("price http");
			const json = await res.json();
			const next = { ...FALLBACK_PRICES };
			for (const asset of ASSET_LIST) {
				const row = json[asset.coingeckoId];
				if (row && typeof row.usd === "number") next[asset.id] = {
					usd: row.usd,
					usd_24h_change: row.usd_24h_change ?? 0
				};
			}
			set({
				prices: next,
				updatedAt: Date.now(),
				status: "live"
			});
		} catch {
			if (get().status === "idle") set({
				status: "fallback",
				updatedAt: Date.now()
			});
		}
	}
}));
function sparklinePath(seed, end, changePct, points = 24) {
	const start = end / (1 + changePct / 100);
	let hash = 0;
	for (let i = 0; i < seed.length; i++) hash = hash * 31 + seed.charCodeAt(i) | 0;
	const values = [];
	for (let i = 0; i < points; i++) {
		hash = hash * 1664525 + 1013904223 | 0;
		const t = i / (points - 1);
		const drift = start + (end - start) * t;
		const noise = (hash % 1e3 / 1e3 - .5) * (end * .018);
		values.push(Math.max(0, drift + noise));
	}
	values[values.length - 1] = end;
	const min = Math.min(...values);
	const span = Math.max(...values) - min || 1;
	return values.map((v, i) => {
		return [i / (points - 1) * 100, 32 - (v - min) / span * 28 - 2];
	}).map(([x, y], i) => `${i === 0 ? "M" : "L"}${x.toFixed(2)} ${y.toFixed(2)}`).join(" ");
}
//#endregion
export { usePrices as n, sparklinePath as t };
