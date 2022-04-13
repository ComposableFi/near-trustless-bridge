use std::collections::HashMap;

use crate::{
    signature::SignatureVerification,
    types::{ApprovalInner, CryptoHash, LightClientBlockView, ValidatorStakeView},
};

use borsh::BorshSerialize;

#[cfg(test)]
use sha2::{Digest as DigestTrait, Sha256};

pub trait BlockValidation {
    type Digest: Digest;

    fn validate_light_block(
        &self,
        head: &LightClientBlockView,
        block_view: &LightClientBlockView,
        epoch_block_producers: &HashMap<CryptoHash, ValidatorStakeView>,
    ) -> bool {
        //The light client updates its head with the information from LightClientBlockView iff:

        // 1. The height of the block is higher than the height of the current head;
        // 2. The epoch of the block is equal to the epoch_id or next_epoch_id known for the current head;
        // 3. If the epoch of the block is equal to the next_epoch_id of the head, then next_bps is not None;
        // 4. approvals_after_next contain valid signatures on approval_message from the block producers of the corresponding
        // epoch
        // 5. The signatures present in approvals_after_next correspond to more than 2/3 of the total stake (see next section).
        // 6. If next_bps is not none, sha256(borsh(next_bps)) corresponds to the next_bp_hash in inner_lite.

        // QUESTION: do we also want to pass the block hash received from the RPC?
        // it's not on the spec, but it's an extra validation
        let (_current_block_hash, _next_block_hash, approval_message) =
            reconstruct_light_client_block_view_fields::<Self::Digest>(block_view);

        // (1)
        if block_view.inner_lite.height <= head.inner_lite.height {
            return false;
        }

        // (2)
        if ![head.inner_lite.epoch_id, head.inner_lite.next_epoch_id]
            .contains(&block_view.inner_lite.epoch_id)
        {
            return false;
        }

        // (3)
        if block_view.inner_lite.epoch_id == head.inner_lite.next_epoch_id
            && block_view.next_bps.is_none()
        {
            return false;
        }

        //  (4) and (5)
        let mut total_stake = 0;
        let mut approved_stake = 0;

        for (maybe_signature, block_producer) in block_view
            .approvals_after_next
            .iter()
            .zip(epoch_block_producers.iter())
        {
            let validator_stake = block_producer.1;
            let bp_stake = validator_stake.stake;
            total_stake += bp_stake;

            if maybe_signature.is_none() {
                continue;
            }

            approved_stake += bp_stake;

            let validator_public_key: [u8; 32] = validator_stake.public_key.try_into().unwrap();
            if !maybe_signature
                .unwrap()
                .verify(&approval_message, vec![validator_public_key])
            {
                return false;
            }
        }

        let threshold = total_stake * 2 / 3;
        if approved_stake <= threshold {
            return false;
        }

        // # (6)
        let block_view_next_bps_serialized = block_view
            .next_bps
            .as_deref()
            .unwrap()
            .try_to_vec()
            .unwrap();
        if block_view.next_bps.is_some() {
            if Self::Digest::digest(block_view_next_bps_serialized).as_slice()
                != block_view.inner_lite.next_bp_hash.as_ref()
            {
                return false;
            }
        }
        true
    }
}

pub fn reconstruct_light_client_block_view_fields<D: Digest>(
    block_view: &LightClientBlockView,
) -> (CryptoHash, CryptoHash, Vec<u8>) {
    let current_block_hash = block_view.current_block_hash();
    let next_block_hash = next_block_hash(block_view.next_block_inner_hash, current_block_hash);
    let approval_message = [
        ApprovalInner::Endorsement(next_block_hash)
            .try_to_vec()
            .unwrap(),
        (block_view.inner_lite.height + 2)
            .to_le()
            .try_to_vec()
            .unwrap(),
    ]
    .concat();
    (current_block_hash, next_block_hash, approval_message)
}

pub(crate) fn next_block_hash<D: Digest>(
    next_block_inner_hash: CryptoHash,
    current_block_hash: CryptoHash,
) -> CryptoHash {
    D::digest([next_block_inner_hash.as_ref(), current_block_hash.as_ref()].concat())
        .as_slice()
        .try_into()
        .unwrap()
}
pub trait Digest {
    fn digest(data: impl AsRef<[u8]>) -> Vec<u8>;
}

#[cfg(test)]
pub struct Sha256Digest;

#[cfg(test)]
impl Digest for Sha256Digest {
    fn digest(data: impl AsRef<[u8]>) -> Vec<u8> {
        Sha256::digest(data).to_vec()
    }
}

#[cfg(test)]
impl BlockValidation for Sha256Digest {
    type Digest = Sha256Digest;

    fn validate_light_block(
        &self,
        head: &LightClientBlockView,
        block_view: &LightClientBlockView,
        epoch_block_producers: &HashMap<CryptoHash, ValidatorStakeView>,
    ) -> bool {
        true
    }
}
