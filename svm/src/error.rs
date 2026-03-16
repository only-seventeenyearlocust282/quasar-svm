use core::fmt;
use solana_instruction_error::InstructionError;

/// Errors returned by program execution in QuasarSVM.
///
/// Maps from the runtime's `InstructionError` into a clean enum for
/// assertions in tests:
///
/// ```ignore
/// let result = svm.process_instruction(&ix, &accounts);
/// assert_eq!(result.error(), Some(ProgramError::InvalidAccountData));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramError {
    // -- Common program errors (what you actually hit in tests) --
    /// The arguments provided were invalid.
    InvalidArgument,
    /// The instruction data was invalid.
    InvalidInstructionData,
    /// The account data was invalid.
    InvalidAccountData,
    /// Account data was too small.
    AccountDataTooSmall,
    /// Insufficient lamports.
    InsufficientFunds,
    /// Wrong program id.
    IncorrectProgramId,
    /// A required signature was missing.
    MissingRequiredSignature,
    /// Account was already initialized.
    AccountAlreadyInitialized,
    /// Account was not initialized.
    UninitializedAccount,
    /// An account required by the instruction is missing.
    MissingAccount,
    /// Invalid PDA seeds.
    InvalidSeeds,
    /// Arithmetic overflow.
    ArithmeticOverflow,
    /// Account is not rent-exempt.
    AccountNotRentExempt,
    /// Invalid account owner.
    InvalidAccountOwner,
    /// Incorrect authority.
    IncorrectAuthority,
    /// Account is immutable.
    Immutable,
    /// Borsh serialization/deserialization failed.
    BorshIoError,
    /// Computational budget exceeded.
    ComputeBudgetExceeded,
    /// Program-specific error code.
    Custom(u32),

    // -- Catch-all for runtime-internal errors --
    /// An `InstructionError` variant that doesn't map to a common program
    /// error. The string contains the debug representation.
    Runtime(String),
}

impl From<InstructionError> for ProgramError {
    fn from(err: InstructionError) -> Self {
        #[allow(deprecated)]
        match err {
            InstructionError::InvalidArgument => Self::InvalidArgument,
            InstructionError::InvalidInstructionData => Self::InvalidInstructionData,
            InstructionError::InvalidAccountData => Self::InvalidAccountData,
            InstructionError::AccountDataTooSmall => Self::AccountDataTooSmall,
            InstructionError::InsufficientFunds => Self::InsufficientFunds,
            InstructionError::IncorrectProgramId => Self::IncorrectProgramId,
            InstructionError::MissingRequiredSignature => Self::MissingRequiredSignature,
            InstructionError::AccountAlreadyInitialized => Self::AccountAlreadyInitialized,
            InstructionError::UninitializedAccount => Self::UninitializedAccount,
            InstructionError::MissingAccount | InstructionError::NotEnoughAccountKeys => {
                Self::MissingAccount
            }
            InstructionError::InvalidSeeds => Self::InvalidSeeds,
            InstructionError::ArithmeticOverflow => Self::ArithmeticOverflow,
            InstructionError::AccountNotRentExempt => Self::AccountNotRentExempt,
            InstructionError::InvalidAccountOwner => Self::InvalidAccountOwner,
            InstructionError::IncorrectAuthority => Self::IncorrectAuthority,
            InstructionError::Immutable => Self::Immutable,
            InstructionError::BorshIoError => Self::BorshIoError,
            InstructionError::ComputationalBudgetExceeded => Self::ComputeBudgetExceeded,
            InstructionError::Custom(code) => Self::Custom(code),
            other => Self::Runtime(format!("{other:?}")),
        }
    }
}

impl fmt::Display for ProgramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument => write!(f, "invalid argument"),
            Self::InvalidInstructionData => write!(f, "invalid instruction data"),
            Self::InvalidAccountData => write!(f, "invalid account data"),
            Self::AccountDataTooSmall => write!(f, "account data too small"),
            Self::InsufficientFunds => write!(f, "insufficient funds"),
            Self::IncorrectProgramId => write!(f, "incorrect program id"),
            Self::MissingRequiredSignature => write!(f, "missing required signature"),
            Self::AccountAlreadyInitialized => write!(f, "account already initialized"),
            Self::UninitializedAccount => write!(f, "uninitialized account"),
            Self::MissingAccount => write!(f, "missing account"),
            Self::InvalidSeeds => write!(f, "invalid seeds"),
            Self::ArithmeticOverflow => write!(f, "arithmetic overflow"),
            Self::AccountNotRentExempt => write!(f, "account not rent-exempt"),
            Self::InvalidAccountOwner => write!(f, "invalid account owner"),
            Self::IncorrectAuthority => write!(f, "incorrect authority"),
            Self::Immutable => write!(f, "account is immutable"),
            Self::BorshIoError => write!(f, "borsh serialization error"),
            Self::ComputeBudgetExceeded => write!(f, "compute budget exceeded"),
            Self::Custom(code) => write!(f, "custom program error: {code} ({code:#x})"),
            Self::Runtime(msg) => write!(f, "runtime error: {msg}"),
        }
    }
}
