#[cfg(feature = "ledger")]
use std::rc::Rc;
use std::{fmt::Display, str::FromStr};

use clap::Args;
use cosmos_sdk_proto::cosmos::auth::v1beta1::BaseAccount;
use cosmrs::{
    bip32::{self},
    crypto::{secp256k1, PublicKey as OtherPublicKey},
    tendermint::PublicKey,
    tx::{self, Fee, Msg, Raw, SignDoc, SignerInfo},
    AccountId, Coin, Denom,
};
use keyring::Entry;
#[cfg(feature = "ledger")]
use ledger_cosmos_secp256k1::CosmosApp;
#[cfg(feature = "ledger")]
use ledger_utility::Connection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumVariantNames};

#[cfg(feature = "ledger")]
use crate::ledger::LedgerInfo;
use crate::{error::DeployError, file::ChainInfo};

// https://github.com/confio/cosmos-hd-key-derivation-spec#the-cosmos-hub-path
//const DERIVATION_PATH: &str = "m/44'/118'/0'/0/0";

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct UserKey {
    /// human readable key name
    pub name: String,
    /// private key associated with `name`
    pub key:  Key,
}

impl UserKey {
    pub async fn to_account(&self, derivation_path: &str, prefix: &str) -> Result<AccountId, DeployError> {
        let public_key: OtherPublicKey = self.public_key(derivation_path).await?.into();
        let account = public_key.account_id(prefix)?;
        Ok(account)
    }
}

impl Display for UserKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.name.fmt(f) }
}

#[derive(Serialize, Deserialize, Display, EnumVariantNames, Debug, Clone)]
pub enum Key {
    /// Mnemonic allows you to pass the private key mnemonic words
    /// to Cosm-orc for configuring a transaction signing key.
    /// DO NOT USE FOR MAINNET
    Mnemonic { phrase: String },

    // TODO: Add Keyring password CRUD operations
    /// Use OS Keyring to access private key.
    /// Safe for testnet / mainnet.
    Keyring { params: KeyringParams },

    /// Use a ledger hardware wallet to sign txs
    #[cfg(feature = "ledger")]
    Ledger {
        info:       LedgerInfo,
        #[serde(skip)]
        // #[serde(default = "Connection::new")]
        connection: Option<Rc<Connection>>,
    },
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Key::Mnemonic { phrase: p1 }, Key::Mnemonic { phrase: p2 }) => p1 == p2,
            (Key::Keyring { params: p1 }, Key::Keyring { params: p2 }) => p1 == p2,
            #[cfg(feature = "ledger")]
            (Key::Ledger { info: i1, .. }, Key::Ledger { info: i2, .. }) => i1 == i2,
            _ => false,
        }
    }
}

impl Eq for Key {
    fn assert_receiver_is_total_eq(&self) {}
}

#[derive(Serialize, Deserialize, JsonSchema, Args, Debug, Clone, PartialEq, Eq)]
pub struct KeyringParams {
    pub service:   String,
    pub user_name: String,
}

impl UserKey {
    pub async fn public_key(&self, derivation_path: &str) -> Result<PublicKey, DeployError> {
        match &self.key {
            Key::Mnemonic { phrase } => Ok(mnemonic_to_signing_key(derivation_path, phrase)?.public_key().into()),
            Key::Keyring { params } => {
                let entry = Entry::new(&params.service, &params.user_name);
                Ok(mnemonic_to_signing_key(derivation_path, &entry.get_password()?)?.public_key().into())
            }
            #[cfg(feature = "ledger")]
            Key::Ledger { info, connection } => {
                println!("Retrieving public key from ledger");
                match connection {
                    Some(connection) => {
                        println!("Connecting to {}", info.device_name);
                        let ledger = connection.connect_with_name(info.device_name.clone(), 5).await.unwrap();
                        let app = CosmosApp::new(ledger);
                        let path = [44, 118, 0, 0, 0];
                        let hrp = "cosmos";
                        let display_on_ledger = false;
                        let res = app.get_addr_secp256k1(path, hrp, display_on_ledger).await.unwrap();
                        println!("Address: {}", res.addr);
                        Ok(res.public_key)
                    }
                    None => panic!("missing connection"),
                }
            }
        }
    }

