import { createRootRoute, HeadContent, Outlet, Scripts, useRouterState } from "@tanstack/react-router";
import { useEffect } from "react";
import { useWallet } from "@/lib/wallet/store";
import { AuthProvider } from "@/lib/auth/provider";
import { PreviewHostBridge } from "@/components/preview-host-bridge";
import { Toaster } from "@/components/ui/sonner";
import appCss from "../styles.css?url";

const APP_NAME = "Sable";

export const Route = createRootRoute({
  head: () => ({
    meta: [
      { charSet: "utf-8" },
      { name: "viewport", content: "width=device-width, initial-scale=1, viewport-fit=cover" },
      { title: APP_NAME },
      {
        name: "description",
        content: "Sable is a private, monochrome wallet for digital assets.",
      },
      { name: "theme-color", content: "#09090b" },
    ],
    links: [
      { rel: "icon", type: "image/svg+xml", href: "/favicon.svg" },
      { rel: "stylesheet", href: appCss },
      { rel: "manifest", href: "/__grok/manifest.webmanifest" },
      { rel: "apple-touch-icon", href: "/favicon.svg" },
      { rel: "preconnect", href: "https://fonts.googleapis.com" },
      { rel: "preconnect", href: "https://fonts.gstatic.com", crossOrigin: "anonymous" },
      {
        rel: "stylesheet",
        href: "https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=Instrument+Sans:wght@400;500;600;700&display=swap",
      },
    ],
  }),
  component: RootDocument,
});

function RootDocument() {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const wallet = useWallet((state) => state.wallet);
  const locked = useWallet((state) => state.locked);
  const setupComplete = useWallet((state) => state.setupComplete);
  useEffect(() => {
    const color = pathname === "/" && wallet && setupComplete && !locked ? "#111214" : "#09090b";
    const meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.setAttribute("content", color);
    document.documentElement.style.backgroundColor = color;
    document.body.style.backgroundColor = color;
  }, [pathname, wallet, setupComplete, locked]);
  return (
    <html lang="en" className="dark antialiased" suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body>
        <PreviewHostBridge />
        <AuthProvider>
          <Outlet />
        </AuthProvider>
        <Toaster />
        <Scripts />
      </body>
    </html>
  );
}
