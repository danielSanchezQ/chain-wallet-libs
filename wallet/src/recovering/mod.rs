//! module for all the recovering mechanism around the cardano blockchains

mod daedalus;
mod dump;
mod icarus;

use crate::{keygen, Password, Wallet};
use chain_path_derivation::{
    rindex::{self, Rindex},
    AnyScheme,
};
use cryptoxide::digest::Digest;
use ed25519_bip32::{self, DerivationScheme, XPrv, XPRV_SIZE};
use hdkeygen::Key;
use thiserror::Error;

pub use self::{daedalus::RecoveringDaedalus, dump::Dump, icarus::RecoveringIcarus};

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error("The wallet scheme does not require a password")]
    SchemeDoesNotRequirePassword,

    #[error("Missing entropy, either missing the mnemonics or need to generate a new wallet")]
    MissingEntropy,
}

#[derive(Default)]
pub struct RecoveryBuilder {
    entropy: Option<bip39::Entropy>,
    password: Option<Password>,
}

impl RecoveryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mnemonics<D>(self, dic: &D, mnemonics: impl AsRef<str>) -> Result<Self, bip39::Error>
    where
        D: bip39::dictionary::Language,
    {
        let mnemonics = bip39::Mnemonics::from_string(dic, mnemonics.as_ref())?;
        let entropy = bip39::Entropy::from_mnemonics(&mnemonics)?;

        Ok(Self {
            entropy: Some(entropy),
            ..self
        })
    }

    pub fn entropy(self, entropy: bip39::Entropy) -> Self {
        Self {
            entropy: Some(entropy),
            ..self
        }
    }

    pub fn password(self, password: Password) -> Self {
        Self {
            password: Some(password),
            ..self
        }
    }

    pub fn build_daedalus(&self) -> Result<RecoveringDaedalus, RecoveryError> {
        if self.password.is_some() {
            return Err(RecoveryError::SchemeDoesNotRequirePassword);
        }

        let entropy = self.entropy.clone().ok_or(RecoveryError::MissingEntropy)?;

        let key = from_daedalus_entropy(entropy, ed25519_bip32::DerivationScheme::V1)
            .expect("Cannot fail to serialize some bytes...");

        let wallet = hdkeygen::rindex::Wallet::from_root_key(key);

        Ok(RecoveringDaedalus::new(wallet))
    }

    pub fn build_yoroi(&self) -> Result<RecoveringIcarus, RecoveryError> {
        let entropy = self.entropy.clone().ok_or(RecoveryError::MissingEntropy)?;
        let password = self.password.clone().unwrap_or_default();

        let key = from_bip39_entropy(entropy, password, ed25519_bip32::DerivationScheme::V2);

        let root = hdkeygen::bip44::Root::from_root_key(key.coerce_unchecked());

        let wallet = hdkeygen::bip44::Wallet::new(root);

        Ok(RecoveringIcarus::new(wallet))
    }

    pub fn build_wallet(&self) -> Result<Wallet, RecoveryError> {
        let entropy = self.entropy.clone().ok_or(RecoveryError::MissingEntropy)?;
        let password = self.password.clone().unwrap_or_default();

        let mut seed = [0u8; hdkeygen::account::SEED_LENGTH];
        keygen::generate_seed(&entropy, password.as_ref(), &mut seed);

        let account = hdkeygen::account::Account::from_seed(seed);

        Ok(Wallet { account })
    }
}

/// Compatibility with daedalus mnemonic addresses
///
/// > 2 Level of randomly chosen hard derivation indexes wallets
/// > uses the bip39 mnemonics but do not follow
/// > the whole BIP39 specifications.
///
/// # considerations
///
/// It is recommended to avoid using it as this is a weak
/// cryptographic scheme:
///
/// 1. it does not allow for mnemonic passwords (no plausible deniability);
/// 2. the use of an invariant of the cryptographic scheme makes it less
///
/// # internals
///
/// 1. the mnemonic words are used to retrieve the entropy;
/// 2. the entropy is serialized in CBOR;
/// 3. the cbor serialised entropy is then hashed with Blake2b 256;
/// 4. the blake2b digest is serialised in cbor;
/// 5. the cbor serialised digest is then fed into HMAC sha256
///
/// There are many things that can go wrong when implementing this
/// process, it is all done correctly by this function: prefer using
/// this function.
fn from_daedalus_entropy(
    entropy: bip39::Entropy,
    derivation_scheme: DerivationScheme,
) -> Result<Key<XPrv, Rindex<rindex::Root>>, cbor_event::Error> {
    let entropy_bytes = cbor_event::Value::Bytes(Vec::from(entropy.as_ref()));
    let entropy_cbor = cbor_event::cbor!(&entropy_bytes)?;
    let seed: Vec<u8> = {
        let mut blake2b = cryptoxide::blake2b::Blake2b::new(32);
        blake2b.input(&entropy_cbor);
        let mut out = [0; 32];
        blake2b.result(&mut out);
        let mut se = cbor_event::se::Serializer::new_vec();
        se.write_bytes(&Vec::from(&out[..]))?;
        se.finalize()
    };

    let xprv = generate_from_daedalus_seed(&seed);
    let key = Key::new_unchecked(xprv, rindex::new(), derivation_scheme);
    Ok(key)
}

/// method to recover the private key from bip39 mnemonics
///
/// this is the method used in yoroiwallet.com
fn from_bip39_entropy(
    entropy: bip39::Entropy,
    password: impl AsRef<[u8]>,
    derivation_scheme: DerivationScheme,
) -> Key<XPrv, AnyScheme> {
    let mut seed = [0u8; XPRV_SIZE];
    keygen::generate_seed(&entropy, password.as_ref(), &mut seed);
    let xprv = XPrv::normalize_bytes_force3rd(seed);

    let key = Key::new_unchecked(xprv, Default::default(), derivation_scheme);
    key
}

/// for some unknown design reasons Daedalus seeds are encoded in cbor
/// We then expect the input here to be cbor encoded before hand.
///
fn generate_from_daedalus_seed(bytes: &[u8]) -> XPrv {
    use cryptoxide::{hmac::Hmac, mac::Mac, sha2::Sha512};

    let mut mac = Hmac::new(Sha512::new(), bytes);

    let mut iter = 1;

    loop {
        let s = format!("Root Seed Chain {}", iter);
        mac.reset();
        mac.input(s.as_bytes());
        let mut block = [0u8; 64];
        mac.raw_result(&mut block);

        let mut sk = [0; 32];
        sk.clone_from_slice(&block.as_ref()[0..32]);
        let mut cc = [0; 32];
        cc.clone_from_slice(&block.as_ref()[32..64]);
        let xprv = XPrv::from_nonextended_force(&sk, &cc);

        // check if we find a good candidate
        if xprv.as_ref()[31] & 0x20 == 0 {
            return xprv;
        }

        iter += 1;
    }
}
