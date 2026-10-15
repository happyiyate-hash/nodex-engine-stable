import { n as clsx } from "../_libs/class-variance-authority+clsx.mjs";
import { t as twMerge } from "../_libs/tailwind-merge.mjs";
import { n as create, t as persist } from "../_libs/zustand.mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/store-CTJXUQRt.js
function cn(...inputs) {
	return twMerge(clsx(inputs));
}
function formatUsd(value, opts) {
	if (!Number.isFinite(value)) return "$0.00";
	if (opts?.compact && Math.abs(value) >= 1e4) return new Intl.NumberFormat("en-US", {
		style: "currency",
		currency: "USD",
		notation: "compact",
		maximumFractionDigits: 2
	}).format(value);
	const digits = opts?.digits ?? (Math.abs(value) < 1 && value !== 0 ? 4 : 2);
	return new Intl.NumberFormat("en-US", {
		style: "currency",
		currency: "USD",
		minimumFractionDigits: digits,
		maximumFractionDigits: digits
	}).format(value);
}
function formatAmount(value, symbol) {
	if (!Number.isFinite(value)) return `0 ${symbol}`;
	const abs = Math.abs(value);
	const digits = abs === 0 ? 2 : abs < .001 ? 6 : abs < 1 ? 4 : abs < 100 ? 4 : 2;
	return `${new Intl.NumberFormat("en-US", {
		minimumFractionDigits: 0,
		maximumFractionDigits: digits
	}).format(value)} ${symbol}`;
}
function formatTokenAmount(value) {
	if (!Number.isFinite(value)) return "0";
	const abs = Math.abs(value);
	const digits = abs === 0 ? 2 : abs < .001 ? 6 : abs < 1 ? 4 : abs < 1e3 ? 4 : 2;
	return new Intl.NumberFormat("en-US", {
		minimumFractionDigits: 0,
		maximumFractionDigits: digits
	}).format(value);
}
function truncateAddress(address, size = 4) {
	if (address.length <= size * 2 + 2) return address;
	return `${address.slice(0, size + (address.startsWith("0x") ? 2 : 0))}…${address.slice(-size)}`;
}
function formatRelative(ts) {
	const delta = Date.now() - ts;
	const min = Math.round(delta / 6e4);
	if (min < 1) return "Just now";
	if (min < 60) return `${min}m ago`;
	const hr = Math.round(min / 60);
	if (hr < 24) return `${hr}h ago`;
	const day = Math.round(hr / 24);
	if (day < 14) return `${day}d ago`;
	return new Intl.DateTimeFormat("en-US", {
		month: "short",
		day: "numeric"
	}).format(new Date(ts));
}
function formatDateHeading(ts) {
	const d = new Date(ts);
	const today = /* @__PURE__ */ new Date();
	const sameDay = (a, b) => a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
	if (sameDay(d, today)) return "Today";
	const yest = new Date(today);
	yest.setDate(today.getDate() - 1);
	if (sameDay(d, yest)) return "Yesterday";
	return new Intl.DateTimeFormat("en-US", {
		month: "long",
		day: "numeric",
		year: d.getFullYear() === today.getFullYear() ? void 0 : "numeric"
	}).format(d);
}
function randomHex(bytes) {
	const arr = crypto.getRandomValues(new Uint8Array(bytes));
	return Array.from(arr, (b) => b.toString(16).padStart(2, "0")).join("");
}
async function sha256Hex(input) {
	const data = typeof input === "string" ? new TextEncoder().encode(input) : input;
	const digest = await crypto.subtle.digest("SHA-256", data);
	return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}
