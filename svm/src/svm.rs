use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

use agave_feature_set::FeatureSet;
use agave_syscalls::{
    create_program_runtime_environment_v1, create_program_runtime_environment_v2,
};
use solana_account::{Account, AccountSharedData, ReadableAccount, WritableAccount};
use solana_compute_budget::compute_budget::ComputeBudget;
use solana_hash::Hash;
use solana_instruction::{BorrowedAccountMeta, BorrowedInstruction, Instruction};
use solana_instruction_error::InstructionError;
use solana_instructions_sysvar::construct_instructions_data;
use solana_message::{LegacyMessage, Message, SanitizedMessage};
use solana_program_runtime::invoke_context::{EnvironmentConfig, InvokeContext};
use solana_program_runtime::loaded_programs::ProgramRuntimeEnvironments;
use solana_program_runtime::sysvar_cache::SysvarCache;
use solana_pubkey::Pubkey;
use solana_svm_callback::InvokeContextCallback;
use solana_svm_log_collector::LogCollector;
use solana_svm_timings::ExecuteTimings;
use solana_svm_transaction::instruction::SVMInstruction;
use solana_transaction_context::{IndexOfAccount, TransactionContext};

use crate::program_cache::ProgramCache;
use crate::sysvars::Sysvars;
use crate::token::{Mint, Token};
use crate::{AccountDiff, SvmAccount};

struct NoOpCallback;

impl InvokeContextCallback for NoOpCallback {
    fn get_epoch_stake(&self) -> u64 {
        0
    }
    fn get_epoch_stake_for_vote_account(&self, _: &Pubkey) -> u64 {
        0
    }
    fn is_precompile(&self, _: &Pubkey) -> bool {
        false
    }
    fn process_precompile(
        &self,
        _: &Pubkey,
        _: &[u8],
        _: Vec<&[u8]>,
    ) -> Result<(), solana_precompile_error::PrecompileError> {
        Ok(())
    }
}

pub struct ExecutionResult {
    pub compute_units_consumed: u64,
    pub execution_time_us: u64,
    pub raw_result: Result<(), InstructionError>,
    pub return_data: Vec<u8>,
    pub accounts: Vec<SvmAccount>,
    pub modified_accounts: Vec<AccountDiff>,
    pub logs: Vec<String>,
}

pub struct QuasarSvm {
    pub compute_budget: ComputeBudget,
    pub feature_set: FeatureSet,
    pub logger: Option<Rc<RefCell<LogCollector>>>,
    pub program_cache: ProgramCache,
    pub sysvars: Sysvars,
    accounts: HashMap<Pubkey, Account>,
}

impl Default for QuasarSvm {
    fn default() -> Self {
        Self::new()
    }
}

impl QuasarSvm {
    pub fn new() -> Self {
        let feature_set = FeatureSet::all_enabled();
        let compute_budget = ComputeBudget::new_with_defaults(true, true);
        let program_cache = ProgramCache::new(&feature_set, &compute_budget);

        Self {
            compute_budget,
            feature_set,
            logger: Some(LogCollector::new_ref()),
            program_cache,
            sysvars: Sysvars::default(),
            accounts: HashMap::new(),
        }
    }

    pub fn add_program(&self, program_id: &Pubkey, loader_key: &Pubkey, elf: &[u8]) {
        self.program_cache.add_program(program_id, loader_key, elf);
    }

    /// Store an account in the SVM's account database.
    /// Stored accounts are automatically included when processing transactions.
    pub fn set_account(&mut self, account: SvmAccount) {
        let (pubkey, acct) = account.to_pair();
        self.accounts.insert(pubkey, acct);
    }

    /// Read an account from the SVM's account database.
    pub fn get_account(&self, pubkey: &Pubkey) -> Option<SvmAccount> {
        self.accounts
            .get(pubkey)
            .map(|a| SvmAccount::from_pair(*pubkey, a.clone()))
    }

    /// Give lamports to an account, creating it if it doesn't exist.
    /// The account is owned by the system program.
    pub fn airdrop(&mut self, pubkey: &Pubkey, lamports: u64) {
        let existing = self.accounts.get(pubkey);
        let new_lamports = existing.map_or(lamports, |a| a.lamports + lamports);
        let account = Account {
            lamports: new_lamports,
            data: existing.map_or_else(Vec::new, |a| a.data.clone()),
            owner: existing.map_or(solana_sdk_ids::system_program::ID, |a| a.owner),
            executable: existing.is_some_and(|a| a.executable),
            rent_epoch: 0,
        };
        self.accounts.insert(*pubkey, account);
    }

