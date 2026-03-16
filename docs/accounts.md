# Accounts

QuasarSVM uses `SvmAccount` as the universal account type across all layers. Account factory functions create `SvmAccount` values that are passed to execution.

## SvmAccount

The unified account type used everywhere — Rust, web3.js, and kit.

**Rust:**

```rust
pub struct SvmAccount {
    pub address: Pubkey,
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
    pub executable: bool,
}
```

**TypeScript (web3.js):**

```ts
interface SvmAccount {
  address: PublicKey;
  lamports: bigint;
  data: Buffer;
  owner: PublicKey;
  executable: boolean;
}
```

**TypeScript (kit):**

```ts
interface SvmAccount {
  address: Address;
  lamports: bigint;
  data: Uint8Array;
  owner: Address;
  executable: boolean;
}
```

## KeyedAccountInfo Interop (web3.js only)

The web3.js layer provides converters for interop with legacy code that uses `KeyedAccountInfo`:

```ts
import { toKeyedAccountInfo, fromKeyedAccountInfo } from "@blueshift-gg/quasar-svm/web3.js";

// SvmAccount -> KeyedAccountInfo
const keyed = toKeyedAccountInfo(svmAccount);

// KeyedAccountInfo -> SvmAccount
const account = fromKeyedAccountInfo(keyed);
```

## Account Factories

All factories return `SvmAccount`. The address parameter is **optional** — when omitted, an address is auto-generated.

### System Account

Create a system-owned account with a SOL balance:

**Rust:**

```rust
use quasar_svm::token::create_system_account;

// Auto-generated address
let account = create_system_account(1_000_000_000);

// Explicit address
let account = create_system_account_at(&pubkey, 1_000_000_000);
```

**TypeScript (web3.js):**

```ts
import { createSystemAccount } from "@blueshift-gg/quasar-svm/web3.js";

// Auto-generated address
const account = createSystemAccount(1_000_000_000n);

// Explicit address
const account = createSystemAccount(pubkey, 1_000_000_000n);
```

**TypeScript (kit):**

```ts
import { createSystemAccount } from "@blueshift-gg/quasar-svm/kit";

const account = createSystemAccount(1_000_000_000n);
const account = createSystemAccount(address, 1_000_000_000n);
```

### Mint Account

Create a pre-initialized SPL Token mint:

**Rust:**

```rust
use quasar_svm::token::{create_mint_account, create_mint_account_at, Mint};

// Auto-generated address
let account = create_mint_account(
    &Mint { decimals: 6, supply: 10_000, ..Default::default() },
    &SPL_TOKEN_PROGRAM_ID,
);

// Explicit address
let account = create_mint_account_at(
    &pubkey,
    &Mint { decimals: 6, ..Default::default() },
    &SPL_TOKEN_PROGRAM_ID,
);

// Token-2022
let account = create_mint_account(
    &Mint { decimals: 6, ..Default::default() },
    &SPL_TOKEN_2022_PROGRAM_ID,
);
```

**TypeScript (web3.js):**

```ts
import { createMintAccount } from "@blueshift-gg/quasar-svm/web3.js";

// Auto-generated address
const account = createMintAccount({ decimals: 6 });
const account = createMintAccount({ decimals: 6, supply: 10_000n });

// Explicit address
const account = createMintAccount(pubkey, { decimals: 6 });

// Token-2022
const account = createMintAccount({ decimals: 6 }, TOKEN_2022_PROGRAM_ID);
const account = createMintAccount(pubkey, { decimals: 6 }, TOKEN_2022_PROGRAM_ID);
```

**TypeScript (kit):**

```ts
import { createMintAccount } from "@blueshift-gg/quasar-svm/kit";

const account = createMintAccount({ decimals: 6 });
const account = createMintAccount(address, { decimals: 6 });
const account = createMintAccount({ decimals: 6 }, TOKEN_2022_PROGRAM_ID);
```

### Token Account

Create a pre-initialized token account:

**Rust:**

```rust
use quasar_svm::token::{create_token_account, create_token_account_at, Token};

// Auto-generated address
let account = create_token_account(
    &Token { mint, owner, amount: 5_000, ..Default::default() },
    &SPL_TOKEN_PROGRAM_ID,
);

// Explicit address
let account = create_token_account_at(
    &pubkey,
    &Token { mint, owner, amount: 5_000, ..Default::default() },
    &SPL_TOKEN_PROGRAM_ID,
);
```

**TypeScript (web3.js):**

