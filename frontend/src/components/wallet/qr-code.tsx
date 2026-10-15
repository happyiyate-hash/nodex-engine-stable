import { useEffect, useState } from "react";
import QRCode from "qrcode";
import { cn } from "@/lib/utils";

export function AddressQr({
  value,
  className,
}: {
  value: string;
  className?: string;
}) {
  const [svg, setSvg] = useState("");

  useEffect(() => {
    let cancelled = false;
    QRCode.toString(value, {
      type: "svg",
      margin: 1,
      color: { dark: "#f5f5f4", light: "#00000000" },
    })
      .then((out) => {
        if (!cancelled) setSvg(out);
      })
      .catch(() => {
        if (!cancelled) setSvg("");
      });
    return () => {
      cancelled = true;
    };
  }, [value]);

  return (
    <div
      className={cn(
        "flex aspect-square items-center justify-center rounded-xl bg-secondary p-4 shadow-[var(--shadow-border)]",
        className,
      )}
      aria-hidden
      dangerouslySetInnerHTML={svg ? { __html: svg } : undefined}
    />
  );
}
