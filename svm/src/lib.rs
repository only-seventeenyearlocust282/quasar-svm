mod error;
mod program_cache;
mod svm;
mod sysvars;

pub use solana_account::Account;
pub use solana_clock::Clock;
pub use solana_instruction::{AccountMeta, Instruction};
pub use solana_instruction_error::InstructionError;
pub use solana_pubkey::Pubkey;
pub use solana_rent::Rent;
pub use solana_sdk_ids;

/// Convenience alias so users can write `quasar_svm::system_program::ID`.
pub use solana_sdk_ids::system_program;

pub use crate::error::ProgramError;
pub use crate::program_cache::loader_keys;
pub use crate::svm::{ExecutionResult, QuasarSvm};
pub use crate::sysvars::Sysvars;

// ---------------------------------------------------------------------------
// Bundled SPL programs
// ---------------------------------------------------------------------------

pub const SPL_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

pub const SPL_TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const SPL_ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

// ---------------------------------------------------------------------------
// Builder-style helpers on QuasarSvm
// ---------------------------------------------------------------------------

impl QuasarSvm {
    /// Load a BPF program from an ELF byte slice (loader v3 / upgradeable).
    pub fn with_program(self, program_id: &Pubkey, elf: &[u8]) -> Self {
        self.add_program(program_id, &loader_keys::LOADER_V3, elf);
        self
    }

    /// Load a BPF program with a specific loader version.
    pub fn with_program_loader(self, program_id: &Pubkey, loader: &Pubkey, elf: &[u8]) -> Self {
        self.add_program(program_id, loader, elf);
        self
    }

    /// No-op — system program is already built in. Exists for parity with
    /// the TypeScript API.
    pub fn with_system_program(self) -> Self {
        self
    }

    /// Load the bundled SPL Token program.
    pub fn with_token_program(self) -> Self {
        let elf = include_bytes!("../../programs/spl_token.so");
        self.with_program_loader(&SPL_TOKEN_PROGRAM_ID, &loader_keys::LOADER_V2, elf)
    }

    /// Load the bundled SPL Token 2022 program.
    pub fn with_token_2022_program(self) -> Self {
        let elf = include_bytes!("../../programs/spl_token_2022.so");
        self.with_program(&SPL_TOKEN_2022_PROGRAM_ID, elf)
    }

    /// Load the bundled SPL Associated Token program.
    pub fn with_associated_token_program(self) -> Self {
        let elf = include_bytes!("../../programs/spl_associated_token.so");
        self.with_program_loader(
            &SPL_ASSOCIATED_TOKEN_PROGRAM_ID,
            &loader_keys::LOADER_V2,
            elf,
        )
    }

    /// Pre-populate an account in the SVM's account database.
    pub fn with_account(mut self, pubkey: Pubkey, account: Account) -> Self {
        self.set_account(pubkey, account);
        self
    }

    /// Set the clock slot (convenience for `sysvars.warp_to_slot`).
    pub fn with_slot(mut self, slot: u64) -> Self {
        self.sysvars.warp_to_slot(slot);
        self
    }
}

// ---------------------------------------------------------------------------
// ExecutionResult
// ---------------------------------------------------------------------------

impl ExecutionResult {
    /// `0` on success, or the error code from the failed instruction.
    pub fn status(&self) -> i32 {
        match &self.raw_result {
            Ok(()) => 0,
            Err(e) => instruction_error_to_code(e),
        }
    }

    pub fn is_ok(&self) -> bool {
        self.raw_result.is_ok()
    }

    pub fn is_err(&self) -> bool {
        self.raw_result.is_err()
    }

    /// Convert the raw result into a typed `ProgramError`.
    pub fn error(&self) -> Option<ProgramError> {
        self.raw_result
            .as_ref()
            .err()
            .map(|e| ProgramError::from(e.clone()))
    }

    /// Panics with the error and program logs if execution failed.
    pub fn unwrap(&self) {
        if let Err(ref e) = self.raw_result {
            panic!("{}", self.format_error(e));
        }
    }

    /// Panics with a custom message, error, and program logs.
    pub fn expect(&self, msg: &str) {
        if let Err(ref e) = self.raw_result {
            panic!("{msg}: {}", self.format_error(e));
        }
    }

    /// Look up a resulting account by pubkey.
    pub fn account(&self, pubkey: &Pubkey) -> Option<&Account> {
        self.resulting_accounts
            .iter()
            .find(|(k, _)| k == pubkey)
            .map(|(_, a)| a)
    }

    fn format_error(&self, e: &InstructionError) -> String {
        let err = ProgramError::from(e.clone());
        if self.logs.is_empty() {
            format!("{err}")
        } else {
            format!(
                "{err}\n\nProgram logs:\n{}",
                self.logs
                    .iter()
                    .map(|l| format!("  {l}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    }
}

/// Map an `InstructionError` to a numeric status code.
/// `Custom(n)` → `n as i32`, known variants → negative codes, unknown → `-1`.
fn instruction_error_to_code(err: &InstructionError) -> i32 {
    match err {
        InstructionError::Custom(n) => *n as i32,
        InstructionError::InvalidArgument => -2,
        InstructionError::InvalidInstructionData => -3,
        InstructionError::InvalidAccountData => -4,
        InstructionError::AccountDataTooSmall => -5,
        InstructionError::InsufficientFunds => -6,
        InstructionError::IncorrectProgramId => -7,
        InstructionError::MissingRequiredSignature => -8,
        InstructionError::AccountAlreadyInitialized => -9,
        InstructionError::UninitializedAccount => -10,
        InstructionError::MissingAccount => -11,
        InstructionError::ComputationalBudgetExceeded => -12,
        InstructionError::ArithmeticOverflow => -13,
        _ => -1,
    }
}
