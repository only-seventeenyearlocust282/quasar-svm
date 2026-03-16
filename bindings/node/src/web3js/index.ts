import { PublicKey } from "@solana/web3.js";
import type { TransactionInstruction } from "@solana/web3.js";
import * as ffi from "../ffi.js";
import {
  serializeInstructions,
  serializeAccounts,
  deserializeResult,
} from "./wire.js";
import { ExecutionResult } from "../result.js";
import type { Clock, EpochSchedule } from "../index.js";
import type { SvmAccount, Web3ExecutionResult } from "./types.js";
import { uniqueAddress } from "../address.js";
import {
  SPL_TOKEN_PROGRAM_ID,
  SPL_TOKEN_2022_PROGRAM_ID,
  SPL_ASSOCIATED_TOKEN_PROGRAM_ID,
  SYSTEM_PROGRAM_ID,
  LOADER_V2,
  LOADER_V3,
  loadElf,
} from "../programs.js";
import {
  packMint, packTokenAccount, rentMinimumBalance,
  tokenTransferData, tokenMintToData, tokenBurnData,
  MINT_LEN, TOKEN_ACCOUNT_LEN,
} from "../token.js";
import type { TokenAccountState } from "../token.js";

export type { SvmAccount, Web3ExecutionResult } from "./types.js";
export { toKeyedAccountInfo, fromKeyedAccountInfo } from "./types.js";
export { ExecutionResult } from "../result.js";
export type { ExecutionStatus, ProgramError, AccountDiff, Clock, EpochSchedule } from "../index.js";
export { SPL_TOKEN_PROGRAM_ID, SPL_TOKEN_2022_PROGRAM_ID, SPL_ASSOCIATED_TOKEN_PROGRAM_ID, LOADER_V2, LOADER_V3 } from "../programs.js";
export { TokenAccountState } from "../token.js";
export type { MintData, TokenAccountData } from "../token.js";

// ---------------------------------------------------------------------------
// Opts
// ---------------------------------------------------------------------------

export interface MintOpts {
  mintAuthority?: PublicKey;
  supply?: bigint;
  decimals?: number;
  freezeAuthority?: PublicKey;
}

export interface TokenAccountOpts {
  mint: PublicKey;
  owner: PublicKey;
  amount: bigint;
  delegate?: PublicKey;
  state?: TokenAccountState;
  isNative?: bigint;
  delegatedAmount?: bigint;
  closeAuthority?: PublicKey;
}

// ---------------------------------------------------------------------------
// QuasarSvm
// ---------------------------------------------------------------------------

const findAccount = (accounts: SvmAccount[], address: PublicKey) =>
  accounts.find(a => a.address.equals(address));

export class QuasarSvm {
  private ptr: unknown;
  private freed = false;

  constructor() {
    this.ptr = ffi.quasar_svm_new();
    if (!this.ptr) {
      throw new Error(
        `Failed to create QuasarSvm: ${ffi.quasar_last_error() ?? "unknown"}`
      );
    }
  }

  /** Release native resources. Call when done with the VM. */
  free(): void {
    if (!this.freed) {
      ffi.quasar_svm_free(this.ptr);
      this.freed = true;
    }
  }

  addProgram(programId: PublicKey, elf: Uint8Array, loaderVersion = LOADER_V3): this {
    this.check(
      ffi.quasar_svm_add_program(
        this.ptr,
        programId.toBuffer(),
        Buffer.from(elf),
        elf.length,
        loaderVersion
      )
    );
    return this;
  }

  addTokenProgram(): this {
    return this.addProgram(new PublicKey(SPL_TOKEN_PROGRAM_ID), loadElf("spl_token.so"), LOADER_V2);
  }

  addToken2022Program(): this {
    return this.addProgram(new PublicKey(SPL_TOKEN_2022_PROGRAM_ID), loadElf("spl_token_2022.so"), LOADER_V3);
  }

  addAssociatedTokenProgram(): this {
    return this.addProgram(new PublicKey(SPL_ASSOCIATED_TOKEN_PROGRAM_ID), loadElf("spl_associated_token.so"), LOADER_V2);
  }

  // ---------- Account store ----------

