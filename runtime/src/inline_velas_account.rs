// Partial Velas Account declarations inlined to avoid an external dependency on the velas-account crate
solana_sdk::declare_id!("VAccToWvWJgyPhEwWgFXt51Aqbt1pNR9JL6vAVcekSx"); //TODO: Update real velas account address.

pub(crate) mod new_velas_account_program {
    solana_sdk::declare_id!("VAccToWvWJgyPhEwWgFXt51Aqbt1pNR9JL6vAVcekSx"); //TODO: Update real velas account address.
}

pub const VELAS_ACCOUNT_OWNERS_OFFSET: usize = 32;

pub mod state {
    use super::*;
    use solana_sdk::pubkey::{Pubkey, PUBKEY_BYTES};

    pub struct Account;
    impl Account {
        const LEN: usize = 200;
        pub fn get_packed_len() -> usize {
            Self::LEN
        }
        pub fn owners_from_data(account_data: &[u8]) -> impl IntoIterator<Item = Pubkey> {
            vec![Pubkey::new(
                &account_data
                    [VELAS_ACCOUNT_OWNERS_OFFSET..VELAS_ACCOUNT_OWNERS_OFFSET + PUBKEY_BYTES],
            )]
        }
        pub fn operationals_from_data(account_data: &[u8]) -> impl IntoIterator<Item = Pubkey> {
            vec![Pubkey::new(
                &account_data
                    [VELAS_ACCOUNT_OWNERS_OFFSET..VELAS_ACCOUNT_OWNERS_OFFSET + PUBKEY_BYTES],
            )]
        }
    }

    pub struct OperationalStorage;
    impl OperationalStorage {
        const LEN: usize = 200;
        pub fn get_packed_len() -> usize {
            Self::LEN
        }
        pub fn operationals_from_data(account_data: &[u8]) -> impl IntoIterator<Item = Pubkey> {
            vec![Pubkey::new(
                &account_data
                    [VELAS_ACCOUNT_OWNERS_OFFSET..VELAS_ACCOUNT_OWNERS_OFFSET + PUBKEY_BYTES],
            )]
        }
    }
    pub struct OwnerStorage;
    impl OwnerStorage {
        const LEN: usize = 200;
        pub fn get_packed_len() -> usize {
            Self::LEN
        }
        pub fn owners_from_data(account_data: &[u8]) -> impl IntoIterator<Item = Pubkey> {
            vec![Pubkey::new(
                &account_data
                    [VELAS_ACCOUNT_OWNERS_OFFSET..VELAS_ACCOUNT_OWNERS_OFFSET + PUBKEY_BYTES],
            )]
        }
    }
}
