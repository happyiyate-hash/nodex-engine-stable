import { i as __toESM } from "../_runtime.mjs";
import { u as require_react } from "../_libs/@floating-ui/react-dom+[...].mjs";
import { a as NETWORKS, h as useWallet, o as cn } from "./store-CTJXUQRt.mjs";
import { O as require_jsx_runtime } from "../_libs/@radix-ui/react-alert-dialog+[...].mjs";
import { t as Button } from "./button-L2zHDOAh.mjs";
import { r as addressForNetwork } from "./router-FskFLzdY.mjs";
import { t as PageHeader } from "./page-header-DWFBPcSd.mjs";
import { t as CopyButton } from "./copy-button-C94AJwkb.mjs";
import { t as require_lib } from "../_libs/qrcode.mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/receive-CkMiC1cY.js
var import_react = /* @__PURE__ */ __toESM(require_react());
var import_jsx_runtime = require_jsx_runtime();
var import_lib = /* @__PURE__ */ __toESM(require_lib());
function AddressQr({ value, className }) {
	const [svg, setSvg] = (0, import_react.useState)("");
	(0, import_react.useEffect)(() => {
		let cancelled = false;
		import_lib.toString(value, {
			type: "svg",
			margin: 1,
			color: {
				dark: "#f5f5f4",
				light: "#00000000"
			}
		}).then((out) => {
			if (!cancelled) setSvg(out);
		}).catch(() => {
			if (!cancelled) setSvg("");
		});
		return () => {
			cancelled = true;
		};
	}, [value]);
	return /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
		className: cn("flex aspect-square items-center justify-center rounded-xl bg-secondary p-4 shadow-[var(--shadow-border)]", className),
		"aria-hidden": true,
		dangerouslySetInnerHTML: svg ? { __html: svg } : void 0
	});
}
function ReceivePage() {
	const wallet = useWallet((s) => s.wallet);
	const current = useWallet((s) => s.network);
	const [network, setNetwork] = (0, import_react.useState)(current === "all" ? "ethereum" : current);
	const address = (0, import_react.useMemo)(() => addressForNetwork(wallet.addresses, network), [wallet.addresses, network]);
	const warning = network === "bitcoin" ? "Send only Bitcoin to this address." : network === "solana" ? "Send only Solana and SPL tokens to this address." : "Send only Ether and ERC-20 tokens on this network.";
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", { children: [
		/* @__PURE__ */ (0, import_jsx_runtime.jsx)(PageHeader, { title: "Receive" }),
		/* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
			className: "flex gap-2 overflow-x-auto pb-2",
			children: NETWORKS.filter((n) => n.id !== "all").map((n) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
				size: "sm",
				variant: network === n.id ? "default" : "secondary",
				onClick: () => setNetwork(n.id),
				children: n.label
			}, n.id))
		}),
		/* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
			className: "mx-auto mt-6 max-w-xs",
			children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(AddressQr, {
				value: address,
				className: "w-full"
			})
		}),
		/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
			className: "mt-6 rounded-xl bg-card px-4 py-4 shadow-[var(--shadow-border)]",
			children: [
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
					className: "text-xs uppercase tracking-[0.16em] text-muted-foreground",
					children: wallet.name
				}),
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
					className: cn("mt-2 break-all font-mono text-sm leading-relaxed"),
					children: address
				}),
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)(CopyButton, {
					value: address,
					label: "Copy address",
					className: "mt-4 w-full"
				})
			]
		}),
		/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("p", {
			className: "mt-4 text-center text-xs leading-relaxed text-muted-foreground",
			children: [warning, " This is a preview address derived on-device."]
		})
	] });
}
//#endregion
export { ReceivePage as component };
