import { O as require_jsx_runtime } from "../_libs/@radix-ui/react-alert-dialog+[...].mjs";
import { t as Button } from "./button-L2zHDOAh.mjs";
import { b as Link } from "../_libs/@tanstack/react-router+[...].mjs";
import { u as ChevronLeft } from "../_libs/lucide-react.mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/page-header-DWFBPcSd.js
var import_jsx_runtime = require_jsx_runtime();
function PageHeader({ title, backTo = "/" }) {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("header", {
		className: "mb-8 flex items-center gap-2",
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
			variant: "ghost",
			size: "icon-sm",
			asChild: true,
			"aria-label": "Back",
			children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Link, {
				to: backTo,
				children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(ChevronLeft, { className: "size-5" })
			})
		}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("h1", {
			className: "text-lg font-medium tracking-tight",
			children: title
		})]
	});
}
//#endregion
export { PageHeader as t };
