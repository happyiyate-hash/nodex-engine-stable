import { useEffect } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { AppShell } from "@/components/layout/app-shell";
import { BootScreen } from "@/components/wallet/boot-screen";
import { LockScreen } from "@/components/wallet/lock-screen";
import { Onboarding } from "@/components/wallet/onboarding";
import { useWallet } from "@/lib/wallet/store";

export const Route = createFileRoute("/_app")({
  component: AppGate,
});

function AppGate() {
  const hydrated = useWallet((s) => s.hydrated);
  const setHydrated = useWallet((s) => s.setHydrated);
  const wallet = useWallet((s) => s.wallet);
  const locked = useWallet((s) => s.locked);
  const setupComplete = useWallet((s) => s.setupComplete);

  useEffect(() => {
    const persistApi = useWallet.persist;
    const unsub = persistApi.onFinishHydration(() => setHydrated());
    if (persistApi.hasHydrated()) {
      setHydrated();
    } else {
      void persistApi.rehydrate();
    }
    const t = window.setTimeout(() => setHydrated(), 80);
    return () => {
      unsub();
      window.clearTimeout(t);
    };
  }, [setHydrated]);

  if (!hydrated) return <BootScreen />;
  if (!wallet || !setupComplete) return <Onboarding />;
  if (locked) return <LockScreen />;
  return <AppShell />;
}
