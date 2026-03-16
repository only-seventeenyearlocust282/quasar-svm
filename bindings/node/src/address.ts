let counter = 0;

/** Generate a unique 32-byte address for test accounts. Deterministic, not cryptographic. */
export function uniqueAddress(): Uint8Array {
  const buf = new Uint8Array(32);
  const view = new DataView(buf.buffer);
  view.setUint32(0, ++counter, true);
  return buf;
}