    pub async fn sign(
        &self, account: &BaseAccount, fee: Option<Fee>, chain_info: &ChainInfo, memo: String, msgs: Vec<impl Msg>,
    ) -> Result<Raw, DeployError> {
        let timeout_height = 0u16;
        let anys = msgs.iter().map(|msg| msg.to_any()).collect::<Result<Vec<_>, _>>().unwrap();
        let tx_body = tx::Body::new(anys, memo, timeout_height);
        let public_key = self.public_key(&chain_info.derivation_path).await?.into();
        let fee = fee.unwrap_or(Fee::from_amount_and_gas(
            Coin { denom: Denom::from_str(&chain_info.denom).unwrap(), amount: 0u64.into() },
            0u64,
        ));
        let auth_info = SignerInfo::single_direct(Some(public_key), account.sequence).auth_info(fee);
        let sign_doc = SignDoc::new(
            &tx_body,
            &auth_info,
            &chain_info.chain_id.clone().try_into().unwrap(),
            account.account_number,
        )?;
        match &self.key {
            Key::Mnemonic { phrase } => {
                let signing_key = mnemonic_to_signing_key(&chain_info.derivation_path, phrase)?;
                Ok(sign_doc.sign(&signing_key)?)
            }
            Key::Keyring { params } => {
                let entry = Entry::new(&params.service, &params.user_name);
                let signing_key = mnemonic_to_signing_key(&chain_info.derivation_path, &entry.get_password()?)?;
                Ok(sign_doc.sign(&signing_key)?)
            }
            #[cfg(feature = "ledger")]
            Key::Ledger { info, connection } => match connection {
                // Some(connection) => {
                //     println!("Signing message with ledger");
                //     println!("Connecting to {}", info.device_name);
                //     let ledger = connection.connect_with_name(info.device_name.clone(), 5).await.unwrap();
                //     let cosmos_app = CosmosApp::new(ledger);
                //     let path = [44, 118, 0, 0, 0];
                //     let serialized = sign_doc.into_bytes().unwrap();
                //     println!("serialized: {:?}", serialized);
                //     let signed = cosmos_app
                //         .sign(derivation_path, fee, chain_id, memo, account_number, sequence, serialized)
                //         .await
                //         .unwrap();
                //     println!("signed: {:?}", signed);
                //     Ok(signed.clone())
                // }
                // None => panic!("missing connection"),
                _ => todo!(),
            },
        }
    }

    // pub async fn sign(&self, derivation_path: &str, sign_doc: SignDoc) -> Result<Raw, DeployError> {
    //     match &self.key {
    //         Key::Mnemonic { phrase } => {
    //             let signing_key = mnemonic_to_signing_key(derivation_path, phrase)?;
    //             Ok(sign_doc.sign(&signing_key).map_err(DeployError::crypto)?)
    //         }
    //         Key::Keyring { params } => {
    //             let entry = Entry::new(&params.service, &params.user_name);
    //             let signing_key = mnemonic_to_signing_key(derivation_path, &entry.get_password()?)?;
    //             Ok(sign_doc.sign(&signing_key).map_err(DeployError::crypto)?)
    //         }
    //         #[cfg(feature = "ledger")]
    //         Key::Ledger { info, connection } => match connection {
    //             Some(connection) => {
    //                 println!("Signing message with ledger");
    //                 println!("Connecting to {}", info.device_name);
    //                 let ledger = connection.connect_with_name(info.device_name.clone(),
    // 5).await.unwrap();                 let path = [44, 118, 0, 0, 0];
    //                 let serialized = sign_doc.into_bytes().unwrap();
    //                 println!("serialized: {:?}", serialized);
    //                 let signed = ledger.sign_secp256k1(path, vec![serialized]).await.unwrap();
    //                 println!("signed: {:?}", signed);
    //                 Ok(signed[0].clone())
    //             }
    //             None => panic!("missing connection"),
    //         },
    //     }
    // }
}

fn mnemonic_to_signing_key(derivation_path: &str, mnemonic: &str) -> Result<secp256k1::SigningKey, DeployError> {
    let seed = bip32::Mnemonic::new(mnemonic, bip32::Language::English)
        .map_err(|_| DeployError::Mnemonic)?
        .to_seed("");
    Ok(bip32::XPrv::derive_from_path(seed, &derivation_path.parse().unwrap())
        .map_err(|_| DeployError::DerviationPath)?
        .into())
}
