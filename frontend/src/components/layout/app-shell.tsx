import { useEffect } from "react";
import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { ArrowLeftRight, Clock, Home, Settings } from "lucide-react";
import { SableWordmark } from "@/components/brand/sable-mark";
import { TooltipProvider } from "@/components/ui/tooltip";
import { cn, truncateAddress } from "@/lib/utils";
import { usePrices } from "@/lib/wallet/prices";
import { useWallet } from "@/lib/wallet/store";
import { connectEngineEventStream, getEventStreamToken } from "@/lib/wallet/event-stream";

function CryptoSwapIcon({ className }: { className?: string }) {
  return (
    <svg
      aria-hidden="true"
      className={className}
      viewBox="0 0 100 100"
      fill="none"
    >
      <circle cx="30" cy="34" r="19" stroke="currentColor" strokeWidth="4" />
      <path
        d="M26 25H33C38 25 38 32 33 32H26M26 32H34C40 32 40 40 34 40H25M30 21V44"
        stroke="currentColor"
        strokeWidth="3"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx="70" cy="66" r="19" stroke="currentColor" strokeWidth="4" />
      <path
        d="M70 53L80 66L70 79L60 66Z"
        stroke="currentColor"
        strokeWidth="3"
        strokeLinejoin="round"
      />
      <path
        d="M48 25H67Q76 25 76 34V38M70 32L76 39L82 32"
        stroke="currentColor"
        strokeWidth="4"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M52 75H33Q24 75 24 66V62M30 68L24 61L18 68"
        stroke="currentColor"
        strokeWidth="4"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function HomeNavIcon({ className }: { className?: string }) {
  return (
    <svg
      aria-hidden="true"
      className={className}
      viewBox="0 0 64 64"
      fill="none"
    >
      <path
        d="M14 25L28.8 13.5Q32 10.8 35.2 13.5 L50 25Q52 27 52 30V47 Q52 52 46 52H18Q12 52 12 47V30 Q12 27 14 25Z"
        fill="currentColor"
      />
      <rect
        x="24.5"
        y="43.5"
        width="15"
        height="3.2"
        rx="1.6"
        fill="#0e111b"
      />
    </svg>
  );
}

const NAV = [
  { to: "/", label: "Home", icon: Home },
  { to: "/swap", label: "Swap", icon: ArrowLeftRight },
  { to: "/activity", label: "Activity", icon: Clock },
  { to: "/settings", label: "Settings", icon: Settings },
] as const;

export function AppShell() {
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const wallet = useWallet((s) => s.wallet);
  const lock = useWallet((s) => s.lock);
  const refresh = usePrices((s) => s.refresh);
  const applyEngineEvent = useWallet((s) => s.applyEngineEvent);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refresh(), 60_000);
    return () => window.clearInterval(id);
  }, [refresh]);

  useEffect(() => {
    let disconnect: (() => void) | undefined;
    const connect = () => {
      disconnect?.();
      disconnect = undefined;
      // Always subscribe when the app mounts. Embedded/local engines can stream
      // without a token; remote deployments may still use a configured token.
      const token = getEventStreamToken();
      disconnect = connectEngineEventStream(token, applyEngineEvent);
    };
    connect();
    window.addEventListener("nodex-event-token-changed", connect);
    return () => {
      window.removeEventListener("nodex-event-token-changed", connect);
      disconnect?.();
    };
  }, [applyEngineEvent]);

  useEffect(() => {
    if (!wallet?.pinHash) return;
    let timer = 0;
    const bump = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => lock(), 5 * 60 * 1000);
    };
    bump();
    window.addEventListener("pointerdown", bump);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener("pointerdown", bump);
    };
  }, [wallet?.pinHash, lock]);

  return (
    <TooltipProvider delayDuration={200}>
      <div className="min-h-dvh bg-background text-foreground">
        <aside className="fixed inset-y-0 left-0 hidden w-56 flex-col border-r border-border px-4 py-6 lg:flex">
          <SableWordmark className="px-2" />
          <nav className="mt-10 flex flex-1 flex-col gap-1">
            {NAV.map((item) => {
              const active = item.to === "/" ? pathname === "/" : pathname.startsWith(item.to);
              const Icon = item.icon;
              return <Link key={item.to} to={item.to} className={cn("flex h-11 items-center gap-3 rounded-md px-3 text-sm font-medium transition-colors duration-150", active ? "bg-accent text-foreground" : "text-muted-foreground hover:bg-accent hover:text-foreground")}><Icon className="size-4" />{item.label}</Link>;
            })}
          </nav>
          {wallet && <div className="rounded-lg bg-secondary px-3 py-3 shadow-[var(--shadow-border)]"><p className="text-sm font-medium">{wallet.name}</p><p className="mt-1 font-mono text-xs text-muted-foreground">{truncateAddress(String(wallet.chainAddresses[0]?.address ?? ""), 5)}</p></div>}
        </aside>
        <div className={cn(pathname !== "/" && "lg:pl-56")}><main className={cn("mx-auto min-h-dvh w-full", pathname === "/" ? "max-w-none px-0 pb-0 pt-0 lg:max-w-none lg:pb-0 lg:pt-0" : "max-w-xl px-5 pb-28 pt-0 lg:max-w-2xl lg:pb-12 lg:pt-10")}><Outlet /></main></div>
        <nav aria-label="Main navigation" className="fixed inset-x-3 bottom-[calc(env(safe-area-inset-bottom)+0.5rem)] z-40 flex items-stretch gap-2 lg:hidden">
          <div className="flex h-[60px] flex-1 items-center rounded-full border border-white/20 bg-secondary/65 p-1.5 shadow-[0_0_0_1px_rgba(255,255,255,0.06),inset_0_1px_0_rgba(255,255,255,0.22),0_-6px_24px_rgba(0,0,0,0.28)] backdrop-blur-2xl">
            <div className="grid h-full w-full grid-cols-3">
              {NAV.filter((item) => item.to !== "/swap" && item.to !== "/settings").map((item) => {
                const active = item.to === "/" ? pathname === "/" : pathname.startsWith(item.to);
                const Icon = item.icon;
                return <Link key={item.to} to={item.to} aria-current={active ? "page" : undefined} aria-label={item.label} title={item.label} className={cn("flex h-full items-center justify-center rounded-full transition-colors", active ? "bg-background/75 text-foreground shadow-[inset_0_1px_0_rgba(255,255,255,0.12)]" : "text-muted-foreground hover:text-foreground")}>
                  {item.to === "/" ? <HomeNavIcon className="size-[21px]" /> : <Icon className="size-[21px]" />}
                </Link>;
              })}
              {(() => {
                const item = NAV[3];
                const active = pathname.startsWith(item.to);
                const Icon = item.icon;
                return <Link key={item.to} to={item.to} aria-current={active ? "page" : undefined} aria-label={item.label} title={item.label} className={cn("flex h-full items-center justify-center rounded-full transition-colors", active ? "bg-background/75 text-foreground" : "text-muted-foreground hover:text-foreground")}>
                  <Icon className="size-[21px]" />
                </Link>;
              })()}
            </div>
          </div>
          {(() => {
            const item = NAV[1];
            const active = pathname.startsWith(item.to);
            const Icon = item.icon;
            return <Link to={item.to} aria-current={active ? "page" : undefined} aria-label={item.label} title={item.label} className={cn("grid size-[60px] shrink-0 place-items-center rounded-full border border-white/25 bg-secondary/65 text-muted-foreground shadow-[0_0_0_1px_rgba(255,255,255,0.06),inset_0_1px_0_rgba(255,255,255,0.22),0_-6px_24px_rgba(0,0,0,0.28)] backdrop-blur-2xl transition-colors", active && "text-foreground")}>
              <CryptoSwapIcon className="size-[36px]" />
            </Link>;
          })()}
        </nav>     </div>
    </TooltipProvider>
  );
}
