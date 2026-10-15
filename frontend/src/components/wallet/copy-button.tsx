import { useState } from "react";
import { Check, Copy } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { cn, copyText } from "@/lib/utils";

export function CopyButton({
  value,
  label = "Copy",
  className,
}: {
  value: string;
  label?: string;
  className?: string;
}) {
  const [copied, setCopied] = useState(false);

  async function onCopy() {
    await copyText(value);
    setCopied(true);
    toast("Copied");
    window.setTimeout(() => setCopied(false), 1400);
  }

  return (
    <Button
      type="button"
      variant="secondary"
      className={className}
      onClick={onCopy}
    >
      <span className="relative size-4">
        <Copy
          className={cn(
            "absolute inset-0 size-4 transition-[opacity,transform,filter] duration-200",
            copied ? "scale-[0.25] opacity-0 blur-[4px]" : "scale-100 opacity-100",
          )}
        />
        <Check
          className={cn(
            "absolute inset-0 size-4 transition-[opacity,transform,filter] duration-200",
            copied ? "scale-100 opacity-100" : "scale-[0.25] opacity-0 blur-[4px]",
          )}
        />
      </span>
      {label}
    </Button>
  );
}
