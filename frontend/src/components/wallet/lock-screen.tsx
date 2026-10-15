import { useState } from "react";
import { Delete } from "lucide-react";
import { SableMark } from "@/components/brand/sable-mark";
import { cn } from "@/lib/utils";
import { useWallet } from "@/lib/wallet/store";

const KEYS = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "", "0", "del"] as const;

export function LockScreen() {
  const unlock = useWallet((s) => s.unlock);
  const name = useWallet((s) => s.wallet?.name ?? "Wallet");
  const [pin, setPin] = useState("");
  const [error, setError] = useState(false);
  const [busy, setBusy] = useState(false);

  async function attempt(next: string) {
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

  function onKey(key: string) {
    if (busy) return;
    if (key === "del") {
      setPin((p) => p.slice(0, -1));
      return;
    }
    if (!key) return;
    if (pin.length >= 6) return;
    void attempt(pin + key);
  }

  return (
    <div className="flex min-h-dvh flex-col items-center justify-between bg-background px-6 py-12">
      <div className="stagger-enter flex flex-col items-center pt-10 text-center">
        <SableMark className="size-14" />
        <p className="mt-6 text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
          Locked
        </p>
        <h1 className="mt-2 text-2xl font-medium tracking-tight">{name}</h1>
        <div
          className={cn(
            "mt-8 flex gap-3",
            error && "animate-pulse",
          )}
        >
          {Array.from({ length: 6 }).map((_, i) => (
            <span
              key={i}
              className={cn(
                "size-2.5 rounded-full transition-colors duration-150",
                i < pin.length ? "bg-foreground" : "bg-foreground/20",
                error && "bg-destructive",
              )}
            />
          ))}
        </div>
        {error && (
          <p className="mt-4 text-sm text-destructive">Incorrect PIN</p>
        )}
      </div>
      <div className="grid w-full max-w-xs grid-cols-3 gap-2 pb-[env(safe-area-inset-bottom)]">
        {KEYS.map((key, i) =>
          key === "" ? (
            <span key={i} />
          ) : (
            <button
              key={key}
              type="button"
              onClick={() => onKey(key)}
              className="flex h-16 items-center justify-center rounded-lg text-xl font-medium tabular-nums transition-[background-color,transform] duration-150 hover:bg-accent active:scale-[0.96]"
            >
              {key === "del" ? <Delete className="size-5" /> : key}
            </button>
          ),
        )}
      </div>
    </div>
  );
}
