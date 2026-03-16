import type { Address } from "@solana/addresses";
import type { ExecutionResult } from "../result.js";

export interface SvmAccount {
  address: Address;
  lamports: bigint;
  data: Uint8Array;
  owner: Address;
  executable: boolean;
}

export type KitExecutionResult = ExecutionResult<SvmAccount, Address>;
