# Tokens

QuasarSVM includes built-in SPL Token types, instruction builders, result helpers, and VM cheatcodes. Everything works with both SPL Token and Token-2022.

## Types

### Mint

The `Mint` struct/interface represents SPL Token mint state (returned from result helpers).

**Rust:**

```rust
pub struct Mint {
    pub mint_authority: Option<Pubkey>,
    pub supply: u64,
    pub decimals: u8,
    pub freeze_authority: Option<Pubkey>,
}

let mint = Mint::default(); // decimals = 9, supply = 0, no authorities
let mint = Mint { decimals: 6, supply: 10_000, ..Default::default() };
```

**TypeScript (web3.js):**

```ts
interface Mint {
  mintAuthority: PublicKey | null;
  supply: bigint;
  decimals: number;
  freezeAuthority: PublicKey | null;
}
```

**TypeScript (kit):**

```ts
interface Mint {
  mintAuthority: Address | null;
  supply: bigint;
  decimals: number;
  freezeAuthority: Address | null;
}
```

### Token

The `Token` struct/interface represents SPL Token account state (returned from result helpers).

**Rust:**

```rust
pub struct Token {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub delegate: Option<Pubkey>,
    pub state: TokenAccountState,
    pub is_native: Option<u64>,
    pub delegated_amount: u64,
    pub close_authority: Option<Pubkey>,
}

let token = Token { mint, owner, amount: 5_000, ..Default::default() };
```

**TypeScript (web3.js):**

```ts
interface Token {
  mint: PublicKey;
  owner: PublicKey;
  amount: bigint;
  delegate: PublicKey | null;
  state: TokenAccountState;
  isNative: bigint | null;
  delegatedAmount: bigint;
  closeAuthority: PublicKey | null;
}
```

**TypeScript (kit):**

```ts
interface Token {
  mint: Address;
  owner: Address;
  amount: bigint;
  delegate: Address | null;
  state: TokenAccountState;
  isNative: bigint | null;
  delegatedAmount: bigint;
  closeAuthority: Address | null;
}
```

### TokenAccountState

```rust
pub enum TokenAccountState {
    Uninitialized = 0,
    Initialized  = 1, // default
    Frozen       = 2,
}
```

```ts
enum TokenAccountState {
  Uninitialized = 0,
  Initialized   = 1, // default
  Frozen        = 2,
}
```

## Instruction Builders

All builders accept an optional `tokenProgramId` parameter (defaults to SPL Token). Pass `SPL_TOKEN_2022_PROGRAM_ID` for Token-2022.

### Transfer

```rust
use quasar_svm::token::token_transfer;

let ix = token_transfer(&source, &destination, &authority, 1_000, &SPL_TOKEN_PROGRAM_ID);

// Token-2022
let ix = token_transfer(&source, &destination, &authority, 1_000, &SPL_TOKEN_2022_PROGRAM_ID);
```

```ts
// web3.js — args are PublicKey, returns TransactionInstruction
import { tokenTransfer } from "@blueshift-gg/quasar-svm/web3.js";

const ix = tokenTransfer(source, destination, authority, 1_000n);
const ix = tokenTransfer(source, destination, authority, 1_000n, TOKEN_2022_PROGRAM_ID);
```

```ts
// kit — args are Address, returns Instruction
import { tokenTransfer } from "@blueshift-gg/quasar-svm/kit";

const ix = tokenTransfer(source, destination, authority, 1_000n);
const ix = tokenTransfer(source, destination, authority, 1_000n, TOKEN_2022_PROGRAM_ID);
```

### MintTo

```rust
use quasar_svm::token::token_mint_to;

let ix = token_mint_to(&mint, &destination, &mint_authority, 5_000, &SPL_TOKEN_PROGRAM_ID);
```

```ts
// web3.js
import { tokenMintTo } from "@blueshift-gg/quasar-svm/web3.js";

const ix = tokenMintTo(mint, destination, mintAuthority, 5_000n);
```