    /// Create a rent-exempt account with the given space and owner.
    pub fn create_account(&mut self, pubkey: &Pubkey, space: usize, owner: &Pubkey) {
        let lamports = self.sysvars.rent.minimum_balance(space);
        let account = Account {
            lamports,
            data: vec![0u8; space],
            owner: *owner,
            executable: false,
            rent_epoch: 0,
        };
        self.accounts.insert(*pubkey, account);
    }

    /// Set the token balance (amount) of an existing token account in the store.
    /// Panics if the account is not found or is not a valid SPL Token account.
    pub fn set_token_balance(&mut self, address: &Pubkey, amount: u64) {
        let acct = self
            .accounts
            .get_mut(address)
            .unwrap_or_else(|| panic!("set_token_balance: account {address} not found"));
        let mut token = Token::unpack(&acct.data)
            .unwrap_or_else(|| panic!("set_token_balance: account {address} is not a valid token account"));
        token.amount = amount;
        acct.data = token.pack();
    }

    /// Set the supply of an existing mint account in the store.
    /// Panics if the account is not found or is not a valid SPL Mint account.
    pub fn set_mint_supply(&mut self, address: &Pubkey, supply: u64) {
        let acct = self
            .accounts
            .get_mut(address)
            .unwrap_or_else(|| panic!("set_mint_supply: account {address} not found"));
        let mut mint = Mint::unpack(&acct.data)
            .unwrap_or_else(|| panic!("set_mint_supply: account {address} is not a valid mint account"));
        mint.supply = supply;
        acct.data = mint.pack();
    }

    /// Set the clock's unix_timestamp only.
    pub fn warp_to_timestamp(&mut self, timestamp: i64) {
        self.sysvars.clock.unix_timestamp = timestamp;
    }

    /// Execute a transaction without committing any state changes.
    pub fn simulate_transaction(
        &mut self,
        instructions: &[Instruction],
        accounts: &[SvmAccount],
    ) -> ExecutionResult {
        self.reset_logger();

        let pairs: Vec<(Pubkey, Account)> = accounts.iter().map(|a| a.to_pair()).collect();
        let merged = self.merge_accounts(&pairs);

        // Snapshot pre-execution state for diffing
        let pre_accounts: HashMap<Pubkey, SvmAccount> = merged
            .iter()
            .map(|(k, v)| (*k, SvmAccount::from_pair(*k, v.clone())))
            .collect();

        let (sanitized_message, transaction_accounts) =
            self.compile_accounts(instructions, &merged);

        let mut transaction_context = TransactionContext::new(
            transaction_accounts,
            self.sysvars.rent.clone(),
            self.compute_budget.max_instruction_stack_depth,
            self.compute_budget.max_instruction_trace_length,
        );

        let sysvar_cache = self.sysvars.setup_sysvar_cache(&merged);

        let (compute_units_consumed, execution_time_us, raw_result, return_data) =
            self.process_message(&sanitized_message, &mut transaction_context, &sysvar_cache);

        // Read resulting accounts but DON'T commit them
        let resulting_pairs = if raw_result.is_ok() {
            Self::deconstruct_resulting_accounts(&transaction_context, &merged)
        } else {
            merged
        };

        let result_accounts = Self::pairs_to_svm_accounts(&resulting_pairs);
        let modified_accounts = Self::compute_diffs(&pre_accounts, &resulting_pairs);

        let logs = self.drain_logs();

        ExecutionResult {
            compute_units_consumed,
            execution_time_us,
            raw_result,
            return_data,
            accounts: result_accounts,
            modified_accounts,
            logs,
        }
    }

    /// Execute multiple instructions as a single atomic transaction.
    /// Accounts from the SVM's database are merged in automatically.
    pub fn process_transaction(
        &mut self,
        instructions: &[Instruction],
        accounts: &[SvmAccount],
    ) -> ExecutionResult {
        self.reset_logger();

        let pairs: Vec<(Pubkey, Account)> = accounts.iter().map(|a| a.to_pair()).collect();
        let merged = self.merge_accounts(&pairs);

        // Snapshot pre-execution state for diffing
        let pre_accounts: HashMap<Pubkey, SvmAccount> = merged
            .iter()
            .map(|(k, v)| (*k, SvmAccount::from_pair(*k, v.clone())))
            .collect();

        let (sanitized_message, transaction_accounts) =
            self.compile_accounts(instructions, &merged);

        let mut transaction_context = TransactionContext::new(
            transaction_accounts,
            self.sysvars.rent.clone(),
            self.compute_budget.max_instruction_stack_depth,
            self.compute_budget.max_instruction_trace_length,
        );

        let sysvar_cache = self.sysvars.setup_sysvar_cache(&merged);

        let (compute_units_consumed, execution_time_us, raw_result, return_data) =
            self.process_message(&sanitized_message, &mut transaction_context, &sysvar_cache);

        let resulting_pairs = if raw_result.is_ok() {
            let result = Self::deconstruct_resulting_accounts(&transaction_context, &merged);
            self.commit_accounts(&result);
            result
        } else {
            merged
        };

        let result_accounts = Self::pairs_to_svm_accounts(&resulting_pairs);
        let modified_accounts = Self::compute_diffs(&pre_accounts, &resulting_pairs);

        let logs = self.drain_logs();

        ExecutionResult {
            compute_units_consumed,
            execution_time_us,
            raw_result,
            return_data,
            accounts: result_accounts,
            modified_accounts,
            logs,
        }
    }

