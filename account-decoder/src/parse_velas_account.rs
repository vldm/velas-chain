use crate::{
    parse_account_data::{ParsableAccount, ParseAccountError},
    
};

use borsh::{BorshDeserialize};
use velas_account::*;
const ACCOUNT_LEN: usize = 67;

pub fn parse_velas_account(data: &[u8]) -> Result<VelasAccountType, ParseAccountError> {
    let account =
        if data.len() == ACCOUNT_LEN {
            VelasAccountType::Account(VAccountInfo::try_from_slice(data).map_err(|_| {
                ParseAccountError::AccountNotParsable(ParsableAccount::VelasAccount)
            })?)
        } else {
            VelasAccountType::Storage(VAccountStorage::try_from_slice(data).map_err(|_| {
                ParseAccountError::AccountNotParsable(ParsableAccount::VelasAccount)
            })?)
        };
    Ok(account)
}

/// A wrapper enum for consistency across programs
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type", content = "info")]
pub enum VelasAccountType {
    Account(VAccountInfo),
    Storage(VAccountStorage),
}

mod velas_account {
    use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
    use serde::{Deserialize, Serialize};
    use solana_sdk::pubkey::Pubkey;

    /// Program states.
    #[repr(C)]
    #[derive(
        BorshSerialize,
        BorshDeserialize,
        BorshSchema,
        PartialEq,
        Debug,
        Clone,
        Serialize,
        Deserialize,
    )]
    pub struct VAccountInfo {
        /// Vaccount version
        pub version: u8,
        /// Genegis owner key that generate Vaccount address
        pub genesis_seed_key: Pubkey,
        /// Storage version
        pub storage_version: u16,
        /// Storage address
        pub storage: Pubkey,
    }

    /// Storage of the basic Vaccount information.
    #[repr(C)]
    #[derive(
        BorshSerialize,
        BorshDeserialize,
        BorshSchema,
        PartialEq,
        Debug,
        Clone,
        Serialize,
        Deserialize,
    )]
    pub struct VAccountStorage {
        /// Owner key in not extended VAccount
        pub owners: Vec<Pubkey>,
        /// Operational in not extended VAccount
        pub operationals: Vec<Operational>,
    }

    /// Operational key state.
    #[repr(C)]
    #[derive(
        Clone,
        Debug,
        Default,
        PartialEq,
        BorshDeserialize,
        BorshSerialize,
        BorshSchema,
        Serialize,
        Deserialize,
    )]
    pub struct Operational {
        /// Operational key
        pub pubkey: Pubkey,
        /// Operational key state
        pub state: OperationalState,
        /// Type of the agent session associated with an operational key
        pub agent_type: Vec<u8>,
        /// Allowed instruction for operational key
        pub scopes: Vec<u8>,
        /// Allowed programs to call
        pub whitelist_programs: Vec<Whitelist>,
        /// Master key is allowed to call any instruction in Vaccount
        pub is_master_key: bool,
    }

    /// Operational key state.
    #[repr(C)]
    #[derive(
        Clone,
        Debug,
        Default,
        PartialEq,
        Eq,
        Hash,
        BorshDeserialize,
        BorshSerialize,
        BorshSchema,
        Serialize,
        Deserialize,
    )]
    pub struct Whitelist {
        /// Allowed to call program code id
        pub program_id: Pubkey,
        /// Allowed to call instruction inside program
        pub scopes: Vec<u8>,
    }

    /// Operational key state.
    #[repr(u8)]
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        BorshDeserialize,
        BorshSerialize,
        BorshSchema,
        Serialize,
        Deserialize,
    )]
    pub enum OperationalState {
        /// Operational key is not yet initialized
        Uninitialized,
        /// Operational key is initialized
        Initialized,
        /// Operational has been frozen by the owner/operational freeze authority.
        Frozen,
    }
    impl Default for OperationalState {
        fn default() -> Self {
            OperationalState::Uninitialized
        }
    }
}
