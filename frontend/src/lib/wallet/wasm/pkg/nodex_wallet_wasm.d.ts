declare module "@/lib/wallet/wasm/pkg/nodex_wallet_wasm.js" {
  export default function init(): Promise<unknown>;
  export function runtime_version(): string;
  export function generate_recovery_phrase(): string;
  export function validate_recovery_phrase(phrase: string): boolean;
}
