import { i as __toESM } from "./_runtime.mjs";
import { u as require_react } from "./_libs/@floating-ui/react-dom+[...].mjs";
import { h as useWallet, m as truncateAddress, o as cn, p as isValidMnemonic } from "./_ssr/store-CTJXUQRt.mjs";
import { n as usePrices } from "./_ssr/prices-Dmpyucw2.mjs";
import { O as require_jsx_runtime } from "./_libs/@radix-ui/react-alert-dialog+[...].mjs";
import { t as Button } from "./_ssr/button-L2zHDOAh.mjs";
import { t as Input } from "./_ssr/input-DZBl2zTv.mjs";
import { t as Label } from "./_ssr/label-CXgsbMIG.mjs";
import { b as Link, g as Outlet, p as useRouterState } from "./_libs/@tanstack/react-router+[...].mjs";
import { a as Eye, i as House, l as Clock, m as ArrowLeftRight, n as Shield, o as EyeOff, p as ArrowRight, r as Settings, s as Delete } from "./_libs/lucide-react.mjs";
import { n as toast } from "./_libs/sonner.mjs";
import { n as Portal, r as Provider, t as Content2 } from "./_libs/@radix-ui/react-tooltip+[...].mjs";
//#region node_modules/.nitro/vite/services/ssr/assets/_app-B6BEWIwF.js
var import_react = /* @__PURE__ */ __toESM(require_react());
var import_jsx_runtime = require_jsx_runtime();
function SableMark({ className }) {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("svg", {
		viewBox: "0 0 32 32",
		className: cn("text-foreground", className),
		"aria-hidden": true,
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("circle", {
			cx: "16",
			cy: "16",
			r: "14",
			fill: "currentColor"
		}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("circle", {
			cx: "21",
			cy: "16",
			r: "11",
			fill: "var(--color-background)"
		})]
	});
}
function SableWordmark({ className }) {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("span", {
		className: cn("flex items-center gap-2.5", className),
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(SableMark, { className: "size-6" }), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
			className: "text-[15px] font-medium tracking-tight",
			children: "Sable"
		})]
	});
}
var TooltipProvider = Provider;
var TooltipContent = import_react.forwardRef(({ className, sideOffset = 6, ...props }, ref) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Portal, { children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Content2, {
	ref,
	sideOffset,
	className: cn("z-50 overflow-hidden rounded-md bg-primary px-2.5 py-1.5 text-xs text-primary-foreground shadow-[var(--shadow-float)]", className),
	...props
}) }));
TooltipContent.displayName = Content2.displayName;
var NAV = [
	{
		to: "/",
		label: "Home",
		icon: House
	},
	{
		to: "/swap",
		label: "Swap",
		icon: ArrowLeftRight
	},
	{
		to: "/activity",
		label: "Activity",
		icon: Clock
	},
	{
		to: "/settings",
		label: "Settings",
		icon: Settings
	}
];
function AppShell() {
	const pathname = useRouterState({ select: (s) => s.location.pathname });
	const wallet = useWallet((s) => s.wallet);
	const lock = useWallet((s) => s.lock);
	const refresh = usePrices((s) => s.refresh);
	(0, import_react.useEffect)(() => {
		refresh();
		const id = window.setInterval(() => void refresh(), 6e4);
		return () => window.clearInterval(id);
	}, [refresh]);
	(0, import_react.useEffect)(() => {
		if (!wallet?.pinHash) return;
		let timer = 0;
		const bump = () => {
			window.clearTimeout(timer);
			timer = window.setTimeout(() => lock(), 3e5);
		};
		bump();
		window.addEventListener("pointerdown", bump);
		return () => {
			window.clearTimeout(timer);
			window.removeEventListener("pointerdown", bump);
		};
	}, [wallet?.pinHash, lock]);
	return /* @__PURE__ */ (0, import_jsx_runtime.jsx)(TooltipProvider, {
		delayDuration: 200,
		children: /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
			className: "min-h-dvh bg-background text-foreground",
			children: [
				/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("aside", {
					className: "fixed inset-y-0 left-0 hidden w-56 flex-col border-r border-border px-4 py-6 lg:flex",
					children: [
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)(SableWordmark, { className: "px-2" }),
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)("nav", {
							className: "mt-10 flex flex-1 flex-col gap-1",
							children: NAV.map((item) => {
								const active = item.to === "/" ? pathname === "/" : pathname.startsWith(item.to);
								const Icon = item.icon;
								return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(Link, {
									to: item.to,
									className: cn("flex h-11 items-center gap-3 rounded-md px-3 text-sm font-medium transition-colors duration-150", active ? "bg-accent text-foreground" : "text-muted-foreground hover:bg-accent hover:text-foreground"),
									children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Icon, { className: "size-4" }), item.label]
								}, item.to);
							})
						}),
						wallet && /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
							className: "rounded-lg bg-secondary px-3 py-3 shadow-[var(--shadow-border)]",
							children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
								className: "text-sm font-medium",
								children: wallet.name
							}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
								className: "mt-1 font-mono text-xs text-muted-foreground",
								children: truncateAddress(wallet.addresses.evm, 5)
							})]
						})
					]
				}),
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
					className: "lg:pl-56",
					children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)("main", {
						className: "mx-auto min-h-dvh w-full max-w-xl px-5 pb-28 pt-6 lg:max-w-2xl lg:pb-12 lg:pt-10",
						children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Outlet, {})
					})
				}),
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)("nav", {
					className: "fixed inset-x-0 bottom-0 z-40 border-t border-border bg-background/95 pb-[env(safe-area-inset-bottom)] backdrop-blur-sm lg:hidden",
					children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
						className: "mx-auto grid max-w-xl grid-cols-4 px-2 pt-1",
						children: NAV.map((item) => {
							const active = item.to === "/" ? pathname === "/" : pathname.startsWith(item.to);
							const Icon = item.icon;
							return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(Link, {
								to: item.to,
								className: cn("flex min-h-14 flex-col items-center justify-center gap-1 text-[11px] font-medium", active ? "text-foreground" : "text-muted-foreground"),
								children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Icon, { className: "size-5" }), item.label]
							}, item.to);
						})
					})
				})
			]
		})
	});
}
function BootScreen() {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
		className: "flex min-h-dvh items-center justify-center bg-background",
		children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(SableMark, { className: "size-12 animate-[sable-pulse_1.6s_ease-in-out_infinite]" })
	});
}
var KEYS = [
	"1",
	"2",
	"3",
	"4",
	"5",
	"6",
	"7",
	"8",
	"9",
	"",
	"0",
	"del"
];
function LockScreen() {
	const unlock = useWallet((s) => s.unlock);
	const name = useWallet((s) => s.wallet?.name ?? "Wallet");
	const [pin, setPin] = (0, import_react.useState)("");
	const [error, setError] = (0, import_react.useState)(false);
	const [busy, setBusy] = (0, import_react.useState)(false);
	async function attempt(next) {
		if (next.length < 6) {
			setPin(next);
			return;
		}
		setPin(next);
		setBusy(true);
		const ok = await unlock(next);
		setBusy(false);
		if (!ok) {
			setError(true);
			window.setTimeout(() => {
				setPin("");
				setError(false);
			}, 420);
		}
	}
	function onKey(key) {
		if (busy) return;
		if (key === "del") {
			setPin((p) => p.slice(0, -1));
			return;
		}
		if (!key) return;
		if (pin.length >= 6) return;
		attempt(pin + key);
	}
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
		className: "flex min-h-dvh flex-col items-center justify-between bg-background px-6 py-12",
		children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
			className: "stagger-enter flex flex-col items-center pt-10 text-center",
			children: [
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)(SableMark, { className: "size-14" }),
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
					className: "mt-6 text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground",
					children: "Locked"
				}),
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)("h1", {
					className: "mt-2 text-2xl font-medium tracking-tight",
					children: name
				}),
				/* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
					className: cn("mt-8 flex gap-3", error && "animate-pulse"),
					children: Array.from({ length: 6 }).map((_, i) => /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", { className: cn("size-2.5 rounded-full transition-colors duration-150", i < pin.length ? "bg-foreground" : "bg-foreground/20", error && "bg-destructive") }, i))
				}),
				error && /* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
					className: "mt-4 text-sm text-destructive",
					children: "Incorrect PIN"
				})
			]
		}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
			className: "grid w-full max-w-xs grid-cols-3 gap-2 pb-[env(safe-area-inset-bottom)]",
			children: KEYS.map((key, i) => key === "" ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {}, i) : /* @__PURE__ */ (0, import_jsx_runtime.jsx)("button", {
				type: "button",
				onClick: () => onKey(key),
				className: "flex h-16 items-center justify-center rounded-lg text-xl font-medium tabular-nums transition-[background-color,transform] duration-150 hover:bg-accent active:scale-[0.96]",
				children: key === "del" ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Delete, { className: "size-5" }) : key
			}, key))
		})]
	});
}
function Onboarding() {
	const createWallet = useWallet((s) => s.createWallet);
	const restoreWallet = useWallet((s) => s.restoreWallet);
	const setPin = useWallet((s) => s.setPin);
	const wallet = useWallet((s) => s.wallet);
	const [step, setStep] = (0, import_react.useState)("welcome");
	const [name, setName] = (0, import_react.useState)("Primary");
	const [phrase, setPhrase] = (0, import_react.useState)("");
	const [restorePhrase, setRestorePhrase] = (0, import_react.useState)("");
	const [revealed, setRevealed] = (0, import_react.useState)(false);
	const [busy, setBusy] = (0, import_react.useState)(false);
	const [pin, setPinValue] = (0, import_react.useState)("");
	const [pin2, setPin2] = (0, import_react.useState)("");
	async function onCreate() {
		setBusy(true);
		try {
			const w = await createWallet(name);
			setPhrase(w.phrase);
			setStep("backup");
		} finally {
			setBusy(false);
		}
	}
	async function onRestore() {
		setBusy(true);
		try {
			await restoreWallet(restorePhrase, name);
			setStep("pin");
		} catch (err) {
			toast(err instanceof Error ? err.message : "Could not restore wallet.");
		} finally {
			setBusy(false);
		}
	}
	async function onPin(skip) {
		if (!skip) {
			if (pin.length !== 6 || pin !== pin2) {
				toast("PINs must match and be 6 digits.");
				return;
			}
			await setPin(pin);
		}
	}
	return /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
		className: "relative flex min-h-dvh flex-col bg-background",
		children: /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
			className: "mx-auto flex w-full max-w-md flex-1 flex-col px-6 py-10",
			children: [
				step === "welcome" && /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
					className: "flex flex-1 flex-col",
					children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "stagger-enter flex flex-1 flex-col justify-end pb-10",
						children: [
							/* @__PURE__ */ (0, import_jsx_runtime.jsx)(SableMark, { className: "mb-8 size-12" }),
							/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
								className: "text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground",
								children: "Sable"
							}),
							/* @__PURE__ */ (0, import_jsx_runtime.jsx)("h1", {
								className: "mt-3 text-4xl font-medium tracking-tight text-foreground",
								children: "A private wallet for digital assets."
							}),
							/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
								className: "mt-4 max-w-sm text-[15px] leading-relaxed text-muted-foreground",
								children: "Self-custodial by design. Preview balances and transfers so you can learn the product without moving real funds."
							})
						]
					}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "stagger-enter flex flex-col gap-3",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)(Button, {
							size: "lg",
							onClick: () => setStep("create"),
							children: ["Create wallet", /* @__PURE__ */ (0, import_jsx_runtime.jsx)(ArrowRight, { className: "size-4" })]
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
							size: "lg",
							variant: "secondary",
							onClick: () => setStep("restore"),
							children: "Restore with phrase"
						})]
					})]
				}),
				step === "create" && /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
					className: "flex flex-1 flex-col",
					children: [
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Header, {
							kicker: "New wallet",
							title: "Name this account",
							copy: "You can rename it later. One account, several networks."
						}),
						/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
							className: "mt-8 space-y-2",
							children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Label, {
								htmlFor: "wallet-name",
								children: "Account name"
							}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Input, {
								id: "wallet-name",
								value: name,
								onChange: (e) => setName(e.target.value),
								placeholder: "Primary",
								autoFocus: true
							})]
						}),
						/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
							className: "mt-auto flex flex-col gap-3 pt-10",
							children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
								size: "lg",
								onClick: onCreate,
								disabled: busy,
								children: "Generate recovery phrase"
							}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
								size: "lg",
								variant: "ghost",
								onClick: () => setStep("welcome"),
								children: "Back"
							})]
						})
					]
				}),
				step === "backup" && /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
					className: "flex flex-1 flex-col",
					children: [
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Header, {
							kicker: "Recovery",
							title: "Write these words down",
							copy: "This preview phrase unlocks this wallet on this device. Store it privately. Never share it."
						}),
						/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
							className: "relative mt-8",
							children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("ol", {
								className: "grid grid-cols-2 gap-2 rounded-xl bg-secondary p-3 shadow-[var(--shadow-border)] sm:grid-cols-3",
								children: phrase.split(" ").map((word, i) => /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("li", {
									className: "flex items-baseline gap-2 rounded-md bg-background/40 px-3 py-2.5 font-mono text-sm",
									children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
										className: "text-xs text-muted-foreground tabular",
										children: i + 1
									}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("span", {
										className: revealed ? "text-foreground" : "blur-[5px]",
										children: word
									})]
								}, `${word}-${i}`))
							}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
								className: "mt-3 flex justify-end",
								children: /* @__PURE__ */ (0, import_jsx_runtime.jsxs)(Button, {
									variant: "ghost",
									size: "sm",
									onClick: () => setRevealed((v) => !v),
									children: [revealed ? /* @__PURE__ */ (0, import_jsx_runtime.jsx)(EyeOff, { className: "size-4" }) : /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Eye, { className: "size-4" }), revealed ? "Hide" : "Reveal"]
								})
							})]
						}),
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)("div", {
							className: "mt-auto flex flex-col gap-3 pt-8",
							children: /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
								size: "lg",
								onClick: () => setStep("pin"),
								children: "I saved these words"
							})
						})
					]
				}),
				step === "restore" && /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
					className: "flex flex-1 flex-col",
					children: [
						/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Header, {
							kicker: "Restore",
							title: "Enter your 12-word phrase",
							copy: "Use words from the Sable preview word list. This derives a local address; it does not import a live chain account."
						}),
						/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
							className: "mt-8 space-y-4",
							children: [/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
								className: "space-y-2",
								children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Label, {
									htmlFor: "restore-name",
									children: "Account name"
								}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Input, {
									id: "restore-name",
									value: name,
									onChange: (e) => setName(e.target.value)
								})]
							}), /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
								className: "space-y-2",
								children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Label, {
									htmlFor: "restore-phrase",
									children: "Recovery phrase"
								}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)("textarea", {
									id: "restore-phrase",
									value: restorePhrase,
									onChange: (e) => setRestorePhrase(e.target.value),
									rows: 4,
									className: "w-full resize-none rounded-md bg-secondary px-3.5 py-3 font-mono text-sm shadow-[var(--shadow-border)] outline-none focus-visible:ring-2 focus-visible:ring-ring/40",
									placeholder: "twelve words separated by spaces"
								})]
							})]
						}),
						/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
							className: "mt-auto flex flex-col gap-3 pt-8",
							children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
								size: "lg",
								disabled: busy || !isValidMnemonic(restorePhrase),
								onClick: onRestore,
								children: "Restore wallet"
							}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
								size: "lg",
								variant: "ghost",
								onClick: () => setStep("welcome"),
								children: "Back"
							})]
						})
					]
				}),
				step === "pin" && wallet && /* @__PURE__ */ (0, import_jsx_runtime.jsx)(PinStep, {
					pin,
					pin2,
					onPin: setPinValue,
					onPin2: setPin2,
					onContinue: onPin
				})
			]
		})
	});
}
function Header({ kicker, title, copy }) {
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
		className: "stagger-enter pt-4",
		children: [
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
				className: "text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground",
				children: kicker
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)("h1", {
				className: "mt-3 text-3xl font-medium tracking-tight",
				children: title
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)("p", {
				className: "mt-3 text-[15px] leading-relaxed text-muted-foreground",
				children: copy
			})
		]
	});
}
function PinStep({ pin, pin2, onPin, onPin2, onContinue }) {
	const [busy, setBusy] = (0, import_react.useState)(false);
	async function finish(skip) {
		setBusy(true);
		try {
			await onContinue(skip);
		} finally {
			setBusy(false);
		}
	}
	return /* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
		className: "flex flex-1 flex-col",
		children: [
			/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Header, {
				kicker: "Security",
				title: "Add a 6-digit PIN",
				copy: "Optional. If you set one, Sable will lock after idle time on this device."
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "mt-8 space-y-4",
				children: [
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "space-y-2",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Label, {
							htmlFor: "pin",
							children: "PIN"
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Input, {
							id: "pin",
							inputMode: "numeric",
							maxLength: 6,
							value: pin,
							onChange: (e) => onPin(e.target.value.replace(/\D/g, "").slice(0, 6)),
							placeholder: "••••••",
							className: "font-mono tracking-[0.4em]"
						})]
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
						className: "space-y-2",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Label, {
							htmlFor: "pin2",
							children: "Confirm PIN"
						}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Input, {
							id: "pin2",
							inputMode: "numeric",
							maxLength: 6,
							value: pin2,
							onChange: (e) => onPin2(e.target.value.replace(/\D/g, "").slice(0, 6)),
							placeholder: "••••••",
							className: "font-mono tracking-[0.4em]"
						})]
					}),
					/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("p", {
						className: "flex items-start gap-2 text-xs leading-relaxed text-muted-foreground",
						children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Shield, { className: "mt-0.5 size-3.5 shrink-0" }), "The PIN never leaves this browser. There is no recovery besides your phrase."]
					})
				]
			}),
			/* @__PURE__ */ (0, import_jsx_runtime.jsxs)("div", {
				className: "mt-auto flex flex-col gap-3 pt-8",
				children: [/* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
					size: "lg",
					disabled: busy || pin.length !== 6,
					onClick: () => finish(false),
					children: "Set PIN"
				}), /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Button, {
					size: "lg",
					variant: "ghost",
					disabled: busy,
					onClick: () => finish(true),
					children: "Skip for now"
				})]
			})
		]
	});
}
function AppGate() {
	const hydrated = useWallet((s) => s.hydrated);
	const setHydrated = useWallet((s) => s.setHydrated);
	const wallet = useWallet((s) => s.wallet);
	const locked = useWallet((s) => s.locked);
	(0, import_react.useEffect)(() => {
		const persistApi = useWallet.persist;
		const unsub = persistApi.onFinishHydration(() => setHydrated());
		persistApi.rehydrate();
		return unsub;
	}, [setHydrated]);
	if (!hydrated) return /* @__PURE__ */ (0, import_jsx_runtime.jsx)(BootScreen, {});
	if (!wallet) return /* @__PURE__ */ (0, import_jsx_runtime.jsx)(Onboarding, {});
	if (locked) return /* @__PURE__ */ (0, import_jsx_runtime.jsx)(LockScreen, {});
	return /* @__PURE__ */ (0, import_jsx_runtime.jsx)(AppShell, {});
}
//#endregion
export { AppGate as component };