  /** Store an account in the SVM's persistent account database. */
  setAccount(account: SvmAccount): void {
    const dataBuf = account.data.length > 0 ? Buffer.from(account.data) : null;
    this.check(
      ffi.quasar_svm_set_account(
        this.ptr,
        account.address.toBuffer(),
        account.owner.toBuffer(),
        BigInt(account.lamports),
        dataBuf,
        account.data.length,
        account.executable
      )
    );
  }

  /** Read an account from the SVM's persistent account database. */
  getAccount(pubkey: PublicKey): SvmAccount | null {
    const ptrOut = [null as unknown];
    const lenOut = [BigInt(0)];
    const code = ffi.quasar_svm_get_account(this.ptr, pubkey.toBuffer(), ptrOut, lenOut);
    if (code !== 0) return null;

    const resultPtr = ptrOut[0];
    const resultLen = Number(lenOut[0]);
    const buf = Buffer.from(ffi.koffi.decode(resultPtr, "uint8_t", resultLen));
    ffi.quasar_result_free(resultPtr, resultLen);

    let o = 0;
    const address = new PublicKey(buf.subarray(o, o + 32));
    o += 32;
    const owner = new PublicKey(buf.subarray(o, o + 32));
    o += 32;
    const lamports = buf.readBigUInt64LE(o);
    o += 8;
    const dLen = buf.readUInt32LE(o);
    o += 4;
    const data = Buffer.from(buf.subarray(o, o + dLen));
    o += dLen;
    const executable = buf[o] !== 0;

    return { address, lamports, data, owner, executable };
  }

  /** Give lamports to an account, creating it if it doesn't exist. */
  airdrop(pubkey: PublicKey, lamports: bigint): void {
    this.check(ffi.quasar_svm_airdrop(this.ptr, pubkey.toBuffer(), lamports));
  }

  /** Create a rent-exempt account with the given space and owner. */
  createAccount(pubkey: PublicKey, space: bigint, owner: PublicKey): void {
    this.check(
      ffi.quasar_svm_create_account(this.ptr, pubkey.toBuffer(), space, owner.toBuffer())
    );
  }

  // ---------- Cheatcodes ----------

  /** Set the token balance (amount) of an existing token account in the store. */
  setTokenBalance(address: PublicKey, amount: bigint): void {
    this.check(ffi.quasar_svm_set_token_balance(this.ptr, address.toBuffer(), amount));
  }

  /** Set the supply of an existing mint account in the store. */
  setMintSupply(address: PublicKey, supply: bigint): void {
    this.check(ffi.quasar_svm_set_mint_supply(this.ptr, address.toBuffer(), supply));
  }

  /** Set the clock's unix_timestamp. Does not advance slot or epoch. */
  warpToTimestamp(timestamp: bigint): void {
    this.check(ffi.quasar_svm_warp_to_timestamp(this.ptr, timestamp));
  }

  // ---------- Sysvars ----------

  setClock(opts: Clock): void {
    this.check(
      ffi.quasar_svm_set_clock(
        this.ptr,
        opts.slot,
        opts.epochStartTimestamp,
        opts.epoch,
        opts.leaderScheduleEpoch,
        opts.unixTimestamp
      )
    );
  }

  warpToSlot(slot: bigint): void {
    this.check(ffi.quasar_svm_warp_to_slot(this.ptr, slot));
  }

  setRent(lamportsPerByte: bigint): void {
    this.check(ffi.quasar_svm_set_rent(this.ptr, lamportsPerByte, 1.0, 0));
  }

  setEpochSchedule(opts: EpochSchedule): void {
    this.check(
      ffi.quasar_svm_set_epoch_schedule(
        this.ptr,
        opts.slotsPerEpoch,
        opts.leaderScheduleSlotOffset,
        opts.warmup,
        opts.firstNormalEpoch,
        opts.firstNormalSlot
      )
    );
  }

  setComputeBudget(maxUnits: bigint): void {
    this.check(ffi.quasar_svm_set_compute_budget(this.ptr, maxUnits));
  }

  // ---------- Execution ----------

  /** Execute instructions as a single atomic transaction. */
  processTransaction(
    instructions: TransactionInstruction | TransactionInstruction[],
    accounts: SvmAccount[]
  ): Web3ExecutionResult {
    const ixs = Array.isArray(instructions) ? instructions : [instructions];
    return this.exec(
      ffi.quasar_svm_process_transaction,
      serializeInstructions(ixs),
      serializeAccounts(accounts)
    );
  }

