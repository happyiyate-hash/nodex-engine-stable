import { d as formatTokenAmount, f as formatUsd, h as useWallet, o as cn, t as ASSETS } from "./store-CTJXUQRt.mjs";
import { n as usePrices, t as sparklinePath } from "./prices-Dmpyucw2.mjs";
import { O as require_jsx_runtime } from "../_libs/@radix-ui/react-alert-dialog+[...].mjs";
import { t as Button } from "./button-L2zHDOAh.mjs";
import { J as notFound, b as Link } from "../_libs/@tanstack/react-router+[...].mjs";
import { n as Route, s as TokenIcon } from "./router-FskFLzdY.mjs";
import { t as PageHeader } from "./page-header-DWFBPcSd.mjs";
import { a as ResponsiveContainer, i as Area, n as YAxis, o as Tooltip, r as XAxis, t as AreaChart } from "../_libs/recharts+[...].mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/asset._id-DTZVQblw.js
var import_jsx_runtime = require_jsx_runtime();
var IDS = new Set(Object.keys(ASSETS));
function AssetPage() {
	const { id } = Route.useParams();
	if (!IDS.has(id)) throw notFound();
	const assetId = id;
	const asset = ASSETS[assetId];
	const amount = useWallet((s) => s.holdings[assetId] ?? 0);
	const hide = useWallet((s) => s.hideBalances);
	const prices = usePrices((s) => s.prices);
	const px = prices[assetId]?.usd ?? 0;
	const change = prices[assetId]?.usd_24h_change ?? 0;
	const value = amount * px;
	const up = change >= 0;
	const data = chartSeries(assetId, px, change);
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", { children: [
		/* @__PURE__ */ (0, import_jsx_runtime.jsx)(PageHeader, { title: asset.symbol }),
		/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
			className: "flex items-center gap-3",
			children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(TokenIcon, {
				id: assetId,
				className: "size-12"
			}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("h1", {
				className: "text-xl font-medium tracking-tight",
				children: asset.name
			}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
				className: "text-sm text-muted-foreground",
				children: asset.networkLabel
			})] })]
		}),
		/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
			className: "mt-8",
			children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
				className: "text-3xl font-medium tracking-tight tabular",
				children: hide ? "••••" : formatUsd(px, { digits: px < 2 ? 4 : 2 })
			}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("p", {
				className: cn("mt-1 tabular text-sm", up ? "text-gain" : "text-loss"),
				children: [
					up ? "+" : "",
					change.toFixed(2),
					"% ",
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
						className: "text-muted-foreground",
						children: "24h"
					})
				]
			})]
		}),
		/* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
			className: "mt-6 h-44",
			children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(ResponsiveContainer, {
				width: "100%",
				height: "100%",
				children: /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(AreaChart, {
					data,
					margin: {
						top: 8,
						right: 0,
						left: 0,
						bottom: 0
					},
					children: [
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)("defs", { children: /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("linearGradient", {
							id: "fillPx",
							x1: "0",
							y1: "0",
							x2: "0",
							y2: "1",
							children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("stop", {
								offset: "0%",
								stopColor: "currentColor",
								stopOpacity: .18
							}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("stop", {
								offset: "100%",
								stopColor: "currentColor",
								stopOpacity: 0
							})]
						}) }),
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)(XAxis, {
							dataKey: "i",
							hide: true
						}),
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)(YAxis, {
							hide: true,
							domain: ["dataMin", "dataMax"]
						}),
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Tooltip, {
							cursor: { stroke: "rgba(245,245,244,0.12)" },
							content: ({ payload }) => {
								const v = payload?.[0]?.value;
								if (typeof v !== "number") return null;
								return /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
									className: "rounded-md bg-popover px-2 py-1 text-xs shadow-[var(--shadow-border)]",
									children: formatUsd(v, { digits: px < 2 ? 4 : 2 })
								});
							}
						}),
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Area, {
							type: "monotone",
							dataKey: "v",
							stroke: "currentColor",
							strokeWidth: 1.6,
							fill: "url(#fillPx)",
							className: up ? "text-gain" : "text-loss"
						})
					]
				})
			})
		}),
		/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("dl", {
			className: "mt-6 grid grid-cols-2 gap-3",
			children: [
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Stat, {
					label: "Holdings",
					value: hide ? "••••" : `${formatTokenAmount(amount)} ${asset.symbol}`
				}),
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Stat, {
					label: "Value",
					value: hide ? "••••" : formatUsd(value)
				}),
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Stat, {
					label: "Network",
					value: asset.networkLabel
				}),
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Stat, {
					label: "Asset",
					value: asset.symbol
				})
			]
		}),
		/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
			className: "mt-8 grid grid-cols-2 gap-2",
			children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
				size: "lg",
				asChild: true,
				children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Link, {
					to: "/send",
					children: "Send"
				})
			}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
				size: "lg",
				variant: "secondary",
				asChild: true,
				children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Link, {
					to: "/receive",
					children: "Receive"
				})
			})]
		})
	] });
}
function Stat({ label, value }) {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
		className: "rounded-lg bg-card px-4 py-3 shadow-[var(--shadow-border)]",
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
			className: "text-xs text-muted-foreground",
			children: label
		}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
			className: "mt-1 text-sm font-medium",
			children: value
		})]
	});
}
function chartSeries(id, end, change) {
	const nums = [...sparklinePath(id, end || 1, change, 32).matchAll(/[\d.]+/g)].map(Number);
	const ys = [];
	for (let i = 1; i < nums.length; i += 2) ys.push(nums[i]);
	const min = Math.min(...ys);
	const span = Math.max(...ys) - min || 1;
	const start = end / (1 + change / 100);
	return ys.map((y, i) => {
		const t = 1 - (y - min) / span;
		return {
			i,
			v: start + (end - start) * (i / (ys.length - 1)) + (t - .5) * end * .02
		};
	});
}
//#endregion
export { AssetPage as component };
