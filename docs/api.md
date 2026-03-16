# Core API

QuasarSVM provides an in-process Solana execution engine. Create a `QuasarSvm` instance, load programs, execute transactions, and inspect results with byte-level account diffs.

## QuasarSvm

### Creating a VM

```rust
let svm = QuasarSvm::new();
```

```ts
const vm = new QuasarSvm();
```

> Native memory is freed automatically by the GC. For deterministic cleanup, use `using vm = new QuasarSvm()` or call `vm.free()`.

### Loading Programs

Load a custom program from an ELF binary:

```rust
let elf = std::fs::read("target/deploy/my_program.so").unwrap();
svm.add_program(&program_id, &loader_keys::LOADER_V3, &elf);

// Builder-style
let svm = QuasarSvm::new()
    .with_program(&program_id, &elf)
    .with_program_loader(&program_id, &loader_keys::LOADER_V2, &elf);
```

```ts
// web3.js — programId is PublicKey
vm.addProgram(programId, elf);             // loader v3 (default)
vm.addProgram(programId, elf, LOADER_V2);  // loader v2

// kit — programId is Address
vm.addProgram(programId, elf);
vm.addProgram(programId, elf, LOADER_V2);
```

Load bundled SPL programs:

```rust
let svm = QuasarSvm::new()
    .with_token_program()
    .with_token_2022_program()
    .with_associated_token_program();
```

```ts
const vm = new QuasarSvm()
  .addTokenProgram()
  .addToken2022Program()
  .addAssociatedTokenProgram();
```

### Executing Instructions

Four execution methods — single or chain, process or simulate:

| Method | Behavior |
|--------|----------|
| `process_instruction` / `processInstruction` | Execute one instruction atomically. Commits state on success. |
| `process_instruction_chain` / `processInstructionChain` | Execute multiple instructions as one atomic chain. State rolls back on failure. |
| `simulate_instruction` / `simulateInstruction` | Simulate one instruction without committing state changes. |
| `simulate_instruction_chain` / `simulateInstructionChain` | Simulate multiple instructions without committing state changes. |

**Rust:**

```rust
// Single instruction
let result = svm.process_instruction(&ix, &accounts);

// Multiple instructions — atomic
let result = svm.process_instruction_chain(&[ix1, ix2], &accounts);

// Read-only simulation
let result = svm.simulate_instruction(&ix, &accounts);
let result = svm.simulate_instruction_chain(&[ix1, ix2], &accounts);
```

Accounts are `&[SvmAccount]` — a slice of `SvmAccount` structs.

**TypeScript (web3.js):**

```ts
// Single instruction
const result = vm.processInstruction(ix, accounts);

// Multiple instructions — atomic
const result = vm.processInstructionChain([ix1, ix2], accounts);

// Read-only simulation
const result = vm.simulateInstruction(ix, accounts);
const result = vm.simulateInstructionChain([ix1, ix2], accounts);
```

**TypeScript (kit):**

```ts
const result = vm.processInstruction(ix, accounts);
const result = vm.processInstructionChain([ix1, ix2], accounts);
const result = vm.simulateInstruction(ix, accounts);
```

Accounts are always `SvmAccount[]`:

```ts
vm.processInstruction(ix, [acct1, acct2, acct3]);
```

### Account Store

The SVM maintains a persistent account database. Accounts passed to execution are merged with the store automatically.

```rust
svm.set_account(svm_account);
let acct = svm.get_account(&pubkey);
svm.airdrop(&pubkey, 1_000_000_000);
svm.create_account(&pubkey, space, &owner);
```

```ts
// web3.js
vm.setAccount(svmAccount);
const acct: SvmAccount | null = vm.getAccount(pubkey);
vm.airdrop(pubkey, 1_000_000_000n);
vm.createAccount(pubkey, 0n, owner);

// kit
vm.setAccount(svmAccount);
const acct: SvmAccount | null = vm.getAccount(address);
vm.airdrop(address, 1_000_000_000n);
vm.createAccount(address, 0n, owner);
```

Builder-style (Rust):

```rust
let svm = QuasarSvm::new()
    .with_account(svm_account)
    .with_airdrop(&pubkey, 1_000_000_000)
    .with_create_account(&pubkey, 0, &owner);
```

### Cheatcodes

Cheatcodes modify VM state directly for test setup.

#### Token Cheatcodes

Modify existing token/mint accounts in the store:

```rust
svm.set_token_balance(&token_account_pubkey, 5_000);
svm.set_mint_supply(&mint_pubkey, 100_000);
```

```ts
// web3.js
vm.setTokenBalance(tokenAccountPubkey, 5_000n);
vm.setMintSupply(mintPubkey, 100_000n);

// kit
vm.setTokenBalance(tokenAccountAddress, 5_000n);
vm.setMintSupply(mintAddress, 100_000n);
```

#### Time & Slot

```rust
svm.warp_to_timestamp(1_700_000_000);  // sets clock.unix_timestamp only
svm.sysvars.warp_to_slot(200);
```

```ts
vm.warpToTimestamp(1_700_000_000n);  // sets clock.unix_timestamp only
vm.warpToSlot(200n);
```

