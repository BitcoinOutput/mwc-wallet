// Copyright 2019 The vault713 Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::bitcoin::BtcUpdate;
use super::multisig::ParticipantData as MultisigParticipant;
use super::ser::*;
use super::types::{Currency, Network};
use super::ErrorKind;
use crate::{ParticipantData as TxParticipant, VersionedSlate};
use chrono::{DateTime, Utc};
use grin_core::libtx::secp_ser;
use grin_util::secp::key::PublicKey;
use grin_util::secp::Signature;
use uuid::Uuid;

/// Swap message that is used for Seller/Buyer interaction
#[derive(Serialize, Deserialize)]
pub struct Message {
	/// Swap session UUID
	pub id: Uuid,
	/// Swap core data
	pub inner: Update,
	/// Secondary currency (BTC) related data
	inner_secondary: SecondaryUpdate,
}

impl Message {
	/// Create message form it's components
	pub fn new(id: Uuid, inner: Update, inner_secondary: SecondaryUpdate) -> Self {
		Self {
			id,
			inner,
			inner_secondary,
		}
	}

	/// Init BTC related messgae
	pub fn set_inner_secondary(&mut self, inner_secondary: SecondaryUpdate) {
		self.inner_secondary = inner_secondary;
	}

	/// Unwrap message as Offer
	pub fn unwrap_offer(self) -> Result<(Uuid, OfferUpdate, SecondaryUpdate), ErrorKind> {
		match self.inner {
			Update::Offer(u) => Ok((self.id, u, self.inner_secondary)),
			_ => Err(ErrorKind::UnexpectedMessageType(format!(
				"Fn unwrap_offer() expecting Update::Offer, get {:?}",
				self.inner
			))),
		}
	}

	/// Unwrap message as Accepted Offer
	pub fn unwrap_accept_offer(
		self,
	) -> Result<(Uuid, AcceptOfferUpdate, SecondaryUpdate), ErrorKind> {
		match self.inner {
			Update::AcceptOffer(u) => Ok((self.id, u, self.inner_secondary)),
			_ => Err(ErrorKind::UnexpectedMessageType(format!(
				"Fn unwrap_accept_offer() expecting Update::AcceptOffer, get {:?}",
				self.inner
			))),
		}
	}

	/// Unwrap message as Init Redeem
	pub fn unwrap_init_redeem(
		self,
	) -> Result<(Uuid, InitRedeemUpdate, SecondaryUpdate), ErrorKind> {
		match self.inner {
			Update::InitRedeem(u) => Ok((self.id, u, self.inner_secondary)),
			_ => Err(ErrorKind::UnexpectedMessageType(format!(
				"Fn unwrap_init_redeem() expecting Update::InitRedeem, get {:?}",
				self.inner
			))),
		}
	}

	/// Unwrap message as Redeem
	pub fn unwrap_redeem(self) -> Result<(Uuid, RedeemUpdate, SecondaryUpdate), ErrorKind> {
		match self.inner {
			Update::Redeem(u) => Ok((self.id, u, self.inner_secondary)),
			_ => Err(ErrorKind::UnexpectedMessageType(format!(
				"Fn unwrap_redeem() expecting Update::Redeem, get {:?}",
				self.inner
			))),
		}
	}

	/// Message to Json String
	pub fn to_json(&self) -> Result<String, ErrorKind> {
		let str = serde_json::to_string(&self)
			.map_err(|e| ErrorKind::Serde(format!("Unable to serialize a message, {}", e)))?;
		Ok(str)
	}

	/// Build message from Json
	pub fn from_json(s: &str) -> Result<Message, ErrorKind> {
		Ok(serde_json::from_str(s).map_err(|e| {
			ErrorKind::Serde(format!("Unable to parse Swap Message from {}, {}", s, e))
		})?)
	}
}

/// Swap core data of the Seller/Buyer message
#[derive(Serialize, Deserialize, Debug)]
pub enum Update {
	/// Empty data placeholder
	None,
	/// Seller to Buyer, Status::Created  Seller creates initial offer
	Offer(OfferUpdate),
	/// Buyer sending back accepted offer
	AcceptOffer(AcceptOfferUpdate),
	/// Buyer to Seller, Status::Locked,  start working on Reedem slate
	InitRedeem(InitRedeemUpdate),
	/// Seller to Buyer, Status::InitRedeem, working on Reedem slate
	Redeem(RedeemUpdate),
}

/// Seller, Status::Created  Seller creates initial offer
#[derive(Serialize, Deserialize, Debug)]
pub struct OfferUpdate {
	/// Swap starting time.
	pub start_time: DateTime<Utc>,
	/// Version of the swap engine. Both party must match
	pub version: u8,
	/// The type of the network. Floonet or mainnet
	pub network: Network,
	/// Number of MWC to offer
	#[serde(with = "secp_ser::string_or_u64")]
	pub primary_amount: u64,
	/// Number of BTC to get
	#[serde(with = "secp_ser::string_or_u64")]
	pub secondary_amount: u64,
	/// BTC
	pub secondary_currency: Currency,
	/// Seller part of multisig
	pub multisig: MultisigParticipant,
	/// Lock Slate that Buyer need to continue to build
	pub lock_slate: VersionedSlate,
	/// Refund slate that byer need to sign.
	pub refund_slate: VersionedSlate,
	/// Needed info to build step 1 on redeem state (that saving some interaction)
	pub redeem_participant: TxParticipant,
	/// Requred confirmations for MWC Locking
	pub required_mwc_lock_confirmations: u64,
	/// Requred confirmations for BTC Locking
	pub required_secondary_lock_confirmations: u64,
	/// Seller MWC funds lock time interval in seconds
	pub mwc_lock_time_seconds: u64,
	/// Sellet BTC redeem time interval. BTC expected to be locked for that time as well.
	pub seller_redeem_time: u64,
}

/// Buyer, Status::Offered  Buyer responded for offer
#[derive(Serialize, Deserialize, Debug)]
pub struct AcceptOfferUpdate {
	/// Buyer part of multisig
	pub multisig: MultisigParticipant,
	/// Public key for Redeem Slate
	#[serde(serialize_with = "pubkey_to_hex", deserialize_with = "pubkey_from_hex")]
	pub redeem_public: PublicKey,
	/// Buyer part needed to build lock slate
	pub lock_participant: TxParticipant,
	/// Buyer part needed to build refund slate
	pub refund_participant: TxParticipant,
}

/// Buyer, Status::Locked   Buyer building the redeem slate
#[derive(Serialize, Deserialize, Debug)]
pub struct InitRedeemUpdate {
	/// redeem slate, construction in the progress
	pub redeem_slate: VersionedSlate,
	/// signature for redeem_slate, see calculate_adaptor_signature  how we build it
	#[serde(serialize_with = "sig_to_hex", deserialize_with = "sig_from_hex")]
	pub adaptor_signature: Signature,
}

/// Seller, Status::InitRedeem.  Sending it's part needed for redeem transaction
#[derive(Serialize, Deserialize, Debug)]
pub struct RedeemUpdate {
	/// Needed data to build redeem transaction
	pub redeem_participant: TxParticipant,
}

/// Update message about Secondary Currency (BTC)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SecondaryUpdate {
	/// None, empty value
	Empty,
	/// BTC upadte
	BTC(BtcUpdate),
}

impl SecondaryUpdate {
	/// Helper to extract BtcUpdate with type validation
	pub fn unwrap_btc(self) -> Result<BtcUpdate, ErrorKind> {
		match self {
			SecondaryUpdate::BTC(d) => Ok(d),
			_ => Err(ErrorKind::UnexpectedCoinType),
		}
	}
}
