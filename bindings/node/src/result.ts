import type { AccountDiff, ExecutionStatus, ProgramError } from "./index.js";
import { unpackMint, unpackTokenAccount } from "./token.js";
import type { MintData, TokenAccountData } from "./token.js";

export type AccountFinder<TAccount, TAddress> = (
  accounts: TAccount[],
  address: TAddress,
) => TAccount | undefined;

export class ExecutionResult<
  TAccount extends { data: Uint8Array; lamports: bigint },
  TAddress = unknown,
> {
  readonly status: ExecutionStatus;
  readonly computeUnits: bigint;
  readonly executionTimeUs: bigint;
  readonly returnData: Uint8Array;
  readonly accounts: TAccount[];
  readonly modifiedAccounts: AccountDiff<TAccount>[];
  readonly logs: string[];
  private readonly findFn: AccountFinder<TAccount, TAddress>;

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
  ) {
    this.status = fields.status;
    this.computeUnits = fields.computeUnits;
    this.executionTimeUs = fields.executionTimeUs;
    this.returnData = fields.returnData;
    this.accounts = fields.accounts;
    this.modifiedAccounts = fields.modifiedAccounts;
    this.logs = fields.logs;
    this.findFn = findAccount;
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

  tokenAccount(address: TAddress): TokenAccountData | null {
    const acct = this.account(address);
    if (!acct) return null;
    return unpackTokenAccount(acct.data);
  }

  mintAccount(address: TAddress): MintData | null {
    const acct = this.account(address);
    if (!acct) return null;
    return unpackMint(acct.data);
  }

  tokenBalance(address: TAddress): bigint | null {
    return this.tokenAccount(address)?.amount ?? null;
  }

  mintSupply(address: TAddress): bigint | null {
    return this.mintAccount(address)?.supply ?? null;
  }
}
