/**
 * Sable's browser bridge to the Rust/WASM wallet core.
 *
 * The generated package is produced by wasm-pack during the WASM build. This
 * module never silently falls back to JavaScript-generated wallet secrets.
 */
type NodeXWasm = {
  default: () => Promise<unknown>;
  runtime_version: () => string;
  generate_recovery_phrase: () => string;
  validate_recovery_phrase: (phrase: string) => boolean;
};

let runtimePromise: Promise<NodeXWasm> | undefined;

async function loadRuntime(): Promise<NodeXWasm> {
  if (!runtimePromise) {
    runtimePromise = (async () => {
      try {
        const wasm = await import("./wasm/pkg/nodex_wallet_wasm.js") as unknown as NodeXWasm;
        await wasm.default();
        if (typeof wasm.generate_recovery_phrase !== "function" ||
            typeof wasm.validate_recovery_phrase !== "function") {
          throw new Error("The loaded NodeX WASM package is missing required wallet functions.");
        }
        return wasm;
      } catch (error) {
        runtimePromise = undefined;
        const detail = error instanceof Error ? error.message : String(error);
        throw new Error(
          `NodeX's local WebAssembly runtime is not available. Rebuild the WASM package before using local wallet functions. (${detail})`,
        );
      }
    })();
  }
  return runtimePromise;
}

export async function localRuntimeVersion(): Promise<string> {
  return (await loadRuntime()).runtime_version();
}

export async function generateLocalRecoveryPhrase(): Promise<string> {
  return (await loadRuntime()).generate_recovery_phrase();
}

export async function validateLocalRecoveryPhrase(phrase: string): Promise<boolean> {
  return (await loadRuntime()).validate_recovery_phrase(phrase.trim().toLowerCase().split(/\s+/).filter(Boolean).join(" "));
}