```ts
// kit
import { tokenMintTo } from "@blueshift-gg/quasar-svm/kit";

const ix = tokenMintTo(mint, destination, mintAuthority, 5_000n);
```

### Burn

```rust
use quasar_svm::token::token_burn;

let ix = token_burn(&source, &mint, &authority, 500, &SPL_TOKEN_PROGRAM_ID);
```

```ts
// web3.js
import { tokenBurn } from "@blueshift-gg/quasar-svm/web3.js";

const ix = tokenBurn(source, mint, authority, 500n);
```

```ts
// kit
import { tokenBurn } from "@blueshift-gg/quasar-svm/kit";

const ix = tokenBurn(source, mint, authority, 500n);
```

## Result Token Helpers

Methods on `ExecutionResult` for unpacking token and mint state from resulting accounts.

### tokenAccount / token_account

```rust
let token: Option<Token> = result.token_account(&ata_pubkey);
assert_eq!(token.unwrap().amount, 1_000);
```

```ts
const token: Token | null = result.tokenAccount(ataPubkey);
console.log(token?.amount); // 1000n
```

### mintAccount / mint_account

```rust
let mint: Option<Mint> = result.mint_account(&mint_pubkey);
assert_eq!(mint.unwrap().supply, 15_000);
```

```ts
const mint: Mint | null = result.mintAccount(mintPubkey);
console.log(mint?.supply); // 15000n
```

### tokenBalance / token_balance

Shorthand to get just the amount:

```rust
let balance: Option<u64> = result.token_balance(&ata_pubkey);
assert_eq!(balance, Some(1_000));
```

```ts
const balance: bigint | null = result.tokenBalance(ataPubkey);
console.log(balance); // 1000n
```

### mintSupply / mint_supply

Shorthand to get just the supply:

```rust
let supply: Option<u64> = result.mint_supply(&mint_pubkey);
assert_eq!(supply, Some(15_000));
```

```ts
const supply: bigint | null = result.mintSupply(mintPubkey);
console.log(supply); // 15000n
```

## VM Cheatcodes

Directly modify token and mint state in the VM's account store without executing instructions.

### setTokenBalance / set_token_balance

Modify the balance of an existing token account in the store:

```rust
svm.set_token_balance(&token_account_pubkey, 10_000);
```

```ts
// web3.js
vm.setTokenBalance(tokenAccountPubkey, 10_000n);

// kit
vm.setTokenBalance(tokenAccountAddress, 10_000n);
```

### setMintSupply / set_mint_supply

Modify the supply of an existing mint account in the store:

```rust
svm.set_mint_supply(&mint_pubkey, 1_000_000);
```

```ts
// web3.js
vm.setMintSupply(mintPubkey, 1_000_000n);

// kit
vm.setMintSupply(mintAddress, 1_000_000n);
```

## ATA Derivation

Derive associated token account addresses without creating accounts.

```rust
use quasar_svm::token::get_associated_token_address;

let ata = get_associated_token_address(&wallet, &mint, &SPL_TOKEN_PROGRAM_ID);

// Token-2022
let ata = get_associated_token_address(&wallet, &mint, &SPL_TOKEN_2022_PROGRAM_ID);
```

```ts
// web3.js (sync)
import { PublicKey } from "@solana/web3.js";

const [ata] = PublicKey.findProgramAddressSync(
  [wallet.toBuffer(), tokenProgramId.toBuffer(), mint.toBuffer()],
  new PublicKey(SPL_ASSOCIATED_TOKEN_PROGRAM_ID),
);
```

```ts
// kit (async)
import { getProgramDerivedAddress } from "@solana/addresses";

const [ata] = await getProgramDerivedAddress({
  programAddress: SPL_ASSOCIATED_TOKEN_PROGRAM_ID,
  seeds: [encode(wallet), encode(tokenProgramId), encode(mint)],
});
```