#### Clock, Rent, Epoch, Compute Budget

```rust
svm.set_clock(clock);
svm.set_rent(rent);
svm.set_epoch_schedule(epoch_schedule);
svm.set_compute_budget(200_000);
```

```ts
vm.setClock({ slot: 100n, epochStartTimestamp: 0n, epoch: 0n, leaderScheduleEpoch: 0n, unixTimestamp: 0n });
vm.setRent(3480n);
vm.setEpochSchedule({ slotsPerEpoch: 432000n, leaderScheduleSlotOffset: 0n, warmup: false, firstNormalEpoch: 0n, firstNormalSlot: 0n });
vm.setComputeBudget(200_000n);
```

### Cleanup

Native memory is freed automatically by the garbage collector — no manual cleanup required in most cases.

For deterministic cleanup (tight loops, benchmarks), use `using` or call `free()`:

```ts
// Automatic — GC handles it
const vm = new QuasarSvm().addTokenProgram();

// Deterministic — freed when scope exits
{
  using vm = new QuasarSvm().addTokenProgram();
} // freed here

// Manual — explicit control
const vm = new QuasarSvm();
vm.free();
```

## ExecutionResult

Every execution returns an `ExecutionResult`. In TypeScript it is a class with methods; in Rust it is a struct with methods.

### Fields

| Field | Rust | TypeScript |
|-------|------|------------|
| Status | `raw_result: Result<(), InstructionError>` | `status: ExecutionStatus` |
| Compute units | `compute_units_consumed: u64` | `computeUnits: bigint` |
| Execution time | `execution_time_us: u64` | `executionTimeUs: bigint` |
| Return data | `return_data: Vec<u8>` | `returnData: Uint8Array` |
| Resulting accounts | `resulting_accounts: Vec<SvmAccount>` | `accounts: SvmAccount[]` |
| Modified accounts | `modified_accounts: Vec<AccountDiff>` | `modifiedAccounts: AccountDiff[]` |
| Logs | `logs: Vec<String>` | `logs: string[]` |

### AccountDiff

Byte-level diff of accounts that changed during execution:

```rust
pub struct AccountDiff {
    pub pre: SvmAccount,
    pub post: SvmAccount,
}
```

```ts
interface AccountDiff {
  pre: SvmAccount;
  post: SvmAccount;
}
```

### Assertion Methods

```rust
result.assert_success();
result.assert_error(ProgramError::InsufficientFunds);
assert!(result.is_success());
assert!(result.is_error());
result.print_logs();
```

```ts
result.assertSuccess();
result.assertError({ type: "InsufficientFunds" });
result.assertError({ type: "Custom", code: 6001 });

result.isSuccess();  // boolean
result.isError();    // boolean
result.printLogs();
```

### Account Lookup Methods

```rust
let acct: Option<&SvmAccount> = result.account(&pubkey);
let data: Option<&[u8]> = result.data(&pubkey);
let lamps: Option<u64> = result.lamports(&pubkey);
```

```ts
const acct: SvmAccount | null = result.account(address);
const data: Uint8Array | null = result.data(address);  // Buffer in web3.js
const lamps: bigint | null = result.lamports(address);
```

### Token Helpers

Unpack token and mint state from resulting accounts:

```rust
let token = result.token_account(&ata_pubkey).unwrap();
assert_eq!(token.amount, 1_000);

let mint = result.mint_account(&mint_pubkey).unwrap();
assert_eq!(mint.supply, 15_000);

let balance: Option<u64> = result.token_balance(&ata_pubkey);
let supply: Option<u64> = result.mint_supply(&mint_pubkey);
```

```ts
const token = result.tokenAccount(ataPubkey);    // Token | null
const mint  = result.mintAccount(mintPubkey);     // Mint | null
const balance = result.tokenBalance(ataPubkey);   // bigint | null
const supply  = result.mintSupply(mintPubkey);    // bigint | null
```

### Borsh Deserialization (Rust)

```rust
// Requires "borsh" feature
let state: MyState = result.account_data(&pubkey).unwrap();
```

## ProgramError

TypeScript uses a discriminated union. Known errors map to negative codes; `Custom(n)` maps to positive codes.

```ts
type ProgramError =
  | { type: "InvalidArgument" }
  | { type: "InvalidInstructionData" }
  | { type: "InvalidAccountData" }
  | { type: "AccountDataTooSmall" }
  | { type: "InsufficientFunds" }
  | { type: "IncorrectProgramId" }
  | { type: "MissingRequiredSignature" }
  | { type: "AccountAlreadyInitialized" }
  | { type: "UninitializedAccount" }
  | { type: "MissingAccount" }
  | { type: "InvalidSeeds" }
  | { type: "ArithmeticOverflow" }
  | { type: "AccountNotRentExempt" }
  | { type: "InvalidAccountOwner" }
  | { type: "IncorrectAuthority" }
  | { type: "Immutable" }
  | { type: "BorshIoError" }
  | { type: "ComputeBudgetExceeded" }
  | { type: "Custom"; code: number }
  | { type: "Runtime"; message: string }
```
