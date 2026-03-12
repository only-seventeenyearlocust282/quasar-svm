import type { Address } from "@solana/addresses";
import type { Lamports } from "@solana/rpc-types";
import type { ExecutionResult } from "../index.js";

export interface SvmAccount {
  address: Address;
  data: Uint8Array;
  executable: boolean;
  lamports: Lamports;
  programAddress: Address;
  space: bigint;
}

export type KitExecutionResult = ExecutionResult<SvmAccount>;

export type { ExecutionResult, Clock, EpochSchedule } from "../index.js";