Or use `createAssociatedTokenAccount` which derives the address and creates the account in one step (see [Accounts](accounts.md)).

## Full Example

### Rust

```rust
use quasar_svm::{QuasarSvm, Pubkey, SPL_TOKEN_PROGRAM_ID};
use quasar_svm::token::*;

let authority = Pubkey::new_unique();
let recipient = Pubkey::new_unique();

let mint  = create_mint_account(
    &Mint { decimals: 6, supply: 10_000, ..Default::default() },
    &SPL_TOKEN_PROGRAM_ID,
);
let alice = create_associated_token_account(&authority, &mint.address, 5_000, &SPL_TOKEN_PROGRAM_ID);
let bob   = create_associated_token_account(&recipient, &mint.address, 0, &SPL_TOKEN_PROGRAM_ID);

let ix = token_transfer(&alice.address, &bob.address, &authority, 1_000, &SPL_TOKEN_PROGRAM_ID);

let mut svm = QuasarSvm::new().with_token_program();

let result = svm.process_instruction(&ix, &[mint, alice, bob]);

result.assert_success();
assert_eq!(result.token_balance(&bob.address), Some(1_000));
assert_eq!(result.token_balance(&alice.address), Some(4_000));

// Inspect byte-level diffs
for diff in &result.modified_accounts {
    println!("{}: {} -> {} lamports", diff.pre.address, diff.pre.lamports, diff.post.lamports);
}
```

### TypeScript (web3.js)

```ts
import {
  QuasarSvm,
  createMintAccount, createAssociatedTokenAccount,
  tokenTransfer,
} from "@blueshift-gg/quasar-svm/web3.js";
import { Keypair } from "@solana/web3.js";

const vm = new QuasarSvm().addTokenProgram();

const authority = Keypair.generate().publicKey;
const recipient = Keypair.generate().publicKey;

const mint  = createMintAccount({ decimals: 6, supply: 10_000n });
const alice = createAssociatedTokenAccount(authority, mint.address, 5_000n);
const bob   = createAssociatedTokenAccount(recipient, mint.address, 0n);

const ix = tokenTransfer(alice.address, bob.address, authority, 1_000n);

const result = vm.processInstruction(ix, [mint, alice, bob]);

result.assertSuccess();
console.log(result.tokenBalance(bob.address));   // 1000n
console.log(result.tokenBalance(alice.address)); // 4000n

// Inspect byte-level diffs
for (const diff of result.modifiedAccounts) {
  console.log(`${diff.pre.address}: ${diff.pre.lamports} -> ${diff.post.lamports}`);
}

```

### TypeScript (kit)

```ts
import {
  QuasarSvm,
  createMintAccount, createAssociatedTokenAccount,
  tokenTransfer,
} from "@blueshift-gg/quasar-svm/kit";
import { generateKeyPair, getAddressFromPublicKey } from "@solana/keys";

const vm = new QuasarSvm().addTokenProgram();

const authorityKp = await generateKeyPair();
const authority = await getAddressFromPublicKey(authorityKp.publicKey);
const recipient = await getAddressFromPublicKey((await generateKeyPair()).publicKey);

const mint  = createMintAccount({ decimals: 6, supply: 10_000n });
const alice = await createAssociatedTokenAccount(authority, mint.address, 5_000n);
const bob   = await createAssociatedTokenAccount(recipient, mint.address, 0n);

const ix = tokenTransfer(alice.address, bob.address, authority, 1_000n);

const result = vm.processInstruction(ix, [mint, alice, bob]);

result.assertSuccess();
console.log(result.tokenBalance(bob.address));   // 1000n
console.log(result.tokenBalance(alice.address)); // 4000n

// Inspect byte-level diffs
for (const diff of result.modifiedAccounts) {
  console.log(`${diff.pre.address}: ${diff.pre.lamports} -> ${diff.post.lamports}`);
}

```
