use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use solana_ledger::blockstore::Blockstore;
use solana_runtime::bank::RewardInfo;
use solana_sdk::{clock::Slot, pubkey::Pubkey};
use solana_transaction_status::Reward;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, Builder, JoinHandle},
    time::Duration,
};

use evm_state::{BlockHeader, BlockNum};

pub type EvmRecorderReceiver = Receiver<(Slot, BlockHeader)>;
pub type EvmRecorderSender = Sender<(Slot, BlockHeader)>;

pub struct EvmRecorderService {
    thread_hdl: JoinHandle<()>,
}

impl EvmRecorderService {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        evm_recorder_receiver: EvmRecorderReceiver,
        blockstore: Arc<Blockstore>,
        exit: &Arc<AtomicBool>,
    ) -> Self {
        let exit = exit.clone();
        let thread_hdl = Builder::new()
            .name("evm-block-writer".to_string())
            .spawn(move || loop {
                if exit.load(Ordering::Relaxed) {
                    break;
                }
                if let Err(RecvTimeoutError::Disconnected) =
                    Self::write_evm_blocks(&evm_recorder_receiver, &blockstore)
                {
                    break;
                }
            })
            .unwrap();
        Self { thread_hdl }
    }

    fn write_evm_blocks(
        evm_records_receiver: &EvmRecorderReceiver,
        blockstore: &Arc<Blockstore>,
    ) -> Result<(), RecvTimeoutError> {
        let (slot, block) = evm_records_receiver.recv_timeout(Duration::from_secs(1))?;

        debug!("Writing evm block num = {}", block.block_number);
        blockstore
            .write_evm_block_header(slot, &block)
            .expect("Expected database write to succed");
        Ok(())
    }

    pub fn join(self) -> thread::Result<()> {
        self.thread_hdl.join()
    }
}
