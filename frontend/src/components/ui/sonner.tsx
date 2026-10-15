import { Toaster as Sonner } from "sonner";

export function Toaster() {
  return (
    <Sonner
      theme="dark"
      position="top-center"
      toastOptions={{
        classNames: {
          toast:
            "bg-card text-foreground shadow-[var(--shadow-border),var(--shadow-float)] border-0",
          title: "text-foreground",
          description: "text-muted-foreground",
        },
      }}
    />
  );
}
