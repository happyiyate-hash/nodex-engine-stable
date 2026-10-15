import { i as __toESM } from "../_runtime.mjs";
import { u as require_react } from "../_libs/@floating-ui/react-dom+[...].mjs";
import { c as formatAmount, h as useWallet, l as formatDateHeading, m as truncateAddress, o as cn, t as ASSETS, u as formatRelative } from "./store-CTJXUQRt.mjs";
import { O as require_jsx_runtime } from "../_libs/@radix-ui/react-alert-dialog+[...].mjs";
import { S as useRouter, _ as lazyRouteComponent, d as Scripts, f as HeadContent, g as Outlet, h as createRouter, v as createFileRoute, y as createRootRoute } from "../_libs/@tanstack/react-router+[...].mjs";
import { f as ArrowUpRight, g as ArrowDownLeft, m as ArrowLeftRight, t as TriangleAlert } from "../_libs/lucide-react.mjs";
import { t as Toaster } from "../_libs/sonner.mjs";
import { a as union, i as string, n as number, r as object, t as literal } from "../_libs/zod.mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/router-FskFLzdY.js
var import_react = /* @__PURE__ */ __toESM(require_react());
var import_jsx_runtime = require_jsx_runtime();
var __defProp = Object.defineProperty;
var __exportAll = (all, no_symbols) => {
	let target = {};
	for (var name in all) __defProp(target, name, {
		get: all[name],
		enumerable: true
	});
	if (!no_symbols) __defProp(target, Symbol.toStringTag, { value: "Module" });
	return target;
};
var FALLBACK_MESSAGE = "An unexpected error occurred. Try reloading the page.";
function errorMessage(error) {
	if (error instanceof Error && error.message) return error.message;
	if (typeof error === "string" && error) return error;
	return FALLBACK_MESSAGE;
}
function AppErrorComponent({ error }) {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("main", {
		className: "flex min-h-screen flex-col items-center justify-center gap-3 px-6 text-center bg-zinc-50 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-50",
		children: [
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
				className: "text-red-500",
				"aria-hidden": "true",
				children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(TriangleAlert, {
					className: "size-10",
					strokeWidth: 2
				})
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)("h1", {
				className: "text-lg font-semibold",
				children: "Something went wrong"
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
				className: "max-w-md text-sm break-words text-zinc-500 dark:text-zinc-400",
				children: errorMessage(error)
			})
		]
	});
}
/**
* App-wide client provider mounted once near the root (in `src/routes/__root.tsx`):
*
*   <AuthProvider><Outlet /></AuthProvider>
*
* Better Auth's React client (`@/lib/auth/client`) needs NO context provider —
* its `useSession()` works standalone — so this is a passthrough today. It's
* kept as the single, stable mount point for any future client-side providers
* (e.g. a toast or theme provider) without churning the root shell.
*/
function AuthProvider({ children }) {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsx)(import_jsx_runtime.Fragment, { children });
}
var CONNECTOR_TOKEN_READY_EVENT = "grok:connector-token-ready";
function isGrokEmbedderOrigin(origin) {
	try {
		const url = new URL(origin);
		if (url.protocol !== "https:" && url.protocol !== "http:") return false;
		const host = url.hostname.toLowerCase();
		if (host === "grok.com" || host.endsWith(".grok.com")) return true;
		if (host === "localhost" || host === "127.0.0.1" || host === "[::1]") return true;
		return false;
	} catch {
		return false;
	}
}
function isSandboxPreviewGuestHost(hostname) {
	const host = hostname.toLowerCase();
	return host === "grok-sandbox.com" || host.endsWith(".grok-sandbox.com");
}
function isRemintPreviewPair(guestHost, parentHost) {
	const guest = guestHost.toLowerCase();
	const parent = parentHost.toLowerCase();
	const i = guest.indexOf(".preview.");
	if (i <= 0) return false;
	const label = guest.slice(0, i);
	const rest = guest.slice(i + 9);
	if (label.includes(".") || !rest.includes(".")) return false;
	return parent === rest || parent === `grok.${rest}`;
}
function resolveParentEmbedderOrigin(parentIsSelf, referrer, ancestorOrigin, guestHostname = "") {
	if (parentIsSelf) return null;
	for (const candidate of [referrer, ancestorOrigin ?? ""].filter(Boolean)) try {
		const url = new URL(candidate.includes("://") ? candidate : `https://${candidate}`);
		if (url.protocol !== "https:" && url.protocol !== "http:") continue;
		if (isGrokEmbedderOrigin(url.origin)) return url.origin;
		if (isSandboxPreviewGuestHost(guestHostname) || isRemintPreviewPair(guestHostname, url.hostname)) return url.origin;
	} catch {}
	return null;
}
/**
* Guest side of the grok-web ↔ sandbox preview postMessage bridge.
*
* Activates only when this page is framed by an allowlisted Grok embedder.
* Top-level runs (download/export, local `npm run dev`, deployed sites) noop.
*/
var PREVIEW_BRIDGE_CHANNEL = "grok-preview-bridge";
var EnvelopeSchema = object({
	channel: literal(PREVIEW_BRIDGE_CHANNEL),
	version: number().int().positive(),
	type: string().min(1)
});
var HelloSchema = EnvelopeSchema.extend({ type: literal("hello") });
var NavigateSchema = EnvelopeSchema.extend({
	type: literal("navigate"),
	path: string().min(1)
});
var HistorySchema = EnvelopeSchema.extend({
	type: literal("history"),
	delta: union([literal(-1), literal(1)])
});
var ConnectorTokenReadySchema = EnvelopeSchema.extend({ type: literal("connector-token-ready") });
function isSafeBridgePath(path) {
	if (!path.startsWith("/") || path.startsWith("//") || path.includes("\\")) return false;
	try {
		return new URL(path, "https://preview.invalid").origin === "https://preview.invalid";
	} catch {
		return false;
	}
}
/**
* Origin of the Grok embedder framing this page, or null when the page runs
* top-level (download/export, local `npm run dev`, deployed sites) or under a
* non-Grok parent. Client-only; null during SSR.
*/
function resolveCurrentEmbedderOrigin() {
	if (typeof window === "undefined") return null;
	const ancestorOrigin = typeof location.ancestorOrigins !== "undefined" && location.ancestorOrigins.length > 0 ? location.ancestorOrigins[0] : null;
	return resolveParentEmbedderOrigin(window.parent === window, document.referrer, ancestorOrigin, window.location.hostname);
}
/**
* Install host↔guest messaging. Returns a dispose function.
* Noops (returns a no-op dispose) when not embedded under a Grok parent.
*/
function installPreviewHostBridge(options = {}) {
	const parentOrigin = resolveCurrentEmbedderOrigin();
	if (parentOrigin === null) return () => {};
	const ROOT_STATE_KEY = "__grokPreviewBridgeRoot";
	const originalPushState = window.history.pushState.bind(window.history);
	const originalReplaceState = window.history.replaceState.bind(window.history);
	const isAtHistoryRoot = () => {
		const state = window.history.state;
		return Boolean(state && typeof state === "object" && state[ROOT_STATE_KEY] === true);
	};
	try {
		const current = window.history.state;
		if (!(current !== null && typeof current === "object" && Object.prototype.hasOwnProperty.call(current, ROOT_STATE_KEY))) {
			const isRoot = window.history.length <= 1;
			originalReplaceState(current && typeof current === "object" ? {
				...current,
				[ROOT_STATE_KEY]: isRoot
			} : { [ROOT_STATE_KEY]: isRoot }, "", window.location.href);
		}
	} catch {}
	const post = (message) => {
		window.parent.postMessage(message, parentOrigin);
	};
	const reportLocation = () => {
		post({
			channel: PREVIEW_BRIDGE_CHANNEL,
			version: 1,
			type: "location",
			path: window.location.pathname || "/",
			search: window.location.search,
			hash: window.location.hash
		});
	};
	const reportRoutes = () => {
		const paths = options.getRoutePaths?.() ?? [];
		post({
			channel: PREVIEW_BRIDGE_CHANNEL,
			version: 1,
			type: "routes",
			paths
		});
	};
	const defaultNavigate = (path) => {
		if (!isSafeBridgePath(path)) return;
		try {
			const url = new URL(path, window.location.origin);
			if (url.origin !== window.location.origin) return;
			const next = `${url.pathname}${url.search}${url.hash}`;
			window.history.pushState(window.history.state, "", next);
			window.dispatchEvent(new PopStateEvent("popstate", { state: window.history.state }));
		} catch {}
	};
	const navigate = (path) => {
		if (!isSafeBridgePath(path)) return;
		if (options.navigate) {
			options.navigate(path);
			return;
		}
		defaultNavigate(path);
	};
	const announce = () => {
		reportLocation();
		reportRoutes();
		post({
			channel: PREVIEW_BRIDGE_CHANNEL,
			version: 1,
			type: "ready"
		});
	};
	const onHello = (data) => {
		if (!HelloSchema.safeParse(data).success) return;
		announce();
	};
	const onNavigate = (data) => {
		const parsed = NavigateSchema.safeParse(data);
		if (!parsed.success) return;
		navigate(parsed.data.path);
		queueMicrotask(reportLocation);
	};
	const onHistory = (data) => {
		const parsed = HistorySchema.safeParse(data);
		if (!parsed.success) return;
		if (parsed.data.delta === -1 && isAtHistoryRoot()) return;
		window.history.go(parsed.data.delta);
	};
	const onConnectorTokenReady = (data) => {
		if (!ConnectorTokenReadySchema.safeParse(data).success) return;
		window.dispatchEvent(new Event(CONNECTOR_TOKEN_READY_EVENT));
	};
	const hostMessageHandlers = /* @__PURE__ */ new Map([
		["hello", onHello],
		["navigate", onNavigate],
		["history", onHistory],
		["connector-token-ready", onConnectorTokenReady]
	]);
	const onMessage = (event) => {
		if (event.source !== window.parent) return;
		if (event.origin !== parentOrigin) return;
		const envelope = EnvelopeSchema.safeParse(event.data);
		if (!envelope.success || envelope.data.version !== 1) return;
		hostMessageHandlers.get(envelope.data.type)?.(event.data);
	};
	const onPopState = () => {
		reportLocation();
	};
	const onHashChange = () => {
		reportLocation();
	};
	window.history.pushState = (data, unused, url) => {
		const next = data && typeof data === "object" ? {
			...data,
			[ROOT_STATE_KEY]: false
		} : data;
		originalPushState(next, unused, url);
		reportLocation();
	};
	window.history.replaceState = (data, unused, url) => {
		const next = isAtHistoryRoot() ? {
			...data && typeof data === "object" ? data : {},
			[ROOT_STATE_KEY]: true
		} : data;
		originalReplaceState(next, unused, url);
		reportLocation();
	};
	window.addEventListener("message", onMessage);
	window.addEventListener("popstate", onPopState);
	window.addEventListener("hashchange", onHashChange);
	announce();
	return () => {
		window.removeEventListener("message", onMessage);
		window.removeEventListener("popstate", onPopState);
		window.removeEventListener("hashchange", onHashChange);
		window.history.pushState = originalPushState;
		window.history.replaceState = originalReplaceState;
	};
}
/** Collect static path patterns from a TanStack route tree (best-effort). */
function collectRoutePathsFromTree(routeTree) {
	const paths = /* @__PURE__ */ new Set();
	const walk = (node) => {
		if (!node || typeof node !== "object") return;
		const record = node;
		const full = typeof record.fullPath === "string" ? record.fullPath : typeof record.path === "string" ? record.path : null;
		if (full !== null && full !== "") paths.add(full.startsWith("/") ? full : `/${full}`);
		else if (full === "") paths.add("/");
		const children = record.children;
		if (Array.isArray(children)) for (const child of children) walk(child);
		else if (children && typeof children === "object") for (const child of Object.values(children)) walk(child);
	};
	walk(routeTree);
	return [...paths];
}
/**
* Mount once in `__root.tsx` so the Grok preview chrome can drive navigation
* (and later receive registered routes). Noops when the app is not embedded.
*/
function PreviewHostBridge() {
	const router = useRouter();
	(0, import_react.useEffect)(() => {
		return installPreviewHostBridge({
			navigate: (path) => {
				router.history.push(path);
			},
			getRoutePaths: () => collectRoutePathsFromTree(router.routeTree)
		});
	}, [router]);
	return null;
}
function Toaster$1() {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Toaster, {
		theme: "dark",
		position: "top-center",
		toastOptions: { classNames: {
			toast: "bg-card text-foreground shadow-[var(--shadow-border),var(--shadow-float)] border-0",
			title: "text-foreground",
			description: "text-muted-foreground"
		} }
	});
}
var styles_default = "/assets/styles-BvUMBF_U.css";
var APP_NAME = "Sable";
var Route$8 = createRootRoute({
	head: () => ({
		meta: [
			{ charSet: "utf-8" },
			{
				name: "viewport",
				content: "width=device-width, initial-scale=1, viewport-fit=cover"
			},
			{ title: APP_NAME },
			{
				name: "description",
				content: "Sable is a private, monochrome wallet for digital assets."
			},
			{
				name: "theme-color",
				content: "#09090b"
			}
		],
		links: [
			{
				rel: "icon",
				type: "image/svg+xml",
				href: "/favicon.svg"
			},
			{
				rel: "stylesheet",
				href: styles_default
			},
			{
				rel: "manifest",
				href: "/__grok/manifest.webmanifest"
			},
			{
				rel: "apple-touch-icon",
				href: "/__grok/icon-180.png"
			},
			{
				rel: "preconnect",
				href: "https://fonts.googleapis.com"
			},
			{
				rel: "preconnect",
				href: "https://fonts.gstatic.com",
				crossOrigin: "anonymous"
			},
			{
				rel: "stylesheet",
				href: "https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=Instrument+Sans:wght@400;500;600;700&display=swap"
			}
		]
	}),
	component: () => /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("html", {
		lang: "en",
		className: "dark antialiased",
		suppressHydrationWarning: true,
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("head", { children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(HeadContent, {}) }), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("body", { children: [
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)(PreviewHostBridge, {}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)(AuthProvider, { children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Outlet, {}) }),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Toaster$1, {}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Scripts, {})
		] })]
	})
});
var $$splitComponentImporter$6 = () => import("../_app-B6BEWIwF.mjs");
var Route$7 = createFileRoute("/_app")({ component: lazyRouteComponent($$splitComponentImporter$6, "component") });
var $$splitComponentImporter$5 = () => import("../_app-DdegeodJ.mjs");
var Route$6 = createFileRoute("/_app/")({ component: lazyRouteComponent($$splitComponentImporter$5, "component") });
var icons = {
	bitcoin: /* @__PURE__ */ (0, import_jsx_runtime.jsx)("path", {
		d: "M16 5.5A10.5 10.5 0 1 1 5.5 16 10.5 10.5 0 0 1 16 5.5Zm.4 4.2v1.6c1.6.1 2.7.8 2.7 2.3 0 1.2-.7 1.9-1.8 2.1 1.3.3 2.2 1.1 2.2 2.5 0 1.8-1.4 2.7-3.1 2.8v1.6h-1.6v-1.6h-1.1v1.6H12v-1.6h-2.1v-1.4h1.3c.5 0 .8-.3.8-.8v-7.1c0-.5-.3-.8-.8-.8H9.9V10h2.1V8.5h1.6V10h1.1V8.5h1.6ZM14.7 16.6h1.9c.9 0 1.5-.5 1.5-1.3s-.6-1.3-1.5-1.3h-1.9Zm0 1.4v2.5h2.1c1 0 1.7-.5 1.7-1.3s-.7-1.2-1.7-1.2Z",
		fill: "currentColor"
	}),
	ethereum: /* @__PURE__ */ (0, import_jsx_runtime.jsx)("path", {
		d: "M16 5.5 9.5 16.2 16 20l6.5-3.8Zm0 16.1-6.5-3.7L16 26.5l6.5-8.6Z",
		fill: "currentColor"
	}),
	solana: /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(import_jsx_runtime.Fragment, { children: [
		/* @__PURE__ */ (0, import_jsx_runtime.jsx)("path", {
			d: "M10.2 11.2h10.4l-2.1-2.1H8.1Z",
			fill: "currentColor"
		}),
		/* @__PURE__ */ (0, import_jsx_runtime.jsx)("path", {
			d: "M8.1 16.9h10.4l2.1-2.1H10.2Z",
			fill: "currentColor"
		}),
		/* @__PURE__ */ (0, import_jsx_runtime.jsx)("path", {
			d: "M10.2 22.9h10.4l-2.1-2.1H8.1Z",
			fill: "currentColor"
		})
	] }),
	"usd-coin": /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(import_jsx_runtime.Fragment, { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("circle", {
		cx: "16",
		cy: "16",
		r: "8.5",
		fill: "none",
		stroke: "currentColor",
		strokeWidth: "1.6"
	}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("path", {
		d: "M16.7 12.2v-.9h-1.4v.9c-1.4.2-2.3 1-2.3 2.2 0 1.4 1.1 2 2.8 2.3 1.3.2 1.6.5 1.6 1s-.6.8-1.5.8c-.8 0-1.4-.3-1.6-1l-1.5.3c.3 1.3 1.3 2 3.1 2.2v.9h1.4v-.9c1.5-.2 2.4-1.1 2.4-2.3 0-1.4-1.1-2-2.9-2.3-1.2-.2-1.5-.5-1.5-1s.6-.8 1.4-.8c.8 0 1.2.3 1.4.8l1.5-.4c-.3-1.1-1.2-1.8-2.8-2Z",
		fill: "currentColor"
	})] }),
	chainlink: /* @__PURE__ */ (0, import_jsx_runtime.jsx)("path", {
		d: "M16 6.5 24 11v10l-8 4.5L8 21V11Zm0 2.3L10.2 12v8L16 23.2 21.8 20v-8Z",
		fill: "currentColor"
	}),
	uniswap: /* @__PURE__ */ (0, import_jsx_runtime.jsx)("path", {
		d: "M11.2 8.5c2.6 3.4 3.4 7.6 2.2 11.4-.5 1.7-1.4 3.2-1.1 4.1.3.8 1.4.7 2.4.1 2.5-1.4 4.8-4.6 5.8-8.2.8-3.1.4-6.1-1.3-7.6 1.9 1 3.2 3.6 3.2 6.7 0 4.8-3.4 8.9-7.8 9.6-1.8.3-3.2-.3-3.7-1.6-.5-1.3.1-3 1-4.6 1.4-2.5 1.6-5.3.3-7.6-.6-1-1.5-1.8-2.4-2.3Z",
		fill: "currentColor"
	})
};
function TokenIcon({ id, className }) {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
		className: cn("inline-flex size-10 items-center justify-center rounded-full bg-secondary text-foreground shadow-[var(--shadow-border)]", className),
		children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)("svg", {
			viewBox: "0 0 32 32",
			className: "size-[22px]",
			"aria-hidden": true,
			children: icons[id]
		})
	});
}
function holdingRows(holdings, prices, network) {
	return Object.keys(holdings).map((id) => {
		const amount = holdings[id] ?? 0;
		const price = prices[id]?.usd ?? 0;
		return {
			id,
			amount,
			price,
			change: prices[id]?.usd_24h_change ?? 0,
			value: amount * price
		};
	}).filter((row) => {
		if (row.amount <= 0) return false;
		if (network === "all") return true;
		if (network === "base") return ASSETS[row.id].network === "ethereum";
		return ASSETS[row.id].network === network;
	}).sort((a, b) => b.value - a.value);
}
function portfolioTotals(rows) {
	const total = rows.reduce((s, r) => s + r.value, 0);
	const prev = rows.reduce((s, r) => {
		const denom = 1 + r.change / 100;
		return s + (denom === 0 ? r.value : r.value / denom);
	}, 0);
	const delta = total - prev;
	return {
		total,
		delta,
		pct: prev === 0 ? 0 : delta / prev * 100
	};
}
function addressForNetwork(addresses, network) {
	if (network === "solana") return addresses.sol;
	if (network === "bitcoin") return addresses.btc;
	return addresses.evm;
}
function groupTxs(txs) {
	const groups = [];
	const map = /* @__PURE__ */ new Map();
	for (const tx of txs) {
		const d = new Date(tx.timestamp);
		const key = `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
		const list = map.get(key) ?? [];
		list.push(tx);
		map.set(key, list);
	}
	for (const [key, items] of map) groups.push({
		key,
		items
	});
	return groups;
}
function isLikelyAddress(value, network) {
	const v = value.trim();
	if (network === "solana") return v.length >= 32 && v.length <= 44;
	if (network === "bitcoin") return /^(bc1|[13])[a-zA-HJ-NP-Z0-9]{24,}$/.test(v);
	return /^0x[a-fA-F0-9]{40}$/.test(v);
}
var Route$5 = createFileRoute("/_app/activity")({ component: ActivityPage });
function ActivityPage() {
	const txs = useWallet((s) => s.txs);
	const groups = groupTxs(txs);
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("header", {
		className: "mb-6",
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
			className: "text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground",
			children: "History"
		}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("h1", {
			className: "mt-1 text-xl font-medium tracking-tight",
			children: "Activity"
		})]
	}), txs.length === 0 ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
		className: "rounded-xl bg-card px-5 py-16 text-center shadow-[var(--shadow-border)]",
		children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
			className: "text-sm text-muted-foreground",
			children: "No activity yet."
		})
	}) : /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
		className: "space-y-8",
		children: groups.map((g) => /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("section", { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("h2", {
			className: "mb-2 px-1 text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground",
			children: formatDateHeading(g.items[0].timestamp)
		}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
			className: "divide-y divide-border rounded-xl bg-card px-2 py-1 shadow-[var(--shadow-border)]",
			children: g.items.map((tx) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)(TxRow, { tx }, tx.id))
		})] }, g.key))
	})] });
}
function TxRow({ tx }) {
	const asset = ASSETS[tx.assetId];
	const toAsset = tx.toAssetId ? ASSETS[tx.toAssetId] : null;
	const Icon = tx.type === "send" ? ArrowUpRight : tx.type === "receive" ? ArrowDownLeft : ArrowLeftRight;
	const title = tx.type === "swap" ? `Swap ${asset.symbol} → ${toAsset?.symbol ?? ""}` : tx.type === "send" ? `Sent ${asset.symbol}` : `Received ${asset.symbol}`;
	const amount = tx.type === "swap" && tx.toAmount ? `+${formatAmount(tx.toAmount, toAsset?.symbol ?? "")}` : `${tx.type === "send" ? "−" : "+"}${formatAmount(tx.amount, asset.symbol)}`;
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("article", {
		className: "flex items-center gap-3 px-2 py-3",
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
			className: "relative",
			children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(TokenIcon, { id: tx.toAssetId ?? tx.assetId }), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
				className: "absolute -bottom-0.5 -right-0.5 flex size-4 items-center justify-center rounded-full bg-background shadow-[var(--shadow-border)]",
				children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Icon, { className: "size-2.5" })
			})]
		}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
			className: "min-w-0 flex-1",
			children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "flex items-baseline justify-between gap-2",
				children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
					className: "truncate text-sm font-medium",
					children: title
				}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
					className: "tabular text-sm font-medium",
					children: amount
				})]
			}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "mt-0.5 flex items-center justify-between gap-2 text-xs text-muted-foreground",
				children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
					className: "truncate font-mono",
					children: tx.counterparty ? truncateAddress(tx.counterparty, 4) : truncateAddress(tx.hash, 4)
				}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", { children: formatRelative(tx.timestamp) })]
			})]
		})]
	});
}
var $$splitComponentImporter$4 = () => import("./receive-CkMiC1cY.mjs");
var Route$4 = createFileRoute("/_app/receive")({ component: lazyRouteComponent($$splitComponentImporter$4, "component") });
var $$splitComponentImporter$3 = () => import("./send-CiEFOlLx.mjs");
var Route$3 = createFileRoute("/_app/send")({ component: lazyRouteComponent($$splitComponentImporter$3, "component") });
var $$splitComponentImporter$2 = () => import("./settings-BPlhI1bP.mjs");
var Route$2 = createFileRoute("/_app/settings")({ component: lazyRouteComponent($$splitComponentImporter$2, "component") });
var $$splitComponentImporter$1 = () => import("./swap-D-6FE0ej.mjs");
var Route$1 = createFileRoute("/_app/swap")({ component: lazyRouteComponent($$splitComponentImporter$1, "component") });
var $$splitComponentImporter = () => import("./asset._id-DTZVQblw.mjs");
var Route = createFileRoute("/_app/asset/$id")({ component: lazyRouteComponent($$splitComponentImporter, "component") });
var AppRoute = Route$7.update({
	id: "/_app",
	getParentRoute: () => Route$8
});
var AppIndexRoute = Route$6.update({
	id: "/",
	path: "/",
	getParentRoute: () => AppRoute
});
var AppRouteChildren = {
	AppActivityRoute: Route$5.update({
		id: "/activity",
		path: "/activity",
		getParentRoute: () => AppRoute
	}),
	AppReceiveRoute: Route$4.update({
		id: "/receive",
		path: "/receive",
		getParentRoute: () => AppRoute
	}),
	AppSendRoute: Route$3.update({
		id: "/send",
		path: "/send",
		getParentRoute: () => AppRoute
	}),
	AppSettingsRoute: Route$2.update({
		id: "/settings",
		path: "/settings",
		getParentRoute: () => AppRoute
	}),
	AppSwapRoute: Route$1.update({
		id: "/swap",
		path: "/swap",
		getParentRoute: () => AppRoute
	}),
	AppIndexRoute,
	AppAssetIdRoute: Route.update({
		id: "/asset/$id",
		path: "/asset/$id",
		getParentRoute: () => AppRoute
	})
};
var rootRouteChildren = { AppRoute: AppRoute._addFileChildren(AppRouteChildren) };
var routeTree = Route$8._addFileChildren(rootRouteChildren)._addFileTypes();
var router_exports = /* @__PURE__ */ __exportAll({ getRouter: () => getRouter });
function getRouter() {
	return createRouter({
		routeTree,
		defaultErrorComponent: AppErrorComponent
	});
}
//#endregion
export { isLikelyAddress as a, holdingRows as i, Route as n, portfolioTotals as o, addressForNetwork as r, TokenIcon as s, router_exports as t };
