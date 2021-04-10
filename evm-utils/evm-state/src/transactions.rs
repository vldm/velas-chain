use evm::backend::Log;
use primitive_types::{H160, H256, U256};
use rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::str::FromStr;

use crate::error::*;
use secp256k1::{
    recovery::{RecoverableSignature, RecoveryId},
    Message,
};
use snafu::ResultExt;

pub use secp256k1::{PublicKey, SecretKey, SECP256K1};

pub type Address = H160;
pub type Gas = U256;

/// Etherium transaction.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Transaction {
    pub nonce: U256,
    pub gas_price: Gas,
    pub gas_limit: Gas,
    pub action: TransactionAction,
    pub value: U256,
    pub signature: TransactionSignature,
    pub input: Vec<u8>,
}

impl Transaction {
    pub fn caller(&self) -> Result<Address, Error> {
        let unsigned = UnsignedTransaction::from((*self).clone());
        let transaction_hash = unsigned.signing_hash(self.signature.chain_id());
        let sig = self
            .signature
            .to_recoverable_signature()
            .context(UnrecoverableCaller { transaction_hash })?;
        let public_key = SECP256K1
            .recover(
                &Message::from_slice(&transaction_hash.as_bytes()).unwrap(),
                &sig,
            )
            .context(UnrecoverableCaller { transaction_hash })?;
        Ok(addr_from_public_key(&public_key))
    }

    pub fn address(&self) -> Result<Address, Error> {
        Ok(self.action.address(self.caller()?, self.nonce))
    }

    fn signing_rlp_append(&self, s: &mut RlpStream, chain_id: Option<u64>) {
        s.begin_list(if chain_id.is_some() { 9 } else { 6 });
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas_limit);
        s.append(&self.action);
        s.append(&self.value);
        s.append(&self.input);