    /// Merge explicit accounts with the stored account database.
    /// Explicit accounts take priority over stored ones.
    fn merge_accounts(&self, accounts: &[(Pubkey, Account)]) -> Vec<(Pubkey, Account)> {
        let explicit: HashSet<Pubkey> = accounts.iter().map(|(k, _)| *k).collect();
        let mut merged: Vec<(Pubkey, Account)> = self
            .accounts
            .iter()
            .filter(|(k, _)| !explicit.contains(k))
            .map(|(k, v)| (*k, v.clone()))
            .collect();
        merged.extend_from_slice(accounts);
        merged
    }

    /// Write resulting accounts back into the stored account database.
    fn commit_accounts(&mut self, resulting: &[(Pubkey, Account)]) {
        for (pubkey, account) in resulting {
            self.accounts.insert(*pubkey, account.clone());
        }
    }

    fn reset_logger(&mut self) {
        self.logger = Some(LogCollector::new_ref());
    }

    pub fn drain_logs(&self) -> Vec<String> {
        self.logger
            .as_ref()
            .map(|rc| rc.borrow().get_recorded_content().to_vec())
            .unwrap_or_default()
    }

    /// Build the instructions sysvar account.
    fn build_instructions_sysvar(instructions: &[Instruction]) -> (Pubkey, Account) {
        let data = construct_instructions_data(
            instructions
                .iter()
                .map(|ix| BorrowedInstruction {
                    program_id: &ix.program_id,
                    accounts: ix
                        .accounts
                        .iter()
                        .map(|meta| BorrowedAccountMeta {
                            pubkey: &meta.pubkey,
                            is_signer: meta.is_signer,
                            is_writable: meta.is_writable,
                        })
                        .collect(),
                    data: &ix.data,
                })
                .collect::<Vec<_>>()
                .as_slice(),
        );
        (
            solana_instructions_sysvar::ID,
            Account {
                lamports: 0,
                data,
                owner: solana_sysvar_id::ID,
                executable: false,
                rent_epoch: 0,
            },
        )
    }

    /// Compile accounts into the format needed by TransactionContext.
    fn compile_accounts(
        &self,
        instructions: &[Instruction],
        accounts: &[(Pubkey, Account)],
    ) -> (SanitizedMessage, Vec<(Pubkey, AccountSharedData)>) {
        let message = Message::new(instructions, None);
        let sanitized_message =
            SanitizedMessage::Legacy(LegacyMessage::new(message, &HashSet::new()));

        let program_ids: HashSet<Pubkey> = instructions.iter().map(|ix| ix.program_id).collect();
        let account_keys: HashSet<&Pubkey> = accounts.iter().map(|(k, _)| k).collect();

        // Build fallback accounts for programs and sysvars not in the provided list.
        let mut fallbacks = HashMap::new();

        for pid in &program_ids {
            if !account_keys.contains(pid) {
                let program_accounts = self.program_cache.maybe_create_program_accounts(pid);
                if program_accounts.is_empty() {
                    let mut stub = Account::default();
                    stub.set_executable(true);
                    fallbacks.insert(*pid, stub);
                } else {
                    for (key, acct) in program_accounts {
                        fallbacks.insert(key, acct);
                    }
                }
            }
        }

        // Instructions sysvar fallback.
        if !account_keys.contains(&solana_instructions_sysvar::ID) {
            let (id, acct) = Self::build_instructions_sysvar(instructions);
            fallbacks.insert(id, acct);
        }

        let transaction_accounts = sanitized_message
            .account_keys()
            .iter()
            .map(|key| {
                // Try provided accounts first.
                if let Some((_, a)) = accounts.iter().find(|(k, _)| k == key) {
                    return (*key, AccountSharedData::from(a.clone()));
                }
                // Then try fallbacks (already built for top-level program IDs).
                if let Some(a) = fallbacks.get(key) {
                    return (*key, AccountSharedData::from(a.clone()));
                }
                // Sysvar fallback.
                if let Some(a) = self.sysvars.maybe_create_sysvar_account(key) {
                    return (*key, AccountSharedData::from(a));
                }
                // Program account fallback (for CPI targets not in top-level instructions).
                let program_accounts = self.program_cache.maybe_create_program_accounts(key);
                if let Some((_, a)) = program_accounts.into_iter().find(|(k, _)| k == key) {
                    return (*key, AccountSharedData::from(a));
                }
                // Empty account as last resort.
                (*key, AccountSharedData::default())
            })
            .collect();

        (sanitized_message, transaction_accounts)
    }

