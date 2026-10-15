import { i as __toESM } from "./_runtime.mjs";
import { t as cva } from "./_libs/class-variance-authority+clsx.mjs";
import { u as require_react } from "./_libs/@floating-ui/react-dom+[...].mjs";
import { a as NETWORKS, d as formatTokenAmount, f as formatUsd, h as useWallet, o as cn, r as COLLECTIBLES, t as ASSETS } from "./_ssr/store-CTJXUQRt.mjs";
import { n as usePrices, t as sparklinePath } from "./_ssr/prices-Dmpyucw2.mjs";
import { O as require_jsx_runtime } from "./_libs/@radix-ui/react-alert-dialog+[...].mjs";
import { t as Button } from "./_ssr/button-L2zHDOAh.mjs";
import { b as Link } from "./_libs/@tanstack/react-router+[...].mjs";
import { a as Eye, d as Check, f as ArrowUpRight, g as ArrowDownLeft, m as ArrowLeftRight, o as EyeOff } from "./_libs/lucide-react.mjs";
import { a as Label2, c as Separator2, i as ItemIndicator2, l as Trigger, n as Content2, o as Portal2, r as Item2, s as Root2, t as CheckboxItem2 } from "./_libs/@radix-ui/react-dropdown-menu+[...].mjs";
import { i as holdingRows, o as portfolioTotals, s as TokenIcon } from "./_ssr/router-FskFLzdY.mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/_app-DdegeodJ.js
var import_react = /* @__PURE__ */ __toESM(require_react());
var import_jsx_runtime = require_jsx_runtime();
function mulberry32(seed) {
	return () => {
		let t = seed += 1831565813;
		t = Math.imul(t ^ t >>> 15, t | 1);
		t ^= t + Math.imul(t ^ t >>> 7, t | 61);
		return ((t ^ t >>> 14) >>> 0) / 4294967296;
	};
}
function CollectibleArt({ seed, className }) {
	const rand = mulberry32(seed * 97);
	const shapes = Array.from({ length: 7 }, (_, i) => {
		return {
			i,
			kind: rand(),
			x: rand() * 80 + 10,
			y: rand() * 80 + 10,
			s: rand() * 28 + 8,
			o: .18 + rand() * .7
		};
	});
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("svg", {
		viewBox: "0 0 100 100",
		className: cn("bg-secondary text-foreground", className),
		"aria-hidden": true,
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("rect", {
			width: "100",
			height: "100",
			fill: "var(--color-secondary)"
		}), shapes.map((sh) => sh.kind > .55 ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)("rect", {
			x: sh.x - sh.s / 2,
			y: sh.y - sh.s / 2,
			width: sh.s,
			height: sh.s,
			fill: "currentColor",
			opacity: sh.o
		}, sh.i) : sh.kind > .25 ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)("circle", {
			cx: sh.x,
			cy: sh.y,
			r: sh.s / 2,
			fill: "currentColor",
			opacity: sh.o
		}, sh.i) : /* @__PURE__ */ (0, import_jsx_runtime.jsx)("line", {
			x1: sh.x - sh.s / 2,
			y1: sh.y,
			x2: sh.x + sh.s / 2,
			y2: sh.y + (sh.kind > .12 ? sh.s / 3 : -sh.s / 3),
			stroke: "currentColor",
			strokeWidth: "1.2",
			opacity: sh.o
		}, sh.i))]
	});
}
function Sparkline({ seed, end, changePct, className }) {
	const d = sparklinePath(seed, end, changePct);
	const up = changePct >= 0;
	return /* @__PURE__ */ (0, import_jsx_runtime.jsx)("svg", {
		viewBox: "0 0 100 32",
		className: cn("overflow-visible", className),
		"aria-hidden": true,
		children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)("path", {
			d,
			fill: "none",
			stroke: "currentColor",
			strokeWidth: "1.6",
			strokeLinecap: "round",
			strokeLinejoin: "round",
			className: up ? "text-gain" : "text-loss"
		})
	});
}
function AssetRow({ row, hidden }) {
	const asset = ASSETS[row.id];
	const up = row.change >= 0;
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(Link, {
		to: "/asset/$id",
		params: { id: row.id },
		className: "flex items-center gap-3 rounded-lg px-2 py-3 transition-colors duration-150 hover:bg-accent",
		children: [
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)(TokenIcon, { id: row.id }),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "min-w-0 flex-1",
				children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
					className: "flex items-baseline justify-between gap-3",
					children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
						className: "truncate text-sm font-medium",
						children: asset.name
					}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
						className: "tabular text-sm font-medium",
						children: hidden ? "••••" : formatUsd(row.value)
					})]
				}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
					className: "mt-0.5 flex items-center justify-between gap-3",
					children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("p", {
						className: "text-xs text-muted-foreground",
						children: [
							hidden ? "••••" : formatTokenAmount(row.amount),
							" ",
							asset.symbol
						]
					}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("p", {
						className: cn("tabular text-xs", up ? "text-gain" : "text-loss"),
						children: [
							up ? "+" : "",
							row.change.toFixed(2),
							"%"
						]
					})]
				})]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Sparkline, {
				seed: row.id,
				end: row.price,
				changePct: row.change,
				className: "hidden h-8 w-16 sm:block"
			})
		]
	});
}
var badgeVariants = cva("inline-flex items-center rounded-full px-2 py-0.5 text-[11px] font-medium tracking-wide", {
	variants: { variant: {
		default: "bg-secondary text-muted-foreground shadow-[var(--shadow-border)]",
		solid: "bg-primary text-primary-foreground",
		gain: "bg-gain/15 text-gain",
		loss: "bg-loss/15 text-loss"
	} },
	defaultVariants: { variant: "default" }
});
function Badge({ className, variant, ...props }) {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
		className: cn(badgeVariants({ variant }), className),
		...props
	});
}
var DropdownMenu = Root2;
var DropdownMenuTrigger = Trigger;
var DropdownMenuContent = import_react.forwardRef(({ className, sideOffset = 8, ...props }, ref) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Portal2, { children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Content2, {
	ref,
	sideOffset,
	className: cn("z-50 min-w-44 overflow-hidden rounded-lg bg-popover p-1 text-popover-foreground shadow-[var(--shadow-border),var(--shadow-float)] data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95", className),
	...props
}) }));
DropdownMenuContent.displayName = Content2.displayName;
var DropdownMenuItem = import_react.forwardRef(({ className, ...props }, ref) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Item2, {
	ref,
	className: cn("relative flex cursor-pointer select-none items-center gap-2 rounded-md px-2.5 py-2 text-sm outline-none transition-colors focus:bg-accent data-[disabled]:pointer-events-none data-[disabled]:opacity-40", className),
	...props
}));
DropdownMenuItem.displayName = Item2.displayName;
var DropdownMenuCheckboxItem = import_react.forwardRef(({ className, children, checked, ...props }, ref) => /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(CheckboxItem2, {
	ref,
	className: cn("relative flex cursor-pointer select-none items-center rounded-md py-2 pl-8 pr-2 text-sm outline-none transition-colors focus:bg-accent", className),
	checked,
	...props,
	children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
		className: "absolute left-2 flex size-4 items-center justify-center",
		children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(ItemIndicator2, { children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Check, { className: "size-3.5" }) })
	}), children]
}));
DropdownMenuCheckboxItem.displayName = CheckboxItem2.displayName;
var DropdownMenuLabel = import_react.forwardRef(({ className, ...props }, ref) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Label2, {
	ref,
	className: cn("px-2.5 py-1.5 text-xs text-muted-foreground", className),
	...props
}));
DropdownMenuLabel.displayName = Label2.displayName;
var DropdownMenuSeparator = import_react.forwardRef(({ className, ...props }, ref) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Separator2, {
	ref,
	className: cn("-mx-1 my-1 h-px bg-border", className),
	...props
}));
DropdownMenuSeparator.displayName = Separator2.displayName;
function HomePage() {
	const holdings = useWallet((s) => s.holdings);
	const network = useWallet((s) => s.network);
	const setNetwork = useWallet((s) => s.setNetwork);
	const hide = useWallet((s) => s.hideBalances);
	const toggleHide = useWallet((s) => s.toggleHideBalances);
	const name = useWallet((s) => s.wallet?.name ?? "Wallet");
	const prices = usePrices((s) => s.prices);
	const rows = holdingRows(holdings, prices, network);
	const { total, delta, pct } = portfolioTotals(rows);
	const up = delta >= 0;
	const networkLabel = NETWORKS.find((n) => n.id === network)?.label ?? "All networks";
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
		className: "stagger-enter space-y-8",
		children: [
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("header", {
				className: "flex items-start justify-between gap-3",
				children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
					className: "text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground",
					children: name
				}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("h1", {
					className: "mt-1 text-xl font-medium tracking-tight",
					children: "Portfolio"
				})] }), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
					className: "flex items-center gap-1",
					children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)(DropdownMenu, { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(DropdownMenuTrigger, {
						asChild: true,
						children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
							variant: "secondary",
							size: "sm",
							children: networkLabel
						})
					}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(DropdownMenuContent, {
						align: "end",
						children: NETWORKS.map((n) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)(DropdownMenuItem, {
							onClick: () => setNetwork(n.id),
							children: n.label
						}, n.id))
					})] }), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
						variant: "ghost",
						size: "icon-sm",
						"aria-label": hide ? "Show balances" : "Hide balances",
						onClick: toggleHide,
						children: hide ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)(EyeOff, { className: "size-4" }) : /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Eye, { className: "size-4" })
					})]
				})]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("section", {
				className: "rounded-xl bg-card px-5 py-6 shadow-[var(--shadow-border)]",
				children: [
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
						className: "text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground",
						children: "Total balance"
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
						className: "mt-3 text-4xl font-medium tracking-tight tabular sm:text-5xl",
						children: hide ? "••••••" : formatUsd(total)
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "mt-3 flex items-center justify-between gap-4",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("p", {
							className: cn("tabular text-sm", up ? "text-gain" : "text-loss"),
							children: [hide ? "••••" : `${up ? "+" : ""}${formatUsd(delta)} (${up ? "+" : ""}${pct.toFixed(2)}%)`, /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
								className: "ml-1 text-muted-foreground",
								children: "24h"
							})]
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Sparkline, {
							seed: "portfolio",
							end: total || 1,
							changePct: pct,
							className: "h-10 w-28"
						})]
					}),
					rows.length > 0 && /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
						className: "mt-6 flex h-1.5 overflow-hidden rounded-full bg-secondary",
						children: rows.map((row) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
							className: "h-full bg-foreground",
							style: {
								width: `${row.value / total * 100}%`,
								opacity: .25 + row.value / total * .75
							}
						}, row.id))
					})
				]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "grid grid-cols-3 gap-2",
				children: [
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
						variant: "secondary",
						className: "h-14 flex-col gap-1 rounded-lg",
						asChild: true,
						children: /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(Link, {
							to: "/send",
							children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(ArrowUpRight, { className: "size-4" }), "Send"]
						})
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
						variant: "secondary",
						className: "h-14 flex-col gap-1 rounded-lg",
						asChild: true,
						children: /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(Link, {
							to: "/receive",
							children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(ArrowDownLeft, { className: "size-4" }), "Receive"]
						})
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
						variant: "secondary",
						className: "h-14 flex-col gap-1 rounded-lg",
						asChild: true,
						children: /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(Link, {
							to: "/swap",
							children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(ArrowLeftRight, { className: "size-4" }), "Swap"]
						})
					})
				]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("section", { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "mb-2 flex items-center justify-between px-1",
				children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("h2", {
					className: "text-sm font-medium",
					children: "Assets"
				}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Badge, { children: "Preview" })]
			}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
				className: "divide-y divide-border rounded-xl bg-card px-2 py-1 shadow-[var(--shadow-border)]",
				children: rows.length === 0 ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
					className: "px-3 py-8 text-center text-sm text-muted-foreground",
					children: "No assets on this network."
				}) : rows.map((row) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)(AssetRow, {
					row,
					hidden: hide
				}, row.id))
			})] }),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("section", { children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "mb-3 flex items-center justify-between px-1",
				children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("h2", {
					className: "text-sm font-medium",
					children: "Collectibles"
				}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
					className: "text-xs text-muted-foreground",
					children: "Monolith"
				})]
			}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
				className: "grid grid-cols-2 gap-3 sm:grid-cols-4",
				children: COLLECTIBLES.map((c) => /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("article", {
					className: "overflow-hidden rounded-lg bg-card shadow-[var(--shadow-border)]",
					children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(CollectibleArt, {
						seed: c.seed,
						className: "aspect-square w-full"
					}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "px-3 py-2.5",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
							className: "text-sm font-medium",
							children: c.name
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
							className: "text-xs text-muted-foreground",
							children: c.collection
						})]
					})]
				}, c.id))
			})] })
		]
	});
}
//#endregion
export { HomePage as component };
