import path from "path";
import fs from "fs";

// Program IDs
export const SPL_TOKEN_PROGRAM_ID = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
export const SPL_TOKEN_2022_PROGRAM_ID = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
export const SPL_ASSOCIATED_TOKEN_PROGRAM_ID = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
export const SYSTEM_PROGRAM_ID = "11111111111111111111111111111111";

// Loader versions
export const LOADER_V2 = 2;
export const LOADER_V3 = 3;

// SOL / token amount helpers
export const LAMPORTS_PER_SOL = 1_000_000_000n;

/** Convert a SOL amount to lamports. Supports fractional amounts like `sol(0.5)`. */
export function sol(amount: number | bigint): bigint {
  if (typeof amount === "bigint") return amount * LAMPORTS_PER_SOL;
  return BigInt(Math.round(amount * 1_000_000_000));
}

/** Convert a human-readable token amount to raw units given decimals. Supports fractional amounts. */
export function tokens(amount: number | bigint, decimals: number): bigint {
  const factor = 10n ** BigInt(decimals);
  if (typeof amount === "bigint") return amount * factor;
  return BigInt(Math.round(amount * Number(factor)));
}

const programsDir = path.resolve(__dirname, "..", "programs");

export function loadElf(name: string): Uint8Array {
  return fs.readFileSync(path.join(programsDir, name));
}