```ts
import { createTokenAccount } from "@blueshift-gg/quasar-svm/web3.js";

// Auto-generated address
const account = createTokenAccount({ mint, owner, amount: 5_000n });

// Explicit address
const account = createTokenAccount(pubkey, { mint, owner, amount: 5_000n });

// Token-2022
const account = createTokenAccount({ mint, owner, amount: 5_000n }, TOKEN_2022_PROGRAM_ID);
```

**TypeScript (kit):**

```ts
import { createTokenAccount } from "@blueshift-gg/quasar-svm/kit";

const account = createTokenAccount({ mint, owner, amount: 5_000n });
const account = createTokenAccount(address, { mint, owner, amount: 5_000n });
```

### Associated Token Account

Derive the ATA address automatically and create a pre-initialized token account. The address is always derived (not optional).

**Rust:**

```rust
use quasar_svm::token::create_associated_token_account;

let account = create_associated_token_account(&wallet, &mint, 5_000, &SPL_TOKEN_PROGRAM_ID);
// account.address is the derived ATA address

// Token-2022
let account = create_associated_token_account(&wallet, &mint, 5_000, &SPL_TOKEN_2022_PROGRAM_ID);
```

**TypeScript (web3.js) — sync:**

```ts
import { createAssociatedTokenAccount } from "@blueshift-gg/quasar-svm/web3.js";

const account = createAssociatedTokenAccount(owner, mint, 5_000n);
account.address; // derived ATA address

// Token-2022
const account = createAssociatedTokenAccount(owner, mint, 5_000n, TOKEN_2022_PROGRAM_ID);
```

**TypeScript (kit) — async:**

```ts
import { createAssociatedTokenAccount } from "@blueshift-gg/quasar-svm/kit";

// Async because PDA derivation is async in @solana/addresses
const account = await createAssociatedTokenAccount(owner, mint, 5_000n);
account.address; // derived ATA address

const account = await createAssociatedTokenAccount(owner, mint, 5_000n, TOKEN_2022_PROGRAM_ID);
```

> **Note:** All factories are synchronous except kit's `createAssociatedTokenAccount`, which is async due to PDA derivation in `@solana/addresses`.

## Token-2022 Support

All factories that create token-related accounts accept an optional `tokenProgramId` / `programId` parameter. Pass `SPL_TOKEN_2022_PROGRAM_ID` to create Token-2022 accounts:

```rust
let mint = create_mint_account(opts, &SPL_TOKEN_2022_PROGRAM_ID);
let token = create_token_account(opts, &SPL_TOKEN_2022_PROGRAM_ID);
let ata = create_associated_token_account(&wallet, &mint_addr, amount, &SPL_TOKEN_2022_PROGRAM_ID);
```

```ts
const mint  = createMintAccount({ decimals: 6 }, TOKEN_2022_PROGRAM_ID);
const token = createTokenAccount({ mint, owner, amount: 5_000n }, TOKEN_2022_PROGRAM_ID);
const ata   = createAssociatedTokenAccount(owner, mint, 5_000n, TOKEN_2022_PROGRAM_ID);
```

## Mint / MintOpts

**Rust:**

```rust
pub struct Mint {
    pub mint_authority: Option<Pubkey>,
    pub supply: u64,
    pub decimals: u8,
    pub freeze_authority: Option<Pubkey>,
}
```

**TypeScript (web3.js):**

```ts
interface MintOpts {
  mintAuthority?: PublicKey;
  supply?: bigint;
  decimals?: number;         // default: 9
  freezeAuthority?: PublicKey;
}
```

**TypeScript (kit):**

```ts
interface MintOpts {
  mintAuthority?: Address;
  supply?: bigint;
  decimals?: number;
  freezeAuthority?: Address;
}
```

## Token / TokenAccountOpts

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
```

**TypeScript (web3.js):**

```ts
interface TokenAccountOpts {
  mint: PublicKey;
  owner: PublicKey;
  amount: bigint;
  delegate?: PublicKey;
  state?: TokenAccountState;       // default: Initialized
  isNative?: bigint;
  delegatedAmount?: bigint;
  closeAuthority?: PublicKey;
}
```

**TypeScript (kit):**

```ts
interface TokenAccountOpts {
  mint: Address;
  owner: Address;
  amount: bigint;
  delegate?: Address;
  state?: TokenAccountState;
  isNative?: bigint;
  delegatedAmount?: bigint;
  closeAuthority?: Address;
}
```
