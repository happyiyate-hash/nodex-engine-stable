import { useEffect, useMemo, useState } from "react";
import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowLeft, Check, CheckCircle2, CircleAlert, Copy, Download, FileJson, RadioTower, RefreshCw, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { readEngineLogs, clearEngineLogs, type EngineLog } from "@/lib/wallet/engine";
import { readLiveEvents, getEventStreamStatus, type EngineStreamEvent, type EngineStreamStatus } from "@/lib/wallet/event-stream";

export const Route = createFileRoute("/engine-logs")({ component: EngineLogsPage });
type LogTab = "requests" | "events";
type DetailTab = "request" | "response";

function EngineLogsPage() {
  const [logs, setLogs] = useState<EngineLog[]>([]);
  const [events, setEvents] = useState<EngineStreamEvent[]>([]);
  const [tab, setTab] = useState<LogTab>("requests");
  const [selectedLog, setSelectedLog] = useState<EngineLog | null>(null);
  const [selectedEvent, setSelectedEvent] = useState<EngineStreamEvent | null>(null);
  const [detailTab, setDetailTab] = useState<DetailTab>("request");
  const [copied, setCopied] = useState(false);
  const [streamStatus, setStreamStatus] = useState<{ status: EngineStreamStatus; message?: string }>({ status: "disconnected" });
  const reload = () => { setLogs(readEngineLogs()); setEvents(readLiveEvents()); setStreamStatus(getEventStreamStatus()); };

  useEffect(() => {
    reload();
    const requestUpdate = (event: Event) => setLogs((event as CustomEvent<EngineLog[]>).detail ?? readEngineLogs());
    const eventUpdate = (event: Event) => setEvents((event as CustomEvent<EngineStreamEvent[]>).detail ?? readLiveEvents());
    const statusUpdate = (event: Event) => { const detail = (event as CustomEvent<{ status: EngineStreamStatus; message?: string }>).detail; if (detail) setStreamStatus(detail); };
    const storageUpdate = (event: StorageEvent) => { if (!event.key || event.key === "sable-engine-logs") setLogs(readEngineLogs()); };
    const refreshTimer = window.setInterval(() => setLogs(readEngineLogs()), 1000);
    window.addEventListener("sable-engine-log", requestUpdate);
    window.addEventListener("nodex-engine-event-log", eventUpdate);
    window.addEventListener("nodex-event-stream-status", statusUpdate);
    window.addEventListener("storage", storageUpdate);
    return () => {
      window.clearInterval(refreshTimer);
      window.removeEventListener("sable-engine-log", requestUpdate);
      window.removeEventListener("nodex-engine-event-log", eventUpdate);
      window.removeEventListener("nodex-event-stream-status", statusUpdate);
      window.removeEventListener("storage", storageUpdate);
    };
  }, []);

  const isDetail = Boolean(selectedLog || selectedEvent);
  const title = selectedLog?.action ?? selectedEvent?.event_type ?? "Event";
  const fullText = useMemo(() => {
    if (selectedLog) return JSON.stringify({
      action: selectedLog.action, at: selectedLog.at, durationMs: selectedLog.durationMs,
      success: selectedLog.ok, error: selectedLog.error ?? null,
      request: selectedLog.request, response: selectedLog.response ?? null,
    }, null, 2);
    return selectedEvent ? JSON.stringify(selectedEvent, null, 2) : "";
  }, [selectedLog, selectedEvent]);
  const requestText = selectedLog ? JSON.stringify(selectedLog.request ?? {}, null, 2) : fullText;
  const responseText = selectedLog ? JSON.stringify({
    success: selectedLog.ok, error: selectedLog.error ?? null,
    response: selectedLog.response ?? null,
  }, null, 2) : fullText;
  const shownText = detailTab === "request" ? requestText : responseText;
  const closeDetail = () => { setSelectedLog(null); setSelectedEvent(null); setCopied(false); };
  const copyText = async (value: string) => {
    try { await navigator.clipboard.writeText(value); setCopied(true); window.setTimeout(() => setCopied(false), 1600); }
    catch { setCopied(false); }
  };
  const download = () => {
    const blob = new Blob([fullText], { type: "application/json;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `nodex-${String(title).replace(/[^a-z0-9-_]+/gi, "-")}-log.json`;
    document.body.appendChild(anchor); anchor.click(); anchor.remove(); URL.revokeObjectURL(url);
  };

  return <main className="relative flex min-h-dvh w-full flex-col bg-background text-foreground">
    <header className="sticky top-0 z-20 border-b border-border/70 bg-background/95 px-3 pb-2 pt-2 backdrop-blur-sm sm:px-5">
      <div className="flex min-h-9 items-center gap-2">
        {isDetail ? <Button variant="ghost" size="icon" className="size-8 shrink-0 rounded-full" onClick={closeDetail} aria-label="Back to logs"><ArrowLeft className="size-4" /></Button> : <Button variant="ghost" size="icon" className="size-8 shrink-0 rounded-full" asChild><Link to="/settings" aria-label="Back to settings"><ArrowLeft className="size-4" /></Link></Button>}
        <div className="min-w-0 flex-1">
          <h1 className="truncate text-base font-semibold tracking-tight">{isDetail ? title : "Logs"}</h1>
          {isDetail && <p className="truncate text-[11px] text-muted-foreground">{selectedLog ? `${new Date(selectedLog.at).toLocaleString()} · ${selectedLog.durationMs} ms` : selectedEvent ? new Date(Number(selectedEvent.timestamp_ms)).toLocaleString() : ""}</p>}
        </div>
        {!isDetail && <Button variant="ghost" size="icon" className="size-8 rounded-full" onClick={reload} aria-label="Refresh logs"><RefreshCw className="size-4" /></Button>}
        {isDetail && <span className={`inline-flex items-center gap-1 rounded-full px-2 py-1 text-[10px] font-medium ${selectedLog?.ok ? "bg-emerald-500/10 text-emerald-500" : selectedLog ? "bg-destructive/10 text-destructive" : "bg-secondary text-muted-foreground"}`}>{selectedLog ? selectedLog.ok ? <><CheckCircle2 className="size-3" /> Success</> : <><CircleAlert className="size-3" /> Failed</> : <><RadioTower className="size-3" /> Event</>}</span>}
      </div>
      {!isDetail && <div className="mt-2 flex items-center justify-between gap-2">
        <div className="grid flex-1 grid-cols-2 gap-1 rounded-xl bg-secondary/70 p-1">
          <button onClick={() => setTab("requests")} className={`flex h-8 items-center justify-center gap-1.5 rounded-lg px-2 text-xs font-medium transition-colors ${tab === "requests" ? "bg-background text-foreground shadow-sm" : "text-muted-foreground"}`}><FileJson className="size-3.5" /> Requests <span className="text-[10px] opacity-70">{logs.length}</span></button>
          <button onClick={() => setTab("events")} className={`flex h-8 items-center justify-center gap-1.5 rounded-lg px-2 text-xs font-medium transition-colors ${tab === "events" ? "bg-background text-foreground shadow-sm" : "text-muted-foreground"}`}><RadioTower className="size-3.5" /> Live events <span className="text-[10px] opacity-70">{events.length}</span></button>
        </div>
        <Link to="/connection" className="flex h-8 shrink-0 items-center gap-1.5 rounded-lg border border-border px-2.5 text-[11px] font-medium"><span className={`size-1.5 rounded-full ${streamStatus.status === "connected" ? "bg-emerald-500" : "bg-muted-foreground"}`} />{streamStatus.status === "connected" ? "Streaming" : "Offline"}</Link>
      </div>}
      {isDetail && selectedLog && <div className="mt-2 grid grid-cols-2 gap-1 rounded-xl bg-secondary/70 p-1">
        <button onClick={() => setDetailTab("request")} className={`h-8 rounded-lg text-xs font-medium ${detailTab === "request" ? "bg-background shadow-sm" : "text-muted-foreground"}`}>Request</button>
        <button onClick={() => setDetailTab("response")} className={`h-8 rounded-lg text-xs font-medium ${detailTab === "response" ? "bg-background shadow-sm" : "text-muted-foreground"}`}>Response</button>
      </div>}
    </header>

    {isDetail ? <section className="flex min-h-0 flex-1 flex-col px-0 pb-24 pt-0">
      {selectedLog?.error && detailTab === "response" && <div className="mx-3 mt-3 flex items-start gap-2 border-l-2 border-destructive px-3 py-2 text-sm text-destructive"><CircleAlert className="mt-0.5 size-4 shrink-0" /><span className="break-words">{selectedLog.error}</span></div>}
      <pre aria-label={selectedEvent ? "Event JSON" : `${detailTab} JSON`} className="min-h-[60dvh] flex-1 overflow-auto bg-background px-3 pb-5 pt-3 font-mono text-[11px] leading-[1.75] [overflow-wrap:anywhere] sm:px-5 sm:text-xs">{shownText}</pre>
      <div className="fixed inset-x-0 bottom-0 z-30 flex justify-center border-t border-border/70 bg-background/90 px-4 py-3 backdrop-blur-md">
        <div className="flex w-full max-w-md items-center gap-2 rounded-2xl border border-border bg-card p-1.5 shadow-lg">
          <Button className="h-9 flex-1 rounded-xl text-xs" onClick={() => void copyText(detailTab === "request" ? requestText : responseText)}><Copy className="mr-1.5 size-3.5" />{copied ? "Copied" : "Copy"}</Button>
          <Button variant="secondary" className="h-9 flex-1 rounded-xl text-xs" onClick={download}><Download className="mr-1.5 size-3.5" />Download JSON</Button>
          <Button variant="ghost" size="icon" className="size-9 shrink-0 rounded-xl" onClick={closeDetail} aria-label="Close log"><X className="size-4" /></Button>
        </div>
      </div>
    </section> : <section className="flex-1 px-3 pb-5 pt-2 sm:px-5">
      {tab === "requests" ? logs.length === 0 ? <Empty title="No requests yet" /> : <div className="space-y-1.5">{logs.map((log) => <button key={log.id} onClick={() => { setSelectedLog(log); setSelectedEvent(null); setDetailTab("request"); }} className="flex w-full items-center gap-2.5 rounded-xl border border-border/70 bg-card px-3 py-2.5 text-left transition-colors hover:border-foreground/20 active:bg-secondary/60">
        <span className={`grid size-8 shrink-0 place-items-center rounded-lg ${log.ok ? "bg-emerald-500/10 text-emerald-500" : "bg-destructive/10 text-destructive"}`}>{log.ok ? <Check className="size-4" /> : <CircleAlert className="size-4" />}</span>
        <span className="min-w-0 flex-1"><span className="block truncate text-[13px] font-medium">{log.action}</span><span className="mt-0.5 block truncate text-[10px] text-muted-foreground">{new Date(log.at).toLocaleString()} · {log.durationMs} ms</span>{!log.ok && <span className="mt-1 block truncate text-[10px] text-destructive">{log.error ?? "Request failed — open to inspect response"}</span>}</span>
        <span className={`shrink-0 text-[10px] font-medium ${log.ok ? "text-emerald-500" : "text-destructive"}`}>{log.ok ? "OK" : "Failed"}</span>
      </button>)}</div> : events.length === 0 ? <Empty title="No live events yet" /> : <div className="space-y-1.5">{events.map((event) => <button key={`${event.event_id}-${event.timestamp_ms}`} onClick={() => { setSelectedEvent(event); setSelectedLog(null); }} className="flex w-full items-center gap-2.5 rounded-xl border border-border/70 bg-card px-3 py-2.5 text-left transition-colors hover:border-foreground/20 active:bg-secondary/60">
        <span className={`grid size-8 shrink-0 place-items-center rounded-lg ${streamStatus.status === "connected" ? "bg-emerald-500/10 text-emerald-500" : "bg-secondary text-muted-foreground"}`}><RadioTower className="size-4" /></span>
        <span className="min-w-0 flex-1"><span className="block truncate text-[13px] font-medium">{event.event_type}</span><span className="mt-0.5 block truncate text-[10px] text-muted-foreground">{new Date(Number(event.timestamp_ms)).toLocaleString()} · {event.chain_id ?? event.data?.chain_id ?? event.data?.chain_name ?? event.data?.blockchain ?? "Engine"}</span></span>
        <span className="shrink-0 text-[10px] text-muted-foreground">#{event.event_id}</span>
      </button>)}</div>}
      {tab === "requests" && logs.length > 0 && <div className="mt-3 flex justify-end"><Button variant="ghost" size="sm" className="h-8 rounded-lg text-xs text-muted-foreground" onClick={() => { clearEngineLogs(); setLogs([]); }}><Trash2 className="mr-1.5 size-3.5" />Clear logs</Button></div>}
    </section>}
  </main>;
}

function Empty({ title }: { title: string }) {
  return <div className="flex min-h-[42dvh] flex-col items-center justify-center text-center"><span className="mb-3 grid size-11 place-items-center rounded-2xl bg-secondary"><FileJson className="size-5 text-muted-foreground" /></span><p className="text-sm font-medium">{title}</p><Link to="/connection" className="mt-2 text-xs text-muted-foreground underline underline-offset-4">Check connection</Link></div>;
}
