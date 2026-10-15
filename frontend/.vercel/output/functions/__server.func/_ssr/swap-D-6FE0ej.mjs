import { i as __toESM } from "../_runtime.mjs";
import { u as require_react } from "../_libs/@floating-ui/react-dom+[...].mjs";
import { d as formatTokenAmount, f as formatUsd, h as useWallet, n as ASSET_LIST, t as ASSETS } from "./store-CTJXUQRt.mjs";
import { n as usePrices } from "./prices-Dmpyucw2.mjs";
import { O as require_jsx_runtime } from "../_libs/@radix-ui/react-alert-dialog+[...].mjs";
import { t as Button } from "./button-L2zHDOAh.mjs";
import { t as Input } from "./input-DZBl2zTv.mjs";
import { h as ArrowDown } from "../_libs/lucide-react.mjs";
import { n as toast } from "../_libs/sonner.mjs";
import { s as TokenIcon } from "./router-FskFLzdY.mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/swap-D-6FE0ej.js
var import_react = /* @__PURE__ */ __toESM(require_react());
var import_jsx_runtime = require_jsx_runtime();
function SwapPage() {
	const holdings = useWallet((s) => s.holdings);
	const swap = useWallet((s) => s.swap);
	const prices = usePrices((s) => s.prices);
	const [fromId, setFromId] = (0, import_react.useState)("ethereum");
	const [toId, setToId] = (0, import_react.useState)("usd-coin");
	const [amount, setAmount] = (0, import_react.useState)("");
	const [busy, setBusy] = (0, import_react.useState)(false);
	const from = ASSETS[fromId];
	const to = ASSETS[toId];
	const available = holdings[fromId] ?? 0;
	const parsed = Number(amount);
	const fromPx = prices[fromId]?.usd ?? 0;
	const toPx = prices[toId]?.usd ?? 0;
	const rate = toPx === 0 ? 0 : fromPx / toPx;
	const toAmount = Number.isFinite(parsed) && parsed > 0 ? parsed * rate * .997 : 0;
	const valid = fromId !== toId && Number.isFinite(parsed) && parsed > 0 && parsed <= available && toAmount > 0;
	const impact = (0, import_react.useMemo)(() => parsed > available * .4 ? .42 : .08, [parsed, available]);
	function flip() {
		setFromId(toId);
		setToId(fromId);
		setAmount(toAmount ? String(Number(toAmount.toPrecision(6))) : "");
	}
	function confirm() {
		if (!valid) return;
		setBusy(true);
		try {
			swap({
				fromId,
				toId,
				fromAmount: parsed,
				toAmount
			});
			toast(`Swapped ${formatTokenAmount(parsed)} ${from.symbol}`);
			setAmount("");
		} catch (err) {
			toast(err instanceof Error ? err.message : "Swap failed.");
		} finally {
			setBusy(false);
		}
	}
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
		className: "space-y-6",
		children: [
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("header", { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
				className: "text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground",
				children: "Exchange"
			}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("h1", {
				className: "mt-1 text-xl font-medium tracking-tight",
				children: "Swap"
			})] }),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "relative space-y-2",
				children: [
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(SwapLeg, {
						label: "You pay",
						assetId: fromId,
						onAsset: setFromId,
						amount,
						onAmount: setAmount,
						available,
						usd: Number.isFinite(parsed) ? parsed * fromPx : 0,
						exclude: toId
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
						className: "relative z-10 -my-1 flex justify-center",
						children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
							type: "button",
							size: "icon-sm",
							variant: "secondary",
							"aria-label": "Flip assets",
							onClick: flip,
							children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(ArrowDown, { className: "size-4" })
						})
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(SwapLeg, {
						label: "You receive",
						assetId: toId,
						onAsset: setToId,
						amount: toAmount ? formatTokenAmount(toAmount) : "",
						usd: toAmount * toPx,
						exclude: fromId,
						readOnly: true
					})
				]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("dl", {
				className: "space-y-2.5 rounded-xl bg-card px-4 py-4 text-sm shadow-[var(--shadow-border)]",
				children: [
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "flex justify-between",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("dt", {
							className: "text-muted-foreground",
							children: "Rate"
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("dd", {
							className: "tabular",
							children: [
								"1 ",
								from.symbol,
								" = ",
								rate ? formatTokenAmount(rate) : "—",
								" ",
								to.symbol
							]
						})]
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "flex justify-between",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("dt", {
							className: "text-muted-foreground",
							children: "Fee"
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("dd", {
							className: "tabular",
							children: "0.30%"
						})]
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "flex justify-between",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("dt", {
							className: "text-muted-foreground",
							children: "Price impact"
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("dd", {
							className: "tabular",
							children: [impact.toFixed(2), "%"]
						})]
					})
				]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
				size: "lg",
				className: "w-full",
				disabled: !valid || busy,
				onClick: confirm,
				children: fromId === toId ? "Select different assets" : "Confirm swap"
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
				className: "text-center text-xs text-muted-foreground",
				children: "Quotes use live market prices when available. Settlement is local to this preview."
			})
		]
	});
}
function SwapLeg({ label, assetId, onAsset, amount, onAmount, available, usd, exclude, readOnly }) {
	const asset = ASSETS[assetId];
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
		className: "rounded-xl bg-card p-4 shadow-[var(--shadow-border)]",
		children: [
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "flex items-center justify-between",
				children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
					className: "text-xs text-muted-foreground",
					children: label
				}), available !== void 0 && /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("button", {
					type: "button",
					className: "text-xs text-muted-foreground hover:text-foreground",
					onClick: () => onAmount?.(String(available)),
					children: ["Balance ", formatTokenAmount(available)]
				})]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "mt-3 flex items-center gap-3",
				children: [
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)("label", {
						className: "sr-only",
						htmlFor: `asset-${label}`,
						children: "Asset"
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "flex items-center gap-2 rounded-md bg-secondary py-1.5 pl-1.5 pr-2 shadow-[var(--shadow-border)]",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(TokenIcon, {
							id: assetId,
							className: "size-7"
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("select", {
							id: `asset-${label}`,
							value: assetId,
							onChange: (e) => onAsset(e.target.value),
							className: "bg-transparent text-sm font-medium outline-none",
							children: ASSET_LIST.map((a) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)("option", {
								value: a.id,
								disabled: a.id === exclude,
								children: a.symbol
							}, a.id))
						})]
					}),
					readOnly ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
						className: "min-w-0 flex-1 text-right text-2xl font-medium tabular",
						children: amount || "0"
					}) : /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Input, {
						value: amount,
						onChange: (e) => onAmount?.(e.target.value),
						placeholder: "0",
						inputMode: "decimal",
						className: "h-12 flex-1 border-0 bg-transparent text-right text-2xl shadow-none focus-visible:ring-0"
					})
				]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("p", {
				className: "mt-2 text-right text-xs text-muted-foreground tabular",
				children: [
					formatUsd(usd),
					" · ",
					asset.name
				]
			})
		]
	});
}
//#endregion
export { SwapPage as component };