    fn deconstruct_resulting_accounts(
        transaction_context: &TransactionContext,
        original_accounts: &[(Pubkey, Account)],
    ) -> Vec<(Pubkey, Account)> {
        original_accounts
            .iter()
            .map(|(pubkey, account)| {
                transaction_context
                    .find_index_of_account(pubkey)
                    .map(|index| {
                        let account_ref = transaction_context.accounts().try_borrow(index).unwrap();
                        let resulting_account = Account {
                            lamports: account_ref.lamports(),
                            data: account_ref.data().to_vec(),
                            owner: *account_ref.owner(),
                            executable: account_ref.executable(),
                            rent_epoch: account_ref.rent_epoch(),
                        };
                        (*pubkey, resulting_account)
                    })
                    .unwrap_or((*pubkey, account.clone()))
            })
            .collect()
    }

    /// Convert a list of (Pubkey, Account) pairs to Vec<SvmAccount>.
    fn pairs_to_svm_accounts(pairs: &[(Pubkey, Account)]) -> Vec<SvmAccount> {
        pairs
            .iter()
            .map(|(k, v)| SvmAccount::from_pair(*k, v.clone()))
            .collect()
    }

    /// Compute byte-level diffs between pre-execution and post-execution account states.
    fn compute_diffs(
        pre: &HashMap<Pubkey, SvmAccount>,
        post: &[(Pubkey, Account)],
    ) -> Vec<AccountDiff> {
        let mut diffs = Vec::new();
        for (pubkey, post_account) in post {
            if let Some(pre_account) = pre.get(pubkey) {
                let post_svm = SvmAccount::from_pair(*pubkey, post_account.clone());
                if pre_account.lamports != post_svm.lamports
                    || pre_account.data != post_svm.data
                    || pre_account.owner != post_svm.owner
                {
                    diffs.push(AccountDiff {
                        address: *pubkey,
                        pre: pre_account.clone(),
                        post: post_svm,
                    });
                }
            }
        }
        diffs
    }

    fn process_message<'a>(
        &self,
        sanitized_message: &'a SanitizedMessage,
        transaction_context: &mut TransactionContext<'a>,
        sysvar_cache: &SysvarCache,
    ) -> (u64, u64, Result<(), InstructionError>, Vec<u8>) {
        let mut compute_units_consumed = 0u64;
        let mut timings = ExecuteTimings::default();

        let mut program_cache = self.program_cache.cache();
        let execution_budget = self.compute_budget.to_budget();
        let runtime_features = self.feature_set.runtime_features();

        let program_runtime_environments = ProgramRuntimeEnvironments {
            program_runtime_v1: Arc::new(
                create_program_runtime_environment_v1(
                    &runtime_features,
                    &execution_budget,
                    false,
                    false,
                )
                .unwrap(),
            ),
            program_runtime_v2: Arc::new(create_program_runtime_environment_v2(
                &execution_budget,
                false,
            )),
        };

        let callback = NoOpCallback;

        let mut invoke_context = InvokeContext::new(
            transaction_context,
            &mut program_cache,
            EnvironmentConfig::new(
                Hash::default(),
                5000,
                &callback,
                &runtime_features,
                &program_runtime_environments,
                &program_runtime_environments,
                sysvar_cache,
            ),
            self.logger.clone(),
            self.compute_budget.to_budget(),
            self.compute_budget.to_cost(),
        );

        let mut raw_result: Result<(), InstructionError> = Ok(());

        for (_program_id, compiled_ix) in sanitized_message.program_instructions_iter() {
            let program_id_index = compiled_ix.program_id_index as IndexOfAccount;

            invoke_context
                .prepare_next_top_level_instruction(
                    sanitized_message,
                    &SVMInstruction::from(compiled_ix),
                    program_id_index,
                    &compiled_ix.data,
                )
                .expect("failed to prepare instruction");

            let mut compute_units_consumed_ix = 0u64;
            let invoke_result =
                invoke_context.process_instruction(&mut compute_units_consumed_ix, &mut timings);

            compute_units_consumed += compute_units_consumed_ix;

            if let Err(err) = invoke_result {
                raw_result = Err(err);
                break;
            }
        }

        let return_data = transaction_context.get_return_data().1.to_vec();

        (
            compute_units_consumed,
            timings.details.execute_us.0,
            raw_result,
            return_data,
        )
    }
}
