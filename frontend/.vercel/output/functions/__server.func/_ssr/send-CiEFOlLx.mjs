import { i as __toESM } from "../_runtime.mjs";
import { u as require_react } from "../_libs/@floating-ui/react-dom+[...].mjs";
import { d as formatTokenAmount, f as formatUsd, h as useWallet, m as truncateAddress, n as ASSET_LIST, t as ASSETS } from "./store-CTJXUQRt.mjs";
import { n as usePrices } from "./prices-Dmpyucw2.mjs";
import { O as require_jsx_runtime } from "../_libs/@radix-ui/react-alert-dialog+[...].mjs";
import { t as Button } from "./button-L2zHDOAh.mjs";
import { t as Input } from "./input-DZBl2zTv.mjs";
import { t as Label } from "./label-CXgsbMIG.mjs";
import { x as useNavigate } from "../_libs/@tanstack/react-router+[...].mjs";
import { n as toast } from "../_libs/sonner.mjs";
import { a as isLikelyAddress, s as TokenIcon } from "./router-FskFLzdY.mjs";
import { t as PageHeader } from "./page-header-DWFBPcSd.mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/send-CiEFOlLx.js
var import_react = /* @__PURE__ */ __toESM(require_react());
var import_jsx_runtime = require_jsx_runtime();
function SendPage() {
	const navigate = useNavigate();
	const holdings = useWallet((s) => s.holdings);
	const send = useWallet((s) => s.send);
	const prices = usePrices((s) => s.prices);
	const [assetId, setAssetId] = (0, import_react.useState)("ethereum");
	const [to, setTo] = (0, import_react.useState)("");
	const [amount, setAmount] = (0, import_react.useState)("");
	const [step, setStep] = (0, import_react.useState)("form");
	const [busy, setBusy] = (0, import_react.useState)(false);
	const asset = ASSETS[assetId];
	const available = holdings[assetId] ?? 0;
	const parsed = Number(amount);
	const usd = Number.isFinite(parsed) ? parsed * (prices[assetId]?.usd ?? 0) : 0;
	const network = asset.network;
	const validAmt = Number.isFinite(parsed) && parsed > 0 && parsed <= available;
	const validTo = isLikelyAddress(to, network);
	const quote = (0, import_react.useMemo)(() => ({
		feeUsd: 1.24,
		eta: "~12 sec"
	}), []);
	function continueReview() {
		if (!validTo) {
			toast("Enter a valid address for this network.");
			return;
		}
		if (!validAmt) {
			toast("Enter an amount within your balance.");
			return;
		}
		setStep("review");
	}
	function confirm() {
		setBusy(true);
		try {
			send({
				assetId,
				amount: parsed,
				to: to.trim()
			});
			toast(`Sent ${formatTokenAmount(parsed)} ${asset.symbol}`);
			navigate({ to: "/activity" });
		} catch (err) {
			toast(err instanceof Error ? err.message : "Send failed.");
		} finally {
			setBusy(false);
		}
	}
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(PageHeader, { title: "Send" }), step === "form" ? /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
		className: "space-y-6",
		children: [
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "space-y-2",
				children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Label, { children: "Asset" }), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
					className: "grid grid-cols-2 gap-2 sm:grid-cols-3",
					children: ASSET_LIST.filter((a) => (holdings[a.id] ?? 0) > 0).map((a) => /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("button", {
						type: "button",
						onClick: () => setAssetId(a.id),
						className: `flex items-center gap-2 rounded-lg px-3 py-2.5 text-left text-sm shadow-[var(--shadow-border)] transition-colors duration-150 ${assetId === a.id ? "bg-accent" : "bg-secondary hover:bg-accent"}`,
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(TokenIcon, {
							id: a.id,
							className: "size-8"
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("span", { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
							className: "block font-medium",
							children: a.symbol
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
							className: "block text-xs text-muted-foreground",
							children: formatTokenAmount(holdings[a.id] ?? 0)
						})] })]
					}, a.id))
				})]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "space-y-2",
				children: [
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Label, {
						htmlFor: "to",
						children: "To"
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Input, {
						id: "to",
						value: to,
						onChange: (e) => setTo(e.target.value),
						placeholder: network === "solana" ? "Solana address" : network === "bitcoin" ? "bc1…" : "0x…",
						className: "font-mono text-sm",
						autoComplete: "off"
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("p", {
						className: "text-xs text-muted-foreground",
						children: [
							"Sending on ",
							asset.networkLabel,
							". Double-check the address."
						]
					})
				]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "space-y-2",
				children: [
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "flex items-center justify-between",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Label, {
							htmlFor: "amount",
							children: "Amount"
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("button", {
							type: "button",
							className: "text-xs text-muted-foreground hover:text-foreground",
							onClick: () => setAmount(String(available)),
							children: [
								"Max ",
								formatTokenAmount(available),
								" ",
								asset.symbol
							]
						})]
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Input, {
						id: "amount",
						inputMode: "decimal",
						value: amount,
						onChange: (e) => setAmount(e.target.value),
						placeholder: "0.00",
						className: "tabular text-lg"
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
						className: "text-xs text-muted-foreground tabular",
						children: formatUsd(usd)
					})
				]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
				size: "lg",
				className: "w-full",
				onClick: continueReview,
				children: "Review"
			})
		]
	}) : /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
		className: "space-y-6",
		children: [
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "rounded-xl bg-card px-5 py-6 text-center shadow-[var(--shadow-border)]",
				children: [
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
						className: "text-xs uppercase tracking-[0.16em] text-muted-foreground",
						children: "You are sending"
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("p", {
						className: "mt-3 text-3xl font-medium tracking-tight tabular",
						children: [
							formatTokenAmount(parsed),
							" ",
							asset.symbol
						]
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
						className: "mt-1 text-sm text-muted-foreground",
						children: formatUsd(usd)
					})
				]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("dl", {
				className: "space-y-3 rounded-xl bg-card px-5 py-4 text-sm shadow-[var(--shadow-border)]",
				children: [
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Row, {
						label: "To",
						value: truncateAddress(to.trim(), 6),
						mono: true
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Row, {
						label: "Network",
						value: asset.networkLabel
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Row, {
						label: "Network fee",
						value: formatUsd(quote.feeUsd)
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Row, {
						label: "Time",
						value: quote.eta
					})
				]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
				className: "text-xs leading-relaxed text-muted-foreground",
				children: "Preview transfer. Funds are not broadcast to a public chain."
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "flex gap-2",
				children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
					variant: "secondary",
					size: "lg",
					className: "flex-1",
					onClick: () => setStep("form"),
					children: "Back"
				}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
					size: "lg",
					className: "flex-1",
					disabled: busy,
					onClick: confirm,
					children: "Confirm send"
				})]
			})
		]
	})] });
}
function Row({ label, value, mono }) {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
		className: "flex items-center justify-between gap-4",
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("dt", {
			className: "text-muted-foreground",
			children: label
		}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("dd", {
			className: mono ? "font-mono text-xs" : "tabular",
			children: value
		})]
	});
}
//#endregion
export { SendPage as component };
