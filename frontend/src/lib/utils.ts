import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatUsd(
  value: number,
  opts?: { compact?: boolean; digits?: number },
) {
  if (!Number.isFinite(value)) return "$0.00";
  if (opts?.compact && Math.abs(value) >= 10_000) {
    return new Intl.NumberFormat("en-US", {
      style: "currency",
      currency: "USD",
      notation: "compact",
      maximumFractionDigits: 2,
    }).format(value);
  }
  const digits = opts?.digits ?? (Math.abs(value) < 1 && value !== 0 ? 4 : 2);
  return new Intl.NumberFormat("en-US", {
    style: "currency",
    currency: "USD",
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  }).format(value);
}

export function formatAmount(value: number, symbol: string) {
  if (!Number.isFinite(value)) return `0 ${symbol}`;
  const abs = Math.abs(value);
  const digits = abs === 0 ? 2 : abs < 0.001 ? 6 : abs < 1 ? 4 : abs < 100 ? 4 : 2;
  const body = new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 0,
    maximumFractionDigits: digits,
  }).format(value);
  return `${body} ${symbol}`;
}

export function formatTokenAmount(value: number) {
  if (!Number.isFinite(value)) return "0";
  const abs = Math.abs(value);
  const digits = abs === 0 ? 2 : abs < 0.001 ? 6 : abs < 1 ? 4 : abs < 1000 ? 4 : 2;
  return new Intl.NumberFormat("en-US", {
    minimumFractionDigits: 0,
    maximumFractionDigits: digits,
  }).format(value);
}

export function truncateAddress(address: string, size = 4) {
  if (address.length <= size * 2 + 2) return address;
  return `${address.slice(0, size + (address.startsWith("0x") ? 2 : 0))}…${address.slice(-size)}`;
}

export function formatRelative(ts: number) {
  const delta = Date.now() - ts;
  const min = Math.round(delta / 60_000);
  if (min < 1) return "Just now";
  if (min < 60) return `${min}m ago`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.round(hr / 24);
  if (day < 14) return `${day}d ago`;
  return new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
  }).format(new Date(ts));
}

export function formatDateHeading(ts: number) {
  const d = new Date(ts);
  const today = new Date();
  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate();
  if (sameDay(d, today)) return "Today";
  const yest = new Date(today);
  yest.setDate(today.getDate() - 1);
  if (sameDay(d, yest)) return "Yesterday";
  return new Intl.DateTimeFormat("en-US", {
    month: "long",
    day: "numeric",
    year: d.getFullYear() === today.getFullYear() ? undefined : "numeric",
  }).format(d);
}

export function randomHex(bytes: number) {
  const arr = crypto.getRandomValues(new Uint8Array(bytes));
  return Array.from(arr, (b) => b.toString(16).padStart(2, "0")).join("");
}

export async function sha256Hex(input: string | ArrayBuffer) {
  const data =
    typeof input === "string" ? new TextEncoder().encode(input) : input;
  const digest = await crypto.subtle.digest("SHA-256", data);
  return Array.from(new Uint8Array(digest), (b) =>
    b.toString(16).padStart(2, "0"),
  ).join("");
}

export async function copyText(value: string) {
  await navigator.clipboard.writeText(value);
}