        if let Some(chain_id) = chain_id {
            s.append(&chain_id);
            s.append(&0u8);
            s.append(&0u8);
        }
    }

    pub fn signing_hash(&self) -> H256 {
        let chain_id = self.signature.chain_id();
        let mut stream = RlpStream::new();
        self.signing_rlp_append(&mut stream, chain_id);
        H256::from_slice(Keccak256::digest(&stream.as_raw()).as_slice())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UnsignedTransaction {
    pub nonce: U256,
    pub gas_price: U256,
    pub gas_limit: U256,
    pub action: TransactionAction,
    pub value: U256,
    pub input: Vec<u8>,
}

impl UnsignedTransaction {
    fn signing_rlp_append(&self, s: &mut RlpStream, chain_id: Option<u64>) {
        s.begin_list(if chain_id.is_some() { 9 } else { 6 });
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas_limit);
        s.append(&self.action);
        s.append(&self.value);
        s.append(&self.input);

        if let Some(chain_id) = chain_id {
            s.append(&chain_id);
            s.append(&0u8);
            s.append(&0u8);
        }
    }

    pub fn signing_hash(&self, chain_id: Option<u64>) -> H256 {
        let mut stream = RlpStream::new();
        self.signing_rlp_append(&mut stream, chain_id);
        H256::from_slice(Keccak256::digest(&stream.as_raw()).as_slice())
    }

    pub fn sign(self, key: &SecretKey, chain_id: Option<u64>) -> Transaction {
        let hash = self.signing_hash(chain_id);
        // hash is always MESSAGE_SIZE bytes.
        let msg = { Message::from_slice(hash.as_bytes()).unwrap() };

        // SecretKey and Message are always valid.
        let s = { SECP256K1.sign_recoverable(&msg, key) };
        let (rid, sig) = { s.serialize_compact() };

        let sig = TransactionSignature {
            v: ({ rid.to_i32() }
                + if let Some(n) = chain_id {
                    (35 + n * 2) as i32
                } else {
                    27
                }) as u64,
            r: H256::from_slice(&sig[0..32]),
            s: H256::from_slice(&sig[32..64]),
        };

        Transaction {
            nonce: self.nonce,
            gas_price: self.gas_price,
            gas_limit: self.gas_limit,
            action: self.action,
            value: self.value,
            input: self.input,
            signature: sig,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum TransactionAction {
    Call(Address),
    Create,
}

impl TransactionAction {
    pub fn address(&self, caller: Address, nonce: U256) -> Address {
        match self {
            TransactionAction::Call(address) => *address,
            TransactionAction::Create => {
                let mut rlp = RlpStream::new_list(2);
                rlp.append(&caller);
                rlp.append(&nonce);

                Address::from(H256::from_slice(
                    Keccak256::digest(rlp.out().as_ref()).as_slice(),
                ))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct TransactionSignature {
    pub v: u64,
    pub r: H256,
    pub s: H256,
}

impl TransactionSignature {
    pub fn standard_v(&self) -> u8 {
        let v = self.v;
        if v == 27 || v == 28 || v > 36 {
            ((v - 1) % 2) as u8
        } else {
            4
        }
    }

    pub fn is_low_s(&self) -> bool {
        self.s
            <= H256::from_str("0x7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF5D576E7357A4501DDFE92F46681B20A0")
                .unwrap()
    }

    pub fn is_valid(&self) -> bool {
        self.standard_v() <= 1
            && self.r
                < H256::from_str(
                    "0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
                )
                .unwrap()
            && self.r > H256::zero()
            && self.s
                < H256::from_str(
                    "0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
                )
                .unwrap()
            && self.s > H256::zero()
    }

    pub fn chain_id(&self) -> Option<u64> {
        if self.v > 36 {
            Some((self.v - 35) / 2)
        } else {
            None
        }
    }

    pub fn to_recoverable_signature(&self) -> Result<RecoverableSignature, secp256k1::Error> {
        let mut sig = [0u8; 64];
        sig[0..32].copy_from_slice(self.r.as_bytes());
        sig[32..64].copy_from_slice(self.s.as_bytes());

        RecoverableSignature::from_compact(&sig, RecoveryId::from_i32(self.standard_v() as i32)?)
    }
}

impl Encodable for TransactionAction {
    fn rlp_append(&self, s: &mut RlpStream) {
        match self {
            TransactionAction::Call(address) => {
                s.append_internal(address);
            }
            TransactionAction::Create => {
                s.append_internal(&"");
            }
        }
    }
}

impl Decodable for TransactionAction {
    fn decode(rlp: &Rlp<'_>) -> Result<Self, DecoderError> {
        Ok(if rlp.is_empty() {
            if rlp.is_data() {
                TransactionAction::Create
            } else {
                return Err(rlp::DecoderError::RlpExpectedToBeData);
            }
        } else {
            TransactionAction::Call(rlp.as_val()?)
        })
    }
}

impl Encodable for Transaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(9);
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas_limit);
        s.append(&self.action);
        s.append(&self.value);
        s.append(&self.input);
        s.append(&self.signature.v);
        s.append(&self.signature.r);
        s.append(&self.signature.s);
    }
}

impl Decodable for Transaction {
    fn decode(rlp: &Rlp<'_>) -> Result<Self, DecoderError> {
        Ok(Self {
            nonce: rlp.val_at(0)?,
            gas_price: rlp.val_at(1)?,
            gas_limit: rlp.val_at(2)?,
            action: rlp.val_at(3)?,
            value: rlp.val_at(4)?,
            input: rlp.val_at(5)?,
            signature: TransactionSignature {
                v: rlp.val_at(6)?,
                r: rlp.val_at(7)?,
                s: rlp.val_at(8)?,
            },
        })
    }
}

impl From<Transaction> for UnsignedTransaction {
    fn from(val: Transaction) -> UnsignedTransaction {
        UnsignedTransaction {
            nonce: val.nonce,
            gas_price: val.gas_price,
            gas_limit: val.gas_limit,
            action: val.action,
            value: val.value,
            input: val.input,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransactionInReceipt {
    Signed(Transaction),
    Unsigned(UnsignedTransactionWithCaller),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnsignedTransactionWithCaller {
    pub unsigned_tx: UnsignedTransaction,
    pub caller: H160,
    pub chain_id: Option<u64>,
}

// TODO: Work on logs and state_root.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub transaction: TransactionInReceipt,
    pub status: evm::ExitReason,
    pub block_number: u64,
    pub index: u64,
    // pub state_root: H256, // State root not needed in newer evm versions
    pub used_gas: u64,
    // pub logs_bloom: LogsBloom,
    pub logs: Vec<Log>,
}

impl TransactionReceipt {
    pub fn new(
        transaction: TransactionInReceipt,
        used_gas: u64,
        block_number: u64,
        index: u64,
        logs: Vec<Log>,
        result: (evm::ExitReason, Vec<u8>),
    ) -> TransactionReceipt {
        TransactionReceipt {
            status: result.0,
            transaction,
            used_gas,
            block_number,
            index,
            logs,
        }
    }
}

pub fn addr_from_public_key(key: &PublicKey) -> H160 {
    let digest = Keccak256::digest(&key.serialize_uncompressed()[1..]);

    let hash = H256::from_slice(digest.as_slice());
    H160::from(hash)
}

#[cfg(test)]
mod test {
    use super::*;
    use secp256k1::{PublicKey, SecretKey, SECP256K1};

    #[test]
    fn test_valid_addr() {
        let addr = H160::from_str("9Edb9E0B88Dbf2a29aE121a657e1860aEceaA53D").unwrap();
        let secret_key =
            SecretKey::from_str("fb507dc8bc8ea30aa275702108e6a22f66096e274a1c4c36e709b12a13dd0e76")
                .unwrap();
        let public_key = PublicKey::from_secret_key(SECP256K1, &secret_key);
        println!("public = {}", public_key);
        let addr2 = addr_from_public_key(&public_key);
        assert_eq!(addr, addr2)
    }

    #[test]
    fn sign_check_signature() {
        let addr = H160::from_str("9Edb9E0B88Dbf2a29aE121a657e1860aEceaA53D").unwrap();
        let secret_key =
            SecretKey::from_str("fb507dc8bc8ea30aa275702108e6a22f66096e274a1c4c36e709b12a13dd0e76")
                .unwrap();

        let tx = UnsignedTransaction {
            nonce: U256::from(1),
            gas_price: U256::from(2),
            gas_limit: U256::from(3),
            action: TransactionAction::Create,
            value: U256::from(4),
            input: vec![2; 3],
        };

        let chain_id = 0x77;

        let mut stream = RlpStream::new();
        tx.signing_rlp_append(&mut stream, Some(chain_id));
        println!("rlp = {}", hex::encode(stream.out()));
        println!("hash = {:x}", tx.signing_hash(Some(chain_id)));

        let tx = tx.sign(&secret_key, Some(chain_id));
        assert_eq!(tx.signature.chain_id(), Some(chain_id));
        assert_eq!(tx.caller().unwrap(), addr);
    }

    #[test]
    fn should_agree_with_vitalik() {
        let test_vector = |tx_data: &str, address: &'static str| {
            let signed: Transaction =
                rlp::decode(&hex::decode(tx_data).unwrap()).expect("decoding tx data failed");
            assert_eq!(
                signed.caller().unwrap(),
                Address::from_str(&address[2..]).unwrap()
            );
            println!("chainid: {:?}", signed.signature.chain_id());
        };

        test_vector("f864808504a817c800825208943535353535353535353535353535353535353535808025a0044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116da0044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116d", "0xf0f6f18bca1b28cd68e4357452947e021241e9ce");
        test_vector("f864018504a817c80182a410943535353535353535353535353535353535353535018025a0489efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bcaa0489efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc6", "0x23ef145a395ea3fa3deb533b8a9e1b4c6c25d112");
        test_vector("f864028504a817c80282f618943535353535353535353535353535353535353535088025a02d7c5bef027816a800da1736444fb58a807ef4c9603b7848673f7e3a68eb14a5a02d7c5bef027816a800da1736444fb58a807ef4c9603b7848673f7e3a68eb14a5", "0x2e485e0c23b4c3c542628a5f672eeab0ad4888be");
        test_vector("f865038504a817c803830148209435353535353535353535353535353535353535351b8025a02a80e1ef1d7842f27f2e6be0972bb708b9a135c38860dbe73c27c3486c34f4e0a02a80e1ef1d7842f27f2e6be0972bb708b9a135c38860dbe73c27c3486c34f4de", "0x82a88539669a3fd524d669e858935de5e5410cf0");
        test_vector("f865048504a817c80483019a28943535353535353535353535353535353535353535408025a013600b294191fc92924bb3ce4b969c1e7e2bab8f4c93c3fc6d0a51733df3c063a013600b294191fc92924bb3ce4b969c1e7e2bab8f4c93c3fc6d0a51733df3c060", "0xf9358f2538fd5ccfeb848b64a96b743fcc930554");
        test_vector("f865058504a817c8058301ec309435353535353535353535353535353535353535357d8025a04eebf77a833b30520287ddd9478ff51abbdffa30aa90a8d655dba0e8a79ce0c1a04eebf77a833b30520287ddd9478ff51abbdffa30aa90a8d655dba0e8a79ce0c1", "0xa8f7aba377317440bc5b26198a363ad22af1f3a4");
        test_vector("f866068504a817c80683023e3894353535353535353535353535353535353535353581d88025a06455bf8ea6e7463a1046a0b52804526e119b4bf5136279614e0b1e8e296a4e2fa06455bf8ea6e7463a1046a0b52804526e119b4bf5136279614e0b1e8e296a4e2d", "0xf1f571dc362a0e5b2696b8e775f8491d3e50de35");
        test_vector("f867078504a817c807830290409435353535353535353535353535353535353535358201578025a052f1a9b320cab38e5da8a8f97989383aab0a49165fc91c737310e4f7e9821021a052f1a9b320cab38e5da8a8f97989383aab0a49165fc91c737310e4f7e9821021", "0xd37922162ab7cea97c97a87551ed02c9a38b7332");
        test_vector("f867088504a817c8088302e2489435353535353535353535353535353535353535358202008025a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c12a064b1702d9298fee62dfeccc57d322a463ad55ca201256d01f62b45b2e1c21c10", "0x9bddad43f934d313c2b79ca28a432dd2b7281029");
        test_vector("f867098504a817c809830334509435353535353535353535353535353535353535358202d98025a052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afba052f8f61201b2b11a78d6e866abc9c3db2ae8631fa656bfe5cb53668255367afb", "0x3c24d7329e92f84f08556ceb6df1cdb0104ca49f");
    }

    #[test]
    fn should_recover_from_chain_specific_signing() {
        let mut rng = secp256k1::rand::thread_rng();
        let key = SecretKey::new(&mut rng);
        let t = UnsignedTransaction {
            action: TransactionAction::Create,
            nonce: U256::from(42),
            gas_price: U256::from(3000),
            gas_limit: U256::from(50_000),
            value: U256::from(1),
            input: b"Hello!".to_vec(),
        }
        .sign(&key, Some(69));
        let public_key = PublicKey::from_secret_key(SECP256K1, &key);
        assert_eq!(addr_from_public_key(&public_key), t.caller().unwrap());
        assert_eq!(t.signature.chain_id(), Some(69));
    }
}
