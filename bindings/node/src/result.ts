import type { AccountDiff, ExecutionStatus, ProgramError } from "./index.js";
import { unpackMint, unpackTokenAccount, TokenAccountState } from "./token.js";

export type AccountFinder<TAccount, TAddress> = (
  accounts: TAccount[],
  address: TAddress,
) => TAccount | undefined;

export type AddressDecoder<TAddress> = (bytes: Uint8Array) => TAddress;

// ---------------------------------------------------------------------------
// Typed token types — pubkey fields use the layer's native address type
// ---------------------------------------------------------------------------

export interface Mint<TAddress = unknown> {
  mintAuthority: TAddress | null;
  supply: bigint;
  decimals: number;
  freezeAuthority: TAddress | null;
}

export interface Token<TAddress = unknown> {
  mint: TAddress;
  owner: TAddress;
  amount: bigint;
  delegate: TAddress | null;
  state: TokenAccountState;
  isNative: bigint | null;
  delegatedAmount: bigint;
  closeAuthority: TAddress | null;
}

// ---------------------------------------------------------------------------
// ExecutionResult
// ---------------------------------------------------------------------------

export class ExecutionResult<
  TAccount extends { address: unknown; data: Uint8Array; lamports: bigint },
  TAddress = TAccount["address"],
> {
  readonly status: ExecutionStatus;
  readonly computeUnits: bigint;
  readonly executionTimeUs: bigint;
  readonly returnData: Uint8Array;
  readonly accounts: TAccount[];
  readonly modifiedAccounts: AccountDiff<TAccount>[];
  readonly logs: string[];
  private readonly findFn: AccountFinder<TAccount, TAddress>;
  private readonly decodeFn: AddressDecoder<TAddress>;

  constructor(
    fields: {
      status: ExecutionStatus;
      computeUnits: bigint;
      executionTimeUs: bigint;
      returnData: Uint8Array;
      accounts: TAccount[];
      modifiedAccounts: AccountDiff<TAccount>[];
      logs: string[];
    },
    findAccount: AccountFinder<TAccount, TAddress>,
    decodeAddress: AddressDecoder<TAddress>,
  ) {
    this.status = fields.status;
    this.computeUnits = fields.computeUnits;
    this.executionTimeUs = fields.executionTimeUs;
    this.returnData = fields.returnData;
    this.accounts = fields.accounts;
    this.modifiedAccounts = fields.modifiedAccounts;
    this.logs = fields.logs;
    this.findFn = findAccount;
    this.decodeFn = decodeAddress;
  }

  isSuccess(): boolean {
    return this.status.ok;
  }

  isError(): boolean {
    return !this.status.ok;
  }

  assertSuccess(): void {
    if (!this.status.ok) {
      const err = this.status.error;
      throw new Error(
        `expected success, got ${err.type}: ${JSON.stringify(err)}\n\nLogs:\n${this.logs.join("\n")}`
      );
    }
  }

  assertError(expected: ProgramError): void {
    if (this.status.ok) {
      throw new Error(
        `expected error ${JSON.stringify(expected)}, but execution succeeded`
      );
    }
    const actual = this.status.error;
    if (actual.type !== expected.type) {
      throw new Error(
        `expected error ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`
      );
    }
    if (
      "code" in expected &&
      "code" in actual &&
      actual.code !== expected.code
    ) {
      throw new Error(
        `expected error code ${expected.code}, got ${actual.code}`
      );
    }
  }

  assertCustomError(code: number): void {
    this.assertError({ type: "Custom", code });
  }

  printLogs(): void {
    for (const log of this.logs) console.log(log);
  }

  account(address: TAddress): TAccount | null {
    return this.findFn(this.accounts, address) ?? null;
  }

  lamports(address: TAddress): bigint {
    return this.account(address)?.lamports ?? 0n;
  }

  data(address: TAddress): Uint8Array | null {
    return this.account(address)?.data ?? null;
  }

  tokenAccount(address: TAddress): Token<TAddress> | null {
    const acct = this.account(address);
    if (!acct) return null;
    const raw = unpackTokenAccount(acct.data);
    if (!raw) return null;
    return {
      mint: this.decodeFn(raw.mint),
      owner: this.decodeFn(raw.owner),
      amount: raw.amount,
      delegate: raw.delegate ? this.decodeFn(raw.delegate) : null,
      state: raw.state ?? TokenAccountState.Initialized,
      isNative: raw.isNative ?? null,
      delegatedAmount: raw.delegatedAmount ?? 0n,
      closeAuthority: raw.closeAuthority ? this.decodeFn(raw.closeAuthority) : null,
    };
  }

  mintAccount(address: TAddress): Mint<TAddress> | null {
    const acct = this.account(address);
    if (!acct) return null;
    const raw = unpackMint(acct.data);
    if (!raw) return null;
    return {
      mintAuthority: raw.mintAuthority ? this.decodeFn(raw.mintAuthority) : null,
      supply: raw.supply ?? 0n,
      decimals: raw.decimals ?? 9,
      freezeAuthority: raw.freezeAuthority ? this.decodeFn(raw.freezeAuthority) : null,
    };
  }

  tokenBalance(address: TAddress): bigint | null {
    return this.tokenAccount(address)?.amount ?? null;
  }

  mintSupply(address: TAddress): bigint | null {
    return this.mintAccount(address)?.supply ?? null;
  }
}
