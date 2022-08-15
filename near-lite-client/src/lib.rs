//! # Near lite client
//!
//! The main purpose of the lite client is to keep track of a small subset
//! of the chain's state while still being able to:
//! 1. verify the chain's state transitions and keep a subset of the state
//! 2. verify that a transaction belongs to a vald block
//!
//! ## Usage
//!
//! ```ignore
//! use near_lite_client::prelude::*;
//! // call the Light Client constructuro with a `TrustedCheckpoint`
//! let mut lite_client = LightClient::with_checkpoint(trusted_checkpoint);
//!
//! // there are two operations that can be performed:
//! // `validate_and_update_head` & `validate_transaction`
//!
//! lite_client.validate_and_update_head(block_view);
//! lite_client.validate_transaction(outcome_proof, outcome_root_proof, expected_block_outcome_root);
//! ```
#![cfg_attr(not(feature = "std"), no_std)]

mod block_validation;
mod checkpoint;
mod client;
mod error;
mod merkle_tree;
mod signature;
mod storage;
mod verifier;

pub use block_validation::{SubstrateDigest};
pub use checkpoint::TrustedCheckpoint;
pub use client::LightClient;
pub use storage::StateStorage;
pub use near_primitives_wasm_friendly::{
    CryptoHash, LightClientBlockView, MerklePath, OutcomeProof, Signature, ValidatorStakeView,
};
pub use verifier::StateTransitionVerificator;

use crate::{ error::NearLiteClientError};

pub type LiteClientResult<T> = Result<T, NearLiteClientError>;

pub mod prelude {
    pub use super::{
        CryptoHash, LightClient, LightClientBlockView, MerklePath, OutcomeProof, Signature,
        StateStorage, StateTransitionVerificator, SubstrateDigest, TrustedCheckpoint,
        ValidatorStakeView,
    };
}