async function copyText(value) {
	await navigator.clipboard.writeText(value);
}
var ASSETS = {
	ethereum: {
		id: "ethereum",
		name: "Ether",
		symbol: "ETH",
		network: "ethereum",
		networkLabel: "Ethereum",
		decimals: 6,
		coingeckoId: "ethereum"
	},
	bitcoin: {
		id: "bitcoin",
		name: "Bitcoin",
		symbol: "BTC",
		network: "bitcoin",
		networkLabel: "Bitcoin",
		decimals: 8,
		coingeckoId: "bitcoin"
	},
	"usd-coin": {
		id: "usd-coin",
		name: "USD Coin",
		symbol: "USDC",
		network: "ethereum",
		networkLabel: "Ethereum",
		decimals: 2,
		coingeckoId: "usd-coin"
	},
	solana: {
		id: "solana",
		name: "Solana",
		symbol: "SOL",
		network: "solana",
		networkLabel: "Solana",
		decimals: 4,
		coingeckoId: "solana"
	},
	chainlink: {
		id: "chainlink",
		name: "Chainlink",
		symbol: "LINK",
		network: "ethereum",
		networkLabel: "Ethereum",
		decimals: 4,
		coingeckoId: "chainlink"
	},
	uniswap: {
		id: "uniswap",
		name: "Uniswap",
		symbol: "UNI",
		network: "ethereum",
		networkLabel: "Ethereum",
		decimals: 4,
		coingeckoId: "uniswap"
	}
};
var ASSET_LIST = Object.values(ASSETS);
var NETWORKS = [
	{
		id: "all",
		label: "All networks",
		short: "All"
	},
	{
		id: "ethereum",
		label: "Ethereum",
		short: "ETH"
	},
	{
		id: "base",
		label: "Base",
		short: "Base"
	},
	{
		id: "solana",
		label: "Solana",
		short: "SOL"
	},
	{
		id: "bitcoin",
		label: "Bitcoin",
		short: "BTC"
	}
];
var FALLBACK_PRICES = {
	ethereum: {
		usd: 3524.18,
		usd_24h_change: 1.84
	},
	bitcoin: {
		usd: 97240,
		usd_24h_change: .62
	},
	"usd-coin": {
		usd: 1,
		usd_24h_change: .01
	},
	solana: {
		usd: 178.42,
		usd_24h_change: -1.12
	},
	chainlink: {
		usd: 18.36,
		usd_24h_change: 2.41
	},
	uniswap: {
		usd: 8.94,
		usd_24h_change: -.48
	}
};
var DEFAULT_HOLDINGS = {
	ethereum: 1.8421,
	bitcoin: .0614,
	"usd-coin": 2840,
	solana: 18.2,
	chainlink: 42,
	uniswap: 65.4
};
var COLLECTIBLES = [
	{
		id: "study-01",
		name: "Study 01",
		collection: "Monolith",
		seed: 11
	},
	{
		id: "orbit-7",
		name: "Orbit 7",
		collection: "Monolith",
		seed: 27
	},
	{
		id: "field-a",
		name: "Field A",
		collection: "Index",
		seed: 44
	},
	{
		id: "grid-12",
		name: "Grid 12",
		collection: "Index",
		seed: 62
	}
];
var WORDS = [
	"abandon",
	"ability",
	"able",
	"about",
	"above",
	"absent",
	"absorb",
	"abstract",
	"absurd",
	"abuse",
	"access",
	"accident",
	"account",
	"accuse",
	"achieve",
	"acid",
	"acoustic",
	"acquire",
	"across",
	"act",
	"action",
	"actor",
	"actress",
	"actual",
	"adapt",
	"add",
	"addict",
	"address",
	"adjust",
	"admit",
	"adult",
	"advance",
	"advice",
	"aerobic",
	"affair",
	"afford",
	"afraid",
	"again",
	"age",
	"agent",
	"agree",
	"ahead",
	"aim",
	"air",
	"airport",
	"aisle",
	"alarm",
	"album",
	"alcohol",
	"alert",
	"alien",
	"all",
	"alley",
	"allow",
	"almost",
	"alone",
	"alpha",
	"already",
	"also",
	"alter",
	"always",
	"amateur",
	"amazing",
	"among",
	"amount",
	"amused",
	"analyst",
	"anchor",
	"ancient",
	"anger",
	"angle",
	"angry",
	"animal",
	"ankle",
	"announce",
	"annual",
	"another",
	"answer",
	"antenna",
	"antique",
	"anxiety",
	"any",
	"apart",
	"apology",
	"appear",
	"apple",
	"approve",
	"april",
	"arch",
	"arctic",
	"area",
	"arena",
	"argue",
	"arm",
	"armed",
	"armor",
	"army",
	"around",
	"arrange",
	"arrest",
	"arrive",
	"arrow",
	"art",
	"artefact",
	"artist",
	"artwork",
	"ask",
	"aspect",
	"assault",
	"asset",
	"assist",
	"assume",
	"asthma",
	"athlete",
	"atom",
	"attack",
	"attend",
	"attitude",
	"attract",
	"auction",
	"audit",
	"august",
	"aunt",
	"author",
	"auto",
	"autumn",
	"average",
	"avocado",
	"avoid",
	"awake",
	"aware",
	"away",
	"awesome",
	"awful",
	"awkward",
	"axis",
	"baby",
	"bachelor",
	"bacon",
	"badge",
	"bag",
	"balance",
	"balcony",
	"ball",
	"bamboo",
	"banana",
	"banner",
	"bar",
	"barely",
	"bargain",
	"barrel",
	"base",
	"basic",
	"basket",
	"battle",
	"beach",
	"bean",
	"beauty",
	"because",
	"become",
	"beef",
	"before",
	"begin",
	"behave",
	"behind",
	"believe",
	"below",
	"belt",
	"bench",
	"benefit",
	"best",
	"betray",
	"better",
	"between",
	"beyond",
	"bicycle",
	"bid",
	"bike",
	"bind",
	"biology",
	"bird",
	"birth",
	"bitter",
	"black",
	"blade",
	"blame",
	"blanket",
	"blast",
	"bleak",
	"bless",
	"blind",
	"blood",
	"blossom",
	"blouse",
	"blue",
	"blur",
	"blush",
	"board",
	"boat",
	"body",
	"boil",
	"bomb",
	"bone",
	"bonus",
	"book",
	"boost",
	"border",
	"boring",
	"borrow",
	"boss",
	"bottom",
	"bounce",
	"box",
	"boy",
	"bracket",
	"brain",
	"brand",
	"brass",
	"brave",
	"bread",
	"breeze",
	"brick",
	"bridge",
	"brief",
	"bright",
	"bring",
	"brisk",
	"broccoli",
	"broken",
	"bronze",
	"broom",
	"brother",
	"brown",
	"brush",
	"bubble",
	"buddy",
	"budget",
	"buffalo",
	"build",
	"bulb",
	"bulk",
	"bullet",
	"bundle",
	"bunker",
	"burden",
	"burger",
	"burst",
	"bus"
];
var BECH32 = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
var BASE58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
function bytesFromHex(hex) {
	const clean = hex.length % 2 ? `0${hex}` : hex;
	const out = new Uint8Array(clean.length / 2);
	for (let i = 0; i < out.length; i++) out[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
	return out;
}
function toBase58(bytes) {
	let n = 0n;
	for (const b of bytes) n = (n << 8n) + BigInt(b);
	let out = "";
	while (n > 0n) {
		out = BASE58[Number(n % 58n)] + out;
		n /= 58n;
	}
	for (const b of bytes) {
		if (b !== 0) break;
		out = "1" + out;
	}
	return out.padStart(44, "1").slice(0, 44);
}
function generateMnemonic() {
	const rand = crypto.getRandomValues(/* @__PURE__ */ new Uint32Array(12));
	return Array.from(rand, (n) => WORDS[n % WORDS.length]).join(" ");
}
function normalizeMnemonic(phrase) {
	return phrase.trim().toLowerCase().split(/\s+/).filter(Boolean).join(" ");
}
function isValidMnemonic(phrase) {
	const words = normalizeMnemonic(phrase).split(" ");
	return words.length === 12 && words.every((w) => WORDS.includes(w));
}
async function deriveAddresses(phrase) {
	const h1 = await sha256Hex(normalizeMnemonic(phrase));
	const h2 = await sha256Hex(h1);
	const b1 = bytesFromHex(h1);
	const b2 = bytesFromHex(h2);
	let btc = "bc1q";
	for (let i = 0; i < 38; i++) btc += BECH32[b1[i % b1.length] % 32];
	return {
		evm: `0x${h1.slice(0, 40)}`,
		sol: toBase58(b2),
		btc
	};
}
async function hashPin(pin, salt) {
	return sha256Hex(`${salt}:${pin}`);
}
function seedTxs(now) {
	return [
		{
			id: "tx-seed-1",
			type: "receive",
			status: "confirmed",
			assetId: "ethereum",
			amount: .5,
			counterparty: "0x28c6d61185d90b2866998c15dc9793b3eb0a47d0",
			hash: `0x${randomHex(32)}`,
			timestamp: now - 252e4
		},
		{
			id: "tx-seed-2",
			type: "swap",
			status: "confirmed",
			assetId: "usd-coin",
			amount: 400,
			toAssetId: "solana",
			toAmount: 2.24,
			hash: `0x${randomHex(32)}`,
			timestamp: now - 648e5
		},
		{
			id: "tx-seed-3",
			type: "send",
			status: "confirmed",
			assetId: "usd-coin",
			amount: 250,
			counterparty: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
			hash: `0x${randomHex(32)}`,
			timestamp: now - 144e6
		},
		{
			id: "tx-seed-4",
			type: "receive",
			status: "confirmed",
			assetId: "bitcoin",
			amount: .02,
			counterparty: "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
			hash: `0x${randomHex(32)}`,
			timestamp: now - 432e6
		}
	];
}
var useWallet = create()(persist((set, get) => ({
	wallet: null,
	holdings: { ...DEFAULT_HOLDINGS },
	txs: [],
	locked: false,
	hideBalances: false,
	network: "all",
	hydrated: false,
	setHydrated: () => set({ hydrated: true }),
	createWallet: async (name) => {
		const phrase = generateMnemonic();
		const addresses = await deriveAddresses(phrase);
		const now = Date.now();
		const wallet = {
			name: name.trim() || "Primary",
			phrase,
			addresses,
			createdAt: now,
			pinHash: null,
			pinSalt: null
		};
		set({
			wallet,
			holdings: { ...DEFAULT_HOLDINGS },
			txs: seedTxs(now),
			locked: false
		});
		return wallet;
	},
	restoreWallet: async (phrase, name) => {
		const normalized = normalizeMnemonic(phrase);
		if (!isValidMnemonic(normalized)) throw new Error("Enter 12 valid recovery words.");
		const addresses = await deriveAddresses(normalized);
		const now = Date.now();
		const wallet = {
			name: name.trim() || "Restored",
			phrase: normalized,
			addresses,
			createdAt: now,
			pinHash: null,
			pinSalt: null
		};
		set({
			wallet,
			holdings: { ...DEFAULT_HOLDINGS },
			txs: seedTxs(now),
			locked: false
		});
		return wallet;
	},
	setPin: async (pin) => {
		const wallet = get().wallet;
		if (!wallet) return;
		const pinSalt = randomHex(16);
		const pinHash = await hashPin(pin, pinSalt);
		set({ wallet: {
			...wallet,
			pinHash,
			pinSalt
		} });
	},
	unlock: async (pin) => {
		const wallet = get().wallet;
		if (!wallet?.pinHash || !wallet.pinSalt) {
			set({ locked: false });
			return true;
		}
		if (await hashPin(pin, wallet.pinSalt) !== wallet.pinHash) return false;
		set({ locked: false });
		return true;
	},
	lock: () => {
		if (get().wallet?.pinHash) set({ locked: true });
	},
	send: ({ assetId, amount, to }) => {
		const holdings = { ...get().holdings };
		const available = holdings[assetId] ?? 0;
		if (amount <= 0 || amount > available) throw new Error("Insufficient balance.");
		holdings[assetId] = available - amount;
		const tx = {
			id: `tx-${randomHex(8)}`,
			type: "send",
			status: "confirmed",
			assetId,
			amount,
			counterparty: to,
			hash: `0x${randomHex(32)}`,
			timestamp: Date.now()
		};
		set({
			holdings,
			txs: [tx, ...get().txs]
		});
		return tx;
	},
	swap: ({ fromId, toId, fromAmount, toAmount }) => {
		const holdings = { ...get().holdings };
		const available = holdings[fromId] ?? 0;
		if (fromAmount <= 0 || fromAmount > available) throw new Error("Insufficient balance.");
		holdings[fromId] = available - fromAmount;
		holdings[toId] = (holdings[toId] ?? 0) + toAmount;
		const tx = {
			id: `tx-${randomHex(8)}`,
			type: "swap",
			status: "confirmed",
			assetId: fromId,
			amount: fromAmount,
			toAssetId: toId,
			toAmount,
			hash: `0x${randomHex(32)}`,
			timestamp: Date.now()
		};
		set({
			holdings,
			txs: [tx, ...get().txs]
		});
		return tx;
	},
	toggleHideBalances: () => set((s) => ({ hideBalances: !s.hideBalances })),
	setNetwork: (network) => set({ network }),
	rename: (name) => {
		const wallet = get().wallet;
		if (!wallet) return;
		set({ wallet: {
			...wallet,
			name: name.trim() || wallet.name
		} });
	},
	reset: () => set({
		wallet: null,
		holdings: { ...DEFAULT_HOLDINGS },
		txs: [],
		locked: false,
		hideBalances: false,
		network: "all"
	})
}), {
	name: "sable-wallet",
	skipHydration: true,
	partialize: (s) => ({
		wallet: s.wallet,
		holdings: s.holdings,
		txs: s.txs,
		hideBalances: s.hideBalances,
		network: s.network,
		locked: s.wallet?.pinHash ? true : false
	}),
	onRehydrateStorage: () => (state) => {
		state?.setHydrated();
		if (state?.wallet?.pinHash) state.lock();
	}
}));
//#endregion
export { NETWORKS as a, formatAmount as c, formatTokenAmount as d, formatUsd as f, useWallet as h, FALLBACK_PRICES as i, formatDateHeading as l, truncateAddress as m, ASSET_LIST as n, cn as o, isValidMnemonic as p, COLLECTIBLES as r, copyText as s, ASSETS as t, formatRelative as u };
