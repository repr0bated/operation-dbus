// WASM decoder with JS fallback - WASM optional
export function decodePayload(input: string): string {
  try {
    return JSON.stringify(JSON.parse(input), null, 2);
  } catch {
    return input;
  }
}

export function validateJson(input: string): boolean {
  try {
    JSON.parse(input);
    return true;
  } catch {
    return false;
  }
}

// WASM init is a no-op until wasm-pack build is run
export async function initDecoder(): Promise<void> {
  // WASM module would be loaded here if built
  // For now, JS fallback is always used
}
