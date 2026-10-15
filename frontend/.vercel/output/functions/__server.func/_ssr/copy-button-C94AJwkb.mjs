import { i as __toESM } from "../_runtime.mjs";
import { u as require_react } from "../_libs/@floating-ui/react-dom+[...].mjs";
import { o as cn, s as copyText } from "./store-CTJXUQRt.mjs";
import { O as require_jsx_runtime } from "../_libs/@radix-ui/react-alert-dialog+[...].mjs";
import { t as Button } from "./button-L2zHDOAh.mjs";
import { c as Copy, d as Check } from "../_libs/lucide-react.mjs";
import { n as toast } from "../_libs/sonner.mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/copy-button-C94AJwkb.js
var import_react = /* @__PURE__ */ __toESM(require_react());
var import_jsx_runtime = require_jsx_runtime();
function CopyButton({ value, label = "Copy", className }) {
	const [copied, setCopied] = (0, import_react.useState)(false);
	async function onCopy() {
		await copyText(value);
		setCopied(true);
		toast("Copied");
		window.setTimeout(() => setCopied(false), 1400);
	}
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(Button, {
		type: "button",
		variant: "secondary",
		className,
		onClick: onCopy,
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("span", {
			className: "relative size-4",
			children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Copy, { className: cn("absolute inset-0 size-4 transition-[opacity,transform,filter] duration-200", copied ? "scale-[0.25] opacity-0 blur-[4px]" : "scale-100 opacity-100") }), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Check, { className: cn("absolute inset-0 size-4 transition-[opacity,transform,filter] duration-200", copied ? "scale-100 opacity-100" : "scale-[0.25] opacity-0 blur-[4px]") })]
		}), label]
	});
}
//#endregion
export { CopyButton as t };