  /** Execute a transaction without committing any state changes. */
  simulateTransaction(
    instructions: TransactionInstruction | TransactionInstruction[],
    accounts: SvmAccount[]
  ): Web3ExecutionResult {
    const ixs = Array.isArray(instructions) ? instructions : [instructions];
    return this.exec(
      ffi.quasar_svm_simulate_transaction,
      serializeInstructions(ixs),
      serializeAccounts(accounts)
    );
  }

  // ---------- Internal ----------

  private check(code: number): void {
    if (code !== 0) {
      throw new Error(
        `QuasarSvm error (${code}): ${ffi.quasar_last_error() ?? "unknown"}`
      );
    }
  }

  private exec(
    fn: Function,
    ixBuf: Buffer,
    acctBuf: Buffer
  ): Web3ExecutionResult {
    const ptrOut = [null as unknown];
    const lenOut = [BigInt(0)];

    const code = fn(
      this.ptr,
      ixBuf,
      ixBuf.length,
      acctBuf,
      acctBuf.length,
      ptrOut,
      lenOut
    );

    if (code !== 0) {
      throw new Error(
        `Execution error (${code}): ${ffi.quasar_last_error() ?? "unknown"}`
      );
    }

    const resultPtr = ptrOut[0];
    const resultLen = Number(lenOut[0]);
    const resultBuf = Buffer.from(
      ffi.koffi.decode(resultPtr, "uint8_t", resultLen)
    );

    ffi.quasar_result_free(resultPtr, resultLen);
    const raw = deserializeResult(resultBuf);
    return new ExecutionResult(raw, findAccount);
  }
}

// ---------------------------------------------------------------------------
// Account factories
// ---------------------------------------------------------------------------

/** Create a system-owned account with the given lamports. Address auto-generated if omitted. */
export function createSystemAccount(lamports: bigint): SvmAccount;
export function createSystemAccount(address: PublicKey, lamports: bigint): SvmAccount;
export function createSystemAccount(addressOrLamports: PublicKey | bigint, lamports?: bigint): SvmAccount {
  let addr: PublicKey;
  let sol: bigint;
  if (addressOrLamports instanceof PublicKey) {
    addr = addressOrLamports;
    sol = lamports!;
  } else {
    addr = new PublicKey(uniqueAddress());
    sol = addressOrLamports;
  }
  return {
    address: addr,
    owner: new PublicKey(SYSTEM_PROGRAM_ID),
    lamports: sol,
    data: Buffer.alloc(0),
    executable: false,
  };
}

/** Create a pre-initialized mint account. Address auto-generated if omitted. */
export function createMintAccount(opts?: MintOpts, tokenProgramId?: PublicKey): SvmAccount;
export function createMintAccount(address: PublicKey, opts?: MintOpts, tokenProgramId?: PublicKey): SvmAccount;
export function createMintAccount(
  first?: PublicKey | MintOpts,
  second?: MintOpts | PublicKey,
  third?: PublicKey,
): SvmAccount {
  let addr: PublicKey;
  let opts: MintOpts;
  let programId: PublicKey;

  if (first instanceof PublicKey) {
    addr = first;
    opts = (second && !(second instanceof PublicKey)) ? second : {};
    programId = third ?? (second instanceof PublicKey ? second : undefined) ?? new PublicKey(SPL_TOKEN_PROGRAM_ID);
  } else {
    addr = new PublicKey(uniqueAddress());
    opts = first ?? {};
    programId = second instanceof PublicKey ? second : new PublicKey(SPL_TOKEN_PROGRAM_ID);
  }

  const data = packMint({
    mintAuthority: opts.mintAuthority?.toBuffer(),
    supply: opts.supply,
    decimals: opts.decimals,
    freezeAuthority: opts.freezeAuthority?.toBuffer(),
  });
  return {
    address: addr,
    owner: programId,
    lamports: rentMinimumBalance(MINT_LEN),
    data,
    executable: false,
  };
}

