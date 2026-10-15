import { useEffect, useState } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { CheckCircle2, CircleAlert, RadioTower } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { getEventStreamStatus, getEventStreamToken, saveEventStreamToken, type EngineStreamStatus } from "@/lib/wallet/event-stream";

export const Route = createFileRoute("/_app/connection")({ component: ConnectionPage });

function ConnectionPage() {
  const [token, setToken] = useState("");
  const [status, setStatus] = useState<{ status: EngineStreamStatus; message?: string }>({ status: "disconnected" });
  useEffect(() => {
    setToken(getEventStreamToken());
    setStatus(getEventStreamStatus());
    const update = (event: Event) => {
      const detail = (event as CustomEvent<{ status: EngineStreamStatus; message?: string }>).detail;
      if (detail) setStatus(detail);
    };
    window.addEventListener("nodex-event-stream-status", update);
    return () => window.removeEventListener("nodex-event-stream-status", update);
  }, []);
  const connected = status.status === "connected";
  const failed = status.status === "error";

  return <div className="space-y-6 pb-4">
    <section className="space-y-4 rounded-2xl border border-border bg-card p-4">
      <div className="flex items-start gap-3"><div className="grid size-11 place-items-center rounded-2xl bg-secondary"><RadioTower className="size-5" /></div><div className="min-w-0 flex-1"><h2 className="font-semibold">Live event stream</h2><p className="mt-1 text-sm text-muted-foreground">NodeX connects automatically when the app opens. Local/embedded engines can stream without a key; only remote hosting needs a configured token.</p></div></div>
      <div className="flex items-center gap-2 rounded-xl bg-secondary/50 px-3 py-3">
        {connected ? <CheckCircle2 className="size-5 text-emerald-500" /> : <CircleAlert className={`size-5 ${failed ? "text-destructive" : "text-muted-foreground"}`} />}
        <div className="min-w-0"><p className="text-sm font-medium capitalize">{status.status}</p><p className="mt-1 break-words text-xs text-muted-foreground">{status.message ?? (connected ? "Receiving events from NodeX." : "Enter the same token configured on the engine host.")}</p></div>
      </div>
      <div className="space-y-2"><Label htmlFor="stream-token">Remote engine token (optional for local mode)</Label><Input id="stream-token" type="password" autoComplete="off" value={token} onChange={(event) => setToken(event.target.value)} placeholder="Only needed for remote hosting" /><p className="text-xs leading-relaxed text-muted-foreground">For an embedded engine, set NODEX_LOCAL_MODE=true; it binds to 127.0.0.1 and connects without a token. For a publicly hosted engine, configure NODEX_EVENT_STREAM_TOKEN on the server and enter the matching value here. Never put the server secret in a VITE_ environment variable.</p></div>
      <div className="flex flex-col gap-2 sm:flex-row"><Button className="flex-1" onClick={() => saveEventStreamToken(token)}>{token.trim() ? "Save remote token and reconnect" : "Connect without token"}</Button><Button variant="outline" onClick={() => { setToken(""); saveEventStreamToken(""); }}>Clear token</Button></div>
      <Button variant="secondary" className="w-full" asChild><Link to="/engine-logs">View request and response logs</Link></Button>
    </section>
    <section className="rounded-2xl border border-border bg-card p-4"><h2 className="font-semibold">If it will not connect</h2><ul className="mt-3 list-disc space-y-2 pl-5 text-sm leading-relaxed text-muted-foreground"><li>Confirm the engine service is running and its public URL is configured as VITE_NODEX_ENGINE_URL in the frontend build environment.</li><li>For a remote engine, confirm NODEX_EVENT_STREAM_TOKEN matches the value saved above. Local mode should run on the same device with NODEX_LOCAL_MODE=true.</li><li>If the browser reports a network or CORS error, the engine server must allow the deployed frontend origin.</li></ul></section>
  </div>;
}
