#![allow(clippy::upper_case_acronyms)]

use std::collections::HashMap;

use jsonrpc_derive::rpc;
use primitive_types::{H256, U256};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

mod serialize;
use self::error::EvmStateError;
use evm_state::{Address, Gas, LogFilterTopicEntry, LogWithLocation, TransactionInReceipt};

pub mod error;
pub use self::error::Error;
pub use self::serialize::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Either<T, U> {
    Left(T),
    Right(U),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum RPCTopicFilter {
    Single(Hex<H256>),
    Or(Vec<Hex<H256>>),
}

impl RPCTopicFilter {
    #[allow(clippy::wrong_self_convention)]
    pub fn into_topics(value: Option<RPCTopicFilter>) -> LogFilterTopicEntry {
        match value {
            Some(RPCTopicFilter::Single(t)) => LogFilterTopicEntry::One(t.0),
            Some(RPCTopicFilter::Or(t)) => {
                LogFilterTopicEntry::Or(t.into_iter().map(|h| h.0).collect())
            }
            None => LogFilterTopicEntry::Any,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCLogFilter {
    pub from_block: Option<String>,
    pub to_block: Option<String>,
    pub address: Option<Hex<Address>>,
    pub topics: Option<Vec<Option<RPCTopicFilter>>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCLog {
    pub removed: bool,
    pub log_index: Hex<usize>,
    pub transaction_index: Hex<usize>,
    pub transaction_hash: Hex<H256>,
    pub block_hash: Hex<H256>,
    pub block_number: Hex<U256>,
    pub address: Hex<Address>,
    pub data: Bytes,
    pub topics: Vec<Hex<H256>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCBlock {
    pub number: Hex<U256>,
    pub hash: Hex<H256>,
    pub parent_hash: Hex<H256>,

    pub size: Hex<usize>,
    pub gas_limit: Hex<Gas>,
    pub gas_used: Hex<Gas>,
    pub timestamp: Hex<u64>,
    pub transactions: Either<Vec<Hex<H256>>, Vec<RPCTransaction>>,
    pub is_finalized: bool,

    pub transactions_root: Hex<H256>,
    pub state_root: Hex<H256>,
    pub receipts_root: Hex<H256>,
    #[serde(with = "serialize::hex_serde::padded")]
    pub nonce: u64,
    pub mix_hash: Hex<H256>,

    pub sha3_uncles: Hex<H256>,
    pub logs_bloom: ethbloom::Bloom, // H2048

    pub miner: Hex<Address>,
    pub difficulty: Hex<U256>,
    pub total_difficulty: Hex<U256>,
    pub extra_data: Bytes,
    pub uncles: Vec<Hex<H256>>,
}
impl Default for RPCBlock {
    fn default() -> Self {
        let empty_uncle: H256 =
            H256::from_hex("0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347")
                .unwrap();
        RPCBlock {
            number: U256::zero().into(),
            hash: H256::zero().into(),
            parent_hash: H256::repeat_byte(0xff).into(),
            size: 0x100.into(),
            gas_limit: U256::one().into(), // avoid divide by zero on explorer, if it calculate percent used.
            gas_used: U256::zero().into(),
            timestamp: 0.into(),
            transactions: Either::Left(vec![]),
            nonce: 0,
            mix_hash: H256::zero().into(),
            logs_bloom: ethbloom::Bloom::zero(), // H2048
            transactions_root: H256::zero().into(),
            state_root: H256::zero().into(),
            receipts_root: H256::zero().into(),
            is_finalized: true,
            miner: Address::zero().into(),
            difficulty: U256::zero().into(),
            total_difficulty: U256::zero().into(),
            uncles: vec![],
            extra_data: b"Velas EVM compatibility layer...".to_vec().into(),
            sha3_uncles: Hex(empty_uncle),
        }
    }
}

impl RPCBlock {
    pub fn new_from_head(
        header: evm_state::BlockHeader,
        confirmed: bool,
        transactions: Either<Vec<Hex<H256>>, Vec<RPCTransaction>>,
    ) -> Self {
        let empty_uncle = evm_state::empty_ommers_hash();
        let block_hash = header.hash();
        let extra_data = match header.version {
            evm_state::BlockVersion::InitVersion => {
                b"Velas EVM compatibility layer...".to_vec().into()
            }
            evm_state::BlockVersion::VersionConsistentHashes => {
                b"Velas EVM compatibility layer.v2".to_vec().into()
            }
        };
        RPCBlock {
            number: U256::from(header.block_number).into(),
            hash: block_hash.into(),
            parent_hash: header.parent_hash.into(),
            gas_limit: Hex(header.gas_limit.into()),
            gas_used: Hex(header.gas_used.into()),
            timestamp: Hex(header.timestamp),
            transactions,
            nonce: header.native_chain_slot,
            mix_hash: header.native_chain_hash.into(),
            logs_bloom: header.logs_bloom, // H2048
            transactions_root: Hex(header.transactions_root),
            state_root: Hex(header.state_root),
            receipts_root: Hex(header.receipts_root),
            extra_data,
            is_finalized: confirmed,
            size: 0x100.into(),
            miner: Address::zero().into(),
            difficulty: U256::zero().into(),
            total_difficulty: U256::zero().into(),
            sha3_uncles: Hex(empty_uncle),
            uncles: vec![],
        }
    }

    pub fn to_native_block(&self, version: evm_state::BlockVersion) -> evm_state::BlockHeader {
        evm_state::BlockHeader {
            state_root: self.state_root.0,
            transactions_root: self.transactions_root.0,
            receipts_root: self.receipts_root.0,
            native_chain_hash: self.mix_hash.0,
            native_chain_slot: self.nonce,
            parent_hash: self.parent_hash.0,
            transactions: vec![],
            logs_bloom: self.logs_bloom,
            block_number: self.number.0.as_u64(),
            gas_limit: self.gas_limit.0.as_u64(),
            gas_used: self.gas_used.0.as_u64(),
            timestamp: self.timestamp.0,
            version,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCTransaction {
    pub from: Option<Hex<Address>>,
    pub to: Option<Hex<Address>>,
    pub creates: Option<Hex<Address>>,
    pub gas: Option<Hex<Gas>>,
    pub gas_price: Option<Hex<Gas>>,
    pub value: Option<Hex<U256>>,
    #[serde(alias = "data")]
    pub input: Option<Bytes>,
    pub nonce: Option<Hex<U256>>,

    pub hash: Option<Hex<H256>>,
    pub block_hash: Option<Hex<H256>>,
    pub block_number: Option<Hex<U256>>,
    pub transaction_index: Option<Hex<usize>>,
    #[serde(rename = "V")]
    pub v: Option<Hex<u64>>,
    #[serde(rename = "R")]
    pub r: Option<Hex<U256>>,
    #[serde(rename = "S")]
    pub s: Option<Hex<U256>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCReceipt {
    pub transaction_hash: Hex<H256>,
    pub transaction_index: Hex<usize>,
    pub block_hash: Hex<H256>,
    pub block_number: Hex<U256>,
    pub cumulative_gas_used: Hex<Gas>,
    pub gas_used: Hex<Gas>,
    pub contract_address: Option<Hex<Address>>,
    pub logs_bloom: ethbloom::Bloom, // H2048
    pub to: Option<Hex<Address>>,
    pub from: Option<Hex<Address>>,
    pub logs: Vec<RPCLog>,
    pub status: Hex<usize>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCTrace {
    pub gas: Hex<Gas>,
    pub return_value: Bytes,
    pub struct_logs: Vec<RPCStep>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RPCTraceConfig {
    #[serde(default)]
    pub disable_memory: bool,
    #[serde(default)]
    pub disable_stack: bool,
    #[serde(default)]
    pub disable_storage: bool,
    #[serde(default)]
    pub breakpoints: Option<RPCBreakpointConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RPCBreakpointConfig {
    pub source_map: HashMap<Hex<H256>, RPCSourceMapConfig>,
    pub breakpoints: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCSourceMapConfig {
    pub source_map: String,
    pub source_list: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCBlockTrace {
    pub struct_logs: Vec<RPCStep>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCStep {
    pub depth: usize,
    pub error: String,
    pub gas: Hex<Gas>,
    pub gas_cost: Hex<Gas>,
    pub op: u8,
    pub pc: usize,
    pub opcode_pc: usize,
    pub code_hash: Hex<H256>,
    pub address: Hex<Address>,
    pub breakpoint_index: Option<usize>,
    pub breakpoint: Option<String>,
    pub memory: Option<Vec<Bytes>>,
    pub stack: Option<Vec<Hex<U256>>>,
    pub storage: Option<HashMap<Hex<U256>, Hex<U256>>>,
}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// #[serde(rename_all = "camelCase")]
// pub struct RPCDump {
//     pub accounts: HashMap<Hex<Address>, RPCDumpAccount>,
//     pub root: Hex<H256>,
// }

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RPCDumpAccountBasic {
    pub balance: Hex<U256>,
    // pub code: Bytes,
    // pub code_hash: Hex<H256>,
    pub nonce: Hex<U256>,
    // pub root: Hex<H256>,
    // pub storage: HashMap<Hex<U256>, Hex<U256>>,
}

pub mod trace {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Res {
        gas_used: Hex<U256>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "address")]
        contract: Option<Address>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<Bytes>,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<Bytes>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ExitError>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum ExitError {
        OutOfGas,
        BadJumpDestination,
        BadInstruction,
        StackUnderflow,
        OutOfStack,
        BuiltIn,
        Internal,
        MutableCallInStaticContext,
        OutOfBounds,
        Reverted,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type", content = "action")]
    #[serde(rename_all = "snake_case")]
    pub enum Action {
        Call {
            from: Hex<Address>,
            to: Hex<Address>,
            value: Hex<U256>,
            gas: Hex<U256>,
            input: Bytes,
            // #[serde(skip_serializing_if = "Option::is_none")]
            #[serde(rename = "callType")]
            call_type: CallScheme,
        },
        Create {
            #[serde(rename = "from")]
            caller: Hex<Address>,
            value: Hex<U256>,
            gas: Hex<U256>,
            #[serde(rename = "init")]
            init_code: Bytes,
            #[serde(rename = "creationMethod")]
            creation_method: CreateScheme,
        },
        // TODO: Trace suicide?!
        // Suicide {
        //     address: Address,
        //     refund_address: Address,
        //     balance: U256,
        // },
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Trace {
        #[serde(flatten)]
        pub action: Action,
        pub result: Res,
        pub subtraces: Hex<usize>,
        pub trace_address: Vec<usize>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TraceResultsWithTransactionHash {
        pub output: Bytes,
        pub trace: Vec<Trace>,
        pub transaction_hash: Option<Hex<H256>>,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum CallScheme {
        Call,
        CallCode,
        DelegateCall,
        StaticCall,
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum CreateScheme {
        Create,
        Create2,
        Unimplemented,
    }

    impl From<evm_state::CallScheme> for CallScheme {
        fn from(scheme: evm_state::CallScheme) -> Self {
            match scheme {
                evm_state::CallScheme::Call => Self::Call,
                evm_state::CallScheme::CallCode => Self::CallCode,
                evm_state::CallScheme::DelegateCall => Self::DelegateCall,
                evm_state::CallScheme::StaticCall => Self::StaticCall,
            }
        }
    }
    impl From<evm_state::CreateScheme> for CreateScheme {
        fn from(scheme: evm_state::CreateScheme) -> Self {
            match scheme {
                evm_state::CreateScheme::Legacy { .. } => Self::Create,
                evm_state::CreateScheme::Create2 { .. } => Self::Create2,
                _ => Self::Unimplemented,
            }
        }
    }
    impl From<evm_state::executor::Trace> for Trace {
        fn from(trace: evm_state::executor::Trace) -> Self {
            Self {
                action: trace.action.into(),
                result: trace.result.into(),
                subtraces: trace.subtraces.into(),
                trace_address: trace.trace_address.into_iter().map(From::from).collect(),
            }
        }
    }
    impl From<evm_state::executor::Action> for Action {
        fn from(action: evm_state::executor::Action) -> Self {
            match action {
                evm_state::executor::Action::Call {
                    code,
                    input,
                    context,
                    gas,
                    call_type,
                } => Self::Call {
                    input: input.into(),
                    from: context.caller.into(),
                    to: code.into(),
                    gas: gas.into(),
                    value: context.apparent_value.into(),
                    call_type: call_type.map(From::from).unwrap_or(CallScheme::Call),
                },
                evm_state::executor::Action::Create {
                    caller,
                    value,
                    gas,
                    init_code,
                    creation_method,
                } => Self::Create {
                    caller: caller.into(),
                    value: value.into(),
                    gas: gas.into(),
                    init_code: init_code.into(),
                    creation_method: creation_method.into(),
                },
            }
        }
    }

    impl From<evm_state::executor::Res> for Res {
        fn from(result: evm_state::executor::Res) -> Self {
            // TODO: Add rest errors panic!()/todo!(), and other keywords for better search.
            let error = match result.reason {
                evm_state::ExitReason::Succeed(_) => None,
                evm_state::ExitReason::Revert(_) => Some(ExitError::Reverted),
                evm_state::ExitReason::Error(E) => None,
                evm_state::ExitReason::Fatal(_) => None,
            };
            let (output, code) = if error.is_some() {
                (None, None)
            } else if result.contract.is_none() {
                // If contract, output = code
                (Some(result.output.into()), None)
            } else {
                (None, Some(result.output.into()))
            };
            Res {
                gas_used: result.gas_used.into(),
                contract: result.contract,
                code,
                output,
                error,
            }
        }
    }
}

pub use basic::BasicERPC;
pub use bridge::BridgeERPC;
pub use chain_mock::ChainMockERPC;

pub mod basic {
    use super::*;

    #[rpc]
    pub trait BasicERPC {
        type Metadata;

        #[rpc(meta, name = "eth_blockNumber")]
        fn block_number(&self, meta: Self::Metadata) -> Result<Hex<usize>, Error>;

        #[rpc(meta, name = "eth_getBalance")]
        fn balance(
            &self,
            meta: Self::Metadata,
            address: Hex<Address>,
            block: Option<String>,
        ) -> Result<Hex<U256>, Error>;

        #[rpc(meta, name = "eth_getStorageAt")]
        fn storage_at(
            &self,
            meta: Self::Metadata,
            address: Hex<Address>,
            data: Hex<H256>,
            block: Option<String>,
        ) -> Result<Hex<H256>, Error>;

        #[rpc(meta, name = "eth_getTransactionCount")]
        fn transaction_count(
            &self,
            meta: Self::Metadata,
            address: Hex<Address>,
            block: Option<String>,
        ) -> Result<Hex<U256>, Error>;

        #[rpc(meta, name = "eth_getCode")]
        fn code(
            &self,
            meta: Self::Metadata,
            address: Hex<Address>,
            block: Option<String>,
        ) -> Result<Bytes, Error>;

        #[rpc(meta, name = "eth_getBlockByHash")]
        fn block_by_hash(
            &self,
            meta: Self::Metadata,
            block_hash: Hex<H256>,
            full: bool,
        ) -> Result<Option<RPCBlock>, Error>;

        #[rpc(meta, name = "eth_getBlockByNumber")]
        fn block_by_number(
            &self,
            meta: Self::Metadata,
            block: String,
            full: bool,
        ) -> Result<Option<RPCBlock>, Error>;

        #[rpc(meta, name = "eth_getTransactionByHash")]
        fn transaction_by_hash(
            &self,
            meta: Self::Metadata,
            tx_hash: Hex<H256>,
        ) -> Result<Option<RPCTransaction>, Error>;

        #[rpc(meta, name = "eth_getTransactionReceipt")]
        fn transaction_receipt(
            &self,
            meta: Self::Metadata,
            tx_hash: Hex<H256>,
        ) -> Result<Option<RPCReceipt>, Error>;

        #[rpc(meta, name = "eth_call")]
        fn call(
            &self,
            meta: Self::Metadata,
            tx: RPCTransaction,
            block: Option<String>,
            meta_keys: Option<Vec<String>>,
        ) -> Result<Bytes, Error>;

        #[rpc(meta, name = "trace_call")]
        fn trace_call(
            &self,
            meta: Self::Metadata,
            tx: RPCTransaction,
            traces: Vec<String>,
            block: Option<String>,
            meta_keys: Option<Vec<String>>,
        ) -> Result<trace::TraceResultsWithTransactionHash, Error>;

        #[rpc(meta, name = "trace_replayTransaction")]
        fn trace_replayTransaction(
            &self,
            meta: Self::Metadata,
            tx_hash: Hex<H256>,
            traces: Vec<String>,
            meta_keys: Option<Vec<String>>,
        ) -> Result<Option<trace::TraceResultsWithTransactionHash>, Error>;

        #[rpc(meta, name = "trace_replayBlockTransactions")]
        fn trace_replayBlock(
            &self,
            meta: Self::Metadata,
            block: String,
            traces: Vec<String>,
            meta_keys: Option<Vec<String>>,
        ) -> Result<Vec<trace::TraceResultsWithTransactionHash>, Error>;

        #[rpc(meta, name = "eth_estimateGas")]
        fn estimate_gas(
            &self,
            meta: Self::Metadata,
            tx: RPCTransaction,
            block: Option<String>,
            meta_keys: Option<Vec<String>>,
        ) -> Result<Hex<Gas>, Error>;

        #[rpc(meta, name = "eth_getLogs")]
        fn logs(
            &self,
            meta: Self::Metadata,
            log_filter: RPCLogFilter,
        ) -> Result<Vec<RPCLog>, Error>;
    }
}

pub mod chain_mock {
    use super::*;

    #[rpc]
    pub trait ChainMockERPC {
        type Metadata;

        #[rpc(meta, name = "web3_clientVersion")]
        fn client_version(&self, meta: Self::Metadata) -> Result<String, Error>;

        #[rpc(meta, name = "web3_sha3")]
        fn sha3(&self, meta: Self::Metadata, bytes: Bytes) -> Result<Hex<H256>, Error>;

        #[rpc(meta, name = "net_version")]
        fn network_id(&self, meta: Self::Metadata) -> Result<String, Error>;

        #[rpc(meta, name = "net_listening")]
        fn is_listening(&self, meta: Self::Metadata) -> Result<bool, Error>;

        #[rpc(meta, name = "net_peerCount")]
        fn peer_count(&self, meta: Self::Metadata) -> Result<Hex<usize>, Error>;

        #[rpc(meta, name = "eth_chainId")]
        fn chain_id(&self, meta: Self::Metadata) -> Result<Hex<u64>, Error>;

        #[rpc(meta, name = "eth_protocolVersion")]
        fn protocol_version(&self, meta: Self::Metadata) -> Result<String, Error>;

        #[rpc(meta, name = "eth_syncing")]
        fn is_syncing(&self, meta: Self::Metadata) -> Result<bool, Error>;

        #[rpc(meta, name = "eth_coinbase")]
        fn coinbase(&self, meta: Self::Metadata) -> Result<Hex<Address>, Error>;

        #[rpc(meta, name = "eth_mining")]
        fn is_mining(&self, meta: Self::Metadata) -> Result<bool, Error>;

        #[rpc(meta, name = "eth_hashrate")]
        fn hashrate(&self, meta: Self::Metadata) -> Result<String, Error>;

        #[rpc(meta, name = "eth_getUncleByBlockHashAndIndex")]
        fn uncle_by_block_hash_and_index(
            &self,
            meta: Self::Metadata,
            block_hash: Hex<H256>,
            uncle_id: Hex<U256>,
        ) -> Result<Option<RPCBlock>, Error>;

        #[rpc(meta, name = "eth_getUncleByBlockNumberAndIndex")]
        fn uncle_by_block_number_and_index(
            &self,
            meta: Self::Metadata,
            block: String,
            uncle_id: Hex<U256>,
        ) -> Result<Option<RPCBlock>, Error>;

        #[rpc(meta, name = "eth_getBlockTransactionCountByHash")]
        fn block_transaction_count_by_hash(
            &self,
            meta: Self::Metadata,
            block_hash: Hex<H256>,
        ) -> Result<Option<Hex<usize>>, Error>;

        #[rpc(meta, name = "eth_getBlockTransactionCountByNumber")]
        fn block_transaction_count_by_number(
            &self,
            meta: Self::Metadata,
            block: String,
        ) -> Result<Option<Hex<usize>>, Error>;

        #[rpc(meta, name = "eth_getUncleCountByBlockHash")]
        fn block_uncles_count_by_hash(
            &self,
            meta: Self::Metadata,
            block_hash: Hex<H256>,
        ) -> Result<Option<Hex<usize>>, Error>;

        #[rpc(meta, name = "eth_getUncleCountByBlockNumber")]
        fn block_uncles_count_by_number(
            &self,
            meta: Self::Metadata,
            block: String,
        ) -> Result<Option<Hex<usize>>, Error>;

        #[rpc(meta, name = "eth_getTransactionByBlockHashAndIndex")]
        fn transaction_by_block_hash_and_index(
            &self,
            meta: Self::Metadata,
            block_hash: Hex<H256>,
            tx_id: Hex<U256>,
        ) -> Result<Option<RPCTransaction>, Error>;

        #[rpc(meta, name = "eth_getTransactionByBlockNumberAndIndex")]
        fn transaction_by_block_number_and_index(
            &self,
            meta: Self::Metadata,
            block: String,
            tx_id: Hex<U256>,
        ) -> Result<Option<RPCTransaction>, Error>;
    }
}

pub mod bridge {
    use super::*;

    #[rpc]
    pub trait BridgeERPC {
        type Metadata;

        #[rpc(meta, name = "eth_accounts")]
        fn accounts(&self, meta: Self::Metadata) -> Result<Vec<Hex<Address>>, Error>;

        #[rpc(meta, name = "eth_sign")]
        fn sign(
            &self,
            meta: Self::Metadata,
            address: Hex<Address>,
            data: Bytes,
        ) -> Result<Bytes, Error>;

        #[rpc(meta, name = "eth_sendTransaction")]
        fn send_transaction(
            &self,
            meta: Self::Metadata,
            tx: RPCTransaction,
            meta_keys: Option<Vec<String>>,
        ) -> Result<Hex<H256>, Error>;

        #[rpc(meta, name = "eth_sendRawTransaction")]
        fn send_raw_transaction(
            &self,
            meta: Self::Metadata,
            tx: Bytes,
            meta_keys: Option<Vec<String>>,
        ) -> Result<Hex<H256>, Error>;

        #[rpc(meta, name = "eth_gasPrice")]
        fn gas_price(&self, meta: Self::Metadata) -> Result<Hex<Gas>, Error>;

        #[rpc(meta, name = "eth_getCompilers")]
        fn compilers(&self, meta: Self::Metadata) -> Result<Vec<String>, Error>;
    }
}

// #[rpc]
// pub trait FilterRPC {
//     #[rpc(meta, name = "eth_newFilter")]
//     fn new_filter(&self, RPCLogFilter) -> Result<String, Error>;
//     #[rpc(meta, name = "eth_newBlockFilter")]
//     fn new_block_filter(&self) -> Result<String, Error>;
//     #[rpc(meta, name = "eth_newPendingTransactionFilter")]
//     fn new_pending_transaction_filter(&self) -> Result<String, Error>;
//     #[rpc(meta, name = "eth_uninstallFilter")]
//     fn uninstall_filter(&self, String) -> Result<bool, Error>;

//     #[rpc(meta, name = "eth_getFilterChanges")]
//     fn filter_changes(&self, String) -> Result<Either<Vec<String>, Vec<RPCLog>>, Error>;
//     #[rpc(meta, name = "eth_getFilterLogs")]
//     fn filter_logs(&self, String) -> Result<Vec<RPCLog>, Error>;
// }

// #[rpc]
// pub trait DebugRPC {
//     #[rpc(name = "debug_getBlockRlp")]
//     fn block_rlp(&self, usize) -> Result<Bytes, Error>;
//     #[rpc(name = "debug_traceTransaction")]
//     fn trace_transaction(&self, Hex<H256>, Option<RPCTraceConfig>)
//                             -> Result<RPCTrace, Error>;
//     #[rpc(name = "debug_traceBlock")]
//     fn trace_block(&self, Bytes, Option<RPCTraceConfig>)
//                     -> Result<RPCBlockTrace, Error>;
//     #[rpc(name = "debug_traceBlockByNumber")]
//     fn trace_block_by_number(&self, usize, Option<RPCTraceConfig>)
//                                 -> Result<RPCBlockTrace, Error>;
//     #[rpc(name = "debug_traceBlockByHash")]
//     fn trace_block_by_hash(&self, Hex<H256>, Option<RPCTraceConfig>)
//                             -> Result<RPCBlockTrace, Error>;
//     #[rpc(name = "debug_traceBlockFromFile")]
//     fn trace_block_from_file(&self, String, Option<RPCTraceConfig>)
//                                 -> Result<RPCBlockTrace, Error>;
//     #[rpc(name = "debug_dumpBlock")]
//     fn dump_block(&self, usize) -> Result<RPCDump, Error>;
// }

// pub fn rpc_loop<P: 'static + Patch + Send>(
//     state: Arc<Mutex<MinerState>>, addr: &SocketAddr, channel: Sender<bool>
// ) {
//     let rpc = serves::MinerEthereumRPC::<P>::new(state.clone(), channel);
//     let filter = serves::MinerFilterRPC::<P>::new(state.clone());
//     let debug = serves::MinerDebugRPC::<P>::new(state);

//     let mut io = IoHandler::default();

//     io.extend_with(rpc.to_delegate());
//     io.extend_with(filter.to_delegate());
//     io.extend_with(debug.to_delegate());

//     let server = ServerBuilder::new(io)
//         .cors(DomainsValidation::AllowOnly(vec![
//             AccessControlAllowOrigin::Any,
//             AccessControlAllowOrigin::Null,
//         ]))
//         .start_http(addr)
//         .expect("Expect to build HTTP RPC server");

//     server.wait();
// }

impl RPCTransaction {
    pub fn new_from_receipt(
        receipt: evm_state::transactions::TransactionReceipt,
        tx_hash: H256,
        block_hash: H256,
        chain_id: u64,
    ) -> Result<Self, crate::Error> {
        let (to, creates, from, gas_limit, gas_price, input, value, nonce, v, r, s) = match receipt
            .transaction
        {
            TransactionInReceipt::Signed(tx) => {
                let from = tx.caller().with_context(|| EvmStateError)?;
                let gas_limit = tx.gas_limit;
                let gas_price = tx.gas_price;
                let input = tx.input;
                let value = tx.value;
                let nonce = tx.nonce;
                let (to, creates) = match tx.action {
                    evm_state::transactions::TransactionAction::Call(address) => {
                        (Some(address), None)
                    }
                    evm_state::transactions::TransactionAction::Create => (
                        None,
                        Some(
                            evm_state::transactions::TransactionAction::Create.address(from, nonce),
                        ),
                    ),
                };
                (
                    to,
                    creates,
                    from,
                    gas_limit,
                    gas_price,
                    input,
                    value,
                    nonce,
                    tx.signature.v,
                    tx.signature.r.as_bytes().into(),
                    tx.signature.s.as_bytes().into(),
                )
            }
            TransactionInReceipt::Unsigned(tx) => {
                let from = tx.caller;
                let gas_limit = tx.unsigned_tx.gas_limit;
                let gas_price = tx.unsigned_tx.gas_price;
                let input = tx.unsigned_tx.input;
                let value = tx.unsigned_tx.value;
                let nonce = tx.unsigned_tx.nonce;
                let (to, creates) = match tx.unsigned_tx.action {
                    evm_state::transactions::TransactionAction::Call(address) => {
                        (Some(address), None)
                    }
                    evm_state::transactions::TransactionAction::Create => (
                        None,
                        Some(
                            evm_state::transactions::TransactionAction::Create.address(from, nonce),
                        ),
                    ),
                };
                let v = chain_id * 2 + 35;
                (
                    to,
                    creates,
                    from,
                    gas_limit,
                    gas_price,
                    input,
                    value,
                    nonce,
                    v,
                    U256::zero(),
                    U256::zero(),
                )
            }
        };
        Ok(RPCTransaction {
            from: Some(from.into()),
            to: to.map(Hex),
            creates: creates.map(Hex),
            gas: Some(gas_limit.into()),
            gas_price: Some(gas_price.into()),
            value: Some(value.into()),
            input: Some(input.into()),
            nonce: Some(nonce.into()),
            hash: Some(tx_hash.into()),
            transaction_index: Some((receipt.index as usize).into()),
            block_hash: Some(block_hash.into()),
            block_number: Some(Hex(receipt.block_number.into())),
            v: Some(Hex(v)),
            r: Some(Hex(r)),
            s: Some(Hex(s)),
        })
    }

    pub fn from_transaction(tx: evm_state::Transaction) -> Result<Self, crate::Error> {
        let from = tx.caller().with_context(|| EvmStateError)?;
        let hash = tx.tx_id_hash();
        let gas_limit = tx.gas_limit;
        let gas_price = tx.gas_price;
        let input = tx.input;
        let value = tx.value;
        let nonce = tx.nonce;
        let (to, creates) = match tx.action {
            evm_state::transactions::TransactionAction::Call(address) => (Some(address), None),
            evm_state::transactions::TransactionAction::Create => (
                None,
                Some(evm_state::transactions::TransactionAction::Create.address(from, nonce)),
            ),
        };
        Ok(RPCTransaction {
            from: Some(from.into()),
            to: to.map(Hex),
            creates: creates.map(Hex),
            gas: Some(gas_limit.into()),
            gas_price: Some(gas_price.into()),
            value: Some(value.into()),
            input: Some(input.into()),
            nonce: Some(nonce.into()),
            hash: Some(hash.into()),
            transaction_index: None,
            block_hash: None,
            block_number: None,
            v: Some(Hex(tx.signature.v)),
            r: Some(Hex(tx.signature.r.as_bytes().into())),
            s: Some(Hex(tx.signature.s.as_bytes().into())),
        })
    }
}

impl RPCReceipt {
    pub fn new_from_receipt(
        receipt: evm_state::transactions::TransactionReceipt,
        tx_hash: H256,
        block_hash: H256,
    ) -> Result<Self, crate::Error> {
        let (from, to, contract_address) = match receipt.transaction {
            TransactionInReceipt::Signed(tx) => {
                let from = tx.caller().with_context(|| EvmStateError)?;
                let nonce = tx.nonce;
                let (to, creates) = match tx.action {
                    evm_state::transactions::TransactionAction::Call(address) => {
                        (Some(address), None)
                    }
                    evm_state::transactions::TransactionAction::Create => (
                        None,
                        Some(
                            evm_state::transactions::TransactionAction::Create.address(from, nonce),
                        ),
                    ),
                };
                (from, to, creates)
            }
            TransactionInReceipt::Unsigned(tx) => {
                let from = tx.caller;
                let nonce = tx.unsigned_tx.nonce;
                let (to, creates) = match tx.unsigned_tx.action {
                    evm_state::transactions::TransactionAction::Call(address) => {
                        (Some(address), None)
                    }
                    evm_state::transactions::TransactionAction::Create => (
                        None,
                        Some(
                            evm_state::transactions::TransactionAction::Create.address(from, nonce),
                        ),
                    ),
                };

                (from, to, creates)
            }
        };

        let tx_index: Hex<_> = (receipt.index as usize).into();
        let block_number = Hex(U256::from(receipt.block_number));

        let logs = receipt
            .logs
            .into_iter()
            .enumerate()
            .map(|(id, log)| RPCLog {
                removed: false,
                log_index: Hex(id),
                transaction_hash: tx_hash.into(),
                transaction_index: tx_index,
                block_hash: block_hash.into(),
                block_number,
                data: log.data.into(),
                topics: log.topics.into_iter().map(Hex).collect(),
                address: Hex(log.address),
            })
            .collect();

        Ok(RPCReceipt {
            from: Hex(from).into(),
            to: to.map(Hex),
            contract_address: contract_address.map(Hex),
            gas_used: Hex(receipt.used_gas.into()),
            cumulative_gas_used: Hex(receipt.used_gas.into()),
            transaction_hash: tx_hash.into(),
            transaction_index: tx_index,
            block_hash: block_hash.into(),
            block_number,
            logs_bloom: receipt.logs_bloom,
            logs,
            status: Hex(if let evm_state::ExitReason::Succeed(_) = receipt.status {
                1
            } else {
                0
            }),
        })
    }
}

impl From<LogWithLocation> for RPCLog {
    fn from(log: LogWithLocation) -> Self {
        RPCLog {
            removed: false,
            transaction_hash: log.transaction_hash.into(),
            transaction_index: (log.transaction_id as usize).into(),
            block_number: Hex(log.block_num.into()),
            block_hash: Hex(H256::zero()),
            log_index: Hex(0),
            address: Hex(log.address),
            topics: log.topics.into_iter().map(Hex).collect(),
            data: Bytes(log.data),
        }
    }
}