/** Create a pre-initialized token account. Address auto-generated if omitted. */
export function createTokenAccount(opts: TokenAccountOpts, tokenProgramId?: PublicKey): SvmAccount;
export function createTokenAccount(address: PublicKey, opts: TokenAccountOpts, tokenProgramId?: PublicKey): SvmAccount;
export function createTokenAccount(
  first: PublicKey | TokenAccountOpts,
  second?: TokenAccountOpts | PublicKey,
  third?: PublicKey,
): SvmAccount {
  let addr: PublicKey;
  let opts: TokenAccountOpts;
  let programId: PublicKey;

  if (first instanceof PublicKey) {
    addr = first;
    opts = second as TokenAccountOpts;
    programId = third ?? new PublicKey(SPL_TOKEN_PROGRAM_ID);
  } else {
    addr = new PublicKey(uniqueAddress());
    opts = first;
    programId = second instanceof PublicKey ? second : new PublicKey(SPL_TOKEN_PROGRAM_ID);
  }

  const data = packTokenAccount({
    mint: opts.mint.toBuffer(),
    owner: opts.owner.toBuffer(),
    amount: opts.amount,
    delegate: opts.delegate?.toBuffer(),
    state: opts.state,
    isNative: opts.isNative,
    delegatedAmount: opts.delegatedAmount,
    closeAuthority: opts.closeAuthority?.toBuffer(),
  });
  return {
    address: addr,
    owner: programId,
    lamports: rentMinimumBalance(TOKEN_ACCOUNT_LEN),
    data,
    executable: false,
  };
}

/** Create a pre-initialized associated token account. Derives the ATA address automatically. */
export function createAssociatedTokenAccount(
  owner: PublicKey,
  mint: PublicKey,
  amount: bigint,
  tokenProgramId = new PublicKey(SPL_TOKEN_PROGRAM_ID),
): SvmAccount {
  const [ata] = PublicKey.findProgramAddressSync(
    [owner.toBuffer(), tokenProgramId.toBuffer(), mint.toBuffer()],
    new PublicKey(SPL_ASSOCIATED_TOKEN_PROGRAM_ID),
  );
  const data = packTokenAccount({
    mint: mint.toBuffer(),
    owner: owner.toBuffer(),
    amount,
  });
  return {
    address: ata,
    owner: tokenProgramId,
    lamports: rentMinimumBalance(TOKEN_ACCOUNT_LEN),
    data,
    executable: false,
  };
}

// ---------------------------------------------------------------------------
// Token instruction builders
// ---------------------------------------------------------------------------

/** Build an SPL Token Transfer instruction. */
export function tokenTransfer(
  source: PublicKey, destination: PublicKey, authority: PublicKey,
  amount: bigint, tokenProgramId = new PublicKey(SPL_TOKEN_PROGRAM_ID),
): TransactionInstruction {
  return {
    programId: tokenProgramId,
    keys: [
      { pubkey: source, isSigner: false, isWritable: true },
      { pubkey: destination, isSigner: false, isWritable: true },
      { pubkey: authority, isSigner: true, isWritable: false },
    ],
    data: tokenTransferData(amount),
  };
}

/** Build an SPL Token MintTo instruction. */
export function tokenMintTo(
  mint: PublicKey, destination: PublicKey, mintAuthority: PublicKey,
  amount: bigint, tokenProgramId = new PublicKey(SPL_TOKEN_PROGRAM_ID),
): TransactionInstruction {
  return {
    programId: tokenProgramId,
    keys: [
      { pubkey: mint, isSigner: false, isWritable: true },
      { pubkey: destination, isSigner: false, isWritable: true },
      { pubkey: mintAuthority, isSigner: true, isWritable: false },
    ],
    data: tokenMintToData(amount),
  };
}

/** Build an SPL Token Burn instruction. */
export function tokenBurn(
  source: PublicKey, mint: PublicKey, authority: PublicKey,
  amount: bigint, tokenProgramId = new PublicKey(SPL_TOKEN_PROGRAM_ID),
): TransactionInstruction {
  return {
    programId: tokenProgramId,
    keys: [
      { pubkey: source, isSigner: false, isWritable: true },
      { pubkey: mint, isSigner: false, isWritable: true },
      { pubkey: authority, isSigner: true, isWritable: false },
    ],
    data: tokenBurnData(amount),
  };
}
