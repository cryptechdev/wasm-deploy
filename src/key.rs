use std::fmt::Display;

use clap::{Args, Subcommand};
use cosm_orc::client::error::ClientError;
use cosmrs::{bip32, crypto::secp256k1, AccountId};
use keyring::Entry;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// https://github.com/confio/cosmos-hd-key-derivation-spec#the-cosmos-hub-path
const DERVIATION_PATH: &str = "m/44'/118'/0'/0/0";

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq)]
pub struct UserKey {
    /// human readable key name
    pub name: String,
    /// private key associated with `name`
    pub key:  Key,
}

impl UserKey {
    pub fn to_account(&self, prefix: &str) -> Result<AccountId, ClientError> {
        let key: secp256k1::SigningKey = self.try_into()?;
        let account = key.public_key().account_id(prefix).map_err(ClientError::crypto)?;
        Ok(account)
    }
}

impl Display for UserKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.name.fmt(f) }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Key {
    /// Mnemonic allows you to pass the private key mnemonic words
    /// to Cosm-orc for configuring a transaction signing key.
    /// DO NOT USE FOR MAINNET
    Mnemonic { phrase: String },

    // TODO: Add Keyring password CRUD operations
    /// Use OS Keyring to access private key.
    /// Safe for testnet / mainnet.
    Keyring {
        #[command(flatten)]
        params: KeyringParams,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Args, Debug, Clone, PartialEq, Eq)]
pub struct KeyringParams {
    pub service:   String,
    pub user_name: String,
}

impl TryFrom<&UserKey> for secp256k1::SigningKey {
    type Error = ClientError;

    fn try_from(signer: &UserKey) -> Result<secp256k1::SigningKey, ClientError> {
        match &signer.key {
            Key::Mnemonic { phrase } => mnemonic_to_signing_key(phrase),
            Key::Keyring { params } => {
                let entry = Entry::new(&params.service, &params.user_name);
                mnemonic_to_signing_key(&entry.get_password()?)
            }
        }
    }
}

fn mnemonic_to_signing_key(mnemonic: &str) -> Result<secp256k1::SigningKey, ClientError> {
    let seed = bip32::Mnemonic::new(mnemonic, bip32::Language::English)
        .map_err(|_| ClientError::Mnemonic)?
        .to_seed("");
    Ok(bip32::XPrv::derive_from_path(seed, &DERVIATION_PATH.parse().unwrap())
        .map_err(|_| ClientError::DerviationPath)?
        .into())
}
