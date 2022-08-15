use borsh::maybestd::{string::String};
use near_crypto::Signature;
use sp_std::vec::Vec;

use crate::{block_validation::Digest, error::NearLiteClientError};
use borsh::{BorshDeserialize, BorshSerialize};
use near_primitives::hash::{CryptoHash};
use near_crypto::PublicKey;

pub type LiteClientResult<T> = Result<T, NearLiteClientError>;
#[derive(Debug)]
pub struct ConversionError(pub String);

pub type BlockHeight = u64;
pub type AccountId = String;
pub type Balance = u128;
pub type Gas = u64;

pub type MerkleHash = CryptoHash;

#[derive(Debug, Clone, BorshDeserialize)]
pub struct MerklePath(pub Vec<MerklePathItem>);

#[derive(Debug, Clone)]
pub struct LightClientBlockLiteView {
    pub prev_block_hash: CryptoHash,
    pub inner_rest_hash: CryptoHash,
    pub inner_lite: BlockHeaderInnerLiteView,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct LightClientBlockView {
    pub prev_block_hash: CryptoHash,
    pub next_block_inner_hash: CryptoHash,
    pub inner_lite: BlockHeaderInnerLiteView,
    pub inner_rest_hash: CryptoHash,
    pub next_bps: Option<Vec<ValidatorStakeView>>,
    pub approvals_after_next: Vec<Option<Signature>>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct BlockHeaderInnerLiteView {
    pub height: BlockHeight,
    pub epoch_id: CryptoHash,
    pub next_epoch_id: CryptoHash,
    pub prev_state_root: CryptoHash,
    pub outcome_root: CryptoHash,
    pub timestamp: u64,
    pub timestamp_nanosec: u64,
    pub next_bp_hash: CryptoHash,
    pub block_merkle_root: CryptoHash,
}

/// For some reason, when calculating the hash of the current block
/// `timestamp_nanosec` is ignored
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct BlockHeaderInnerLiteViewFinal {
    pub height: BlockHeight,
    pub epoch_id: CryptoHash,
    pub next_epoch_id: CryptoHash,
    pub prev_state_root: CryptoHash,
    pub outcome_root: CryptoHash,
    pub timestamp: u64,
    pub next_bp_hash: CryptoHash,
    pub block_merkle_root: CryptoHash,
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
pub enum ApprovalInner {
    Endorsement(CryptoHash),
    Skip(BlockHeight),
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum ValidatorStakeView {
    V1(ValidatorStakeViewV1),
}
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ValidatorStakeViewV1 {
    pub account_id: AccountId,
    pub public_key: PublicKey,
    pub stake: Balance,
}

#[derive(Debug, Clone, BorshDeserialize)]
pub struct ExecutionOutcomeView {
    /// Logs from this transaction or receipt.
    pub logs: Vec<String>,
    /// Receipt IDs generated by this transaction or receipt.
    pub receipt_ids: Vec<CryptoHash>,
    /// The amount of the gas burnt by the given transaction or receipt.
    pub gas_burnt: Gas,
    /// The amount of tokens burnt corresponding to the burnt gas amount.
    /// This value doesn't always equal to the `gas_burnt` multiplied by the gas price, because
    /// the prepaid gas price might be lower than the actual gas price and it creates a deficit.
    pub tokens_burnt: u128,
    /// The id of the account on which the execution happens. For transaction this is signer_id,
    /// for receipt this is receiver_id.
    pub executor_id: AccountId,
    /// Execution status. Contains the result in case of successful execution.
    pub status: Vec<u8>, // NOTE(blas): no need to deserialize this one (in order to avoid having to define too many unnecessary structs)
}

#[derive(Debug, BorshDeserialize)]
pub struct OutcomeProof {
    pub proof: Vec<MerklePathItem>,
    pub block_hash: CryptoHash,
    pub id: CryptoHash,
    pub outcome: ExecutionOutcomeView,
}

#[cfg_attr(feature = "deepsize_feature", derive(deepsize::DeepSizeOf))]
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum Direction {
    Left,
    Right,
}

impl ValidatorStakeView {
    pub fn into_validator_stake(self) -> ValidatorStakeViewV1 {
        match self {
            Self::V1(inner) => inner,
        }
    }
}
#[cfg_attr(feature = "deepsize_feature", derive(deepsize::DeepSizeOf))]
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct MerklePathItem {
    pub hash: MerkleHash,
    pub direction: Direction,
}

impl LightClientBlockView {
    pub fn current_block_hash<D: Digest>(&self) -> CryptoHash {
        // NOTE: current block hash does not contain `timestamp_nanosec` from BlockHeaderInnerLiteView
        // hence the reason of creating a new struct (i.e: BlockHeaderInnerLiteViewFinal) to conform
        // with the struct that is actually being hashed.
        current_block_hash::<D>(
            D::digest(
                BlockHeaderInnerLiteViewFinal::from(self.inner_lite.clone())
                    .try_to_vec()
                    .unwrap(),
            )
            .as_slice()
            .try_into()
            .unwrap(),
            self.inner_rest_hash,
            self.prev_block_hash,
        )
    }
    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self {
            prev_block_hash: CryptoHash([0; 32]),
            next_block_inner_hash: CryptoHash([0; 32]),
            inner_lite: BlockHeaderInnerLiteView::new_for_test(),
            inner_rest_hash: CryptoHash([0; 32]),
            next_bps: Some(vec![]),
            approvals_after_next: vec![],
        }
    }
}

/// The hash of the block is:
/// ```ignore
/// sha256(concat(
///     sha256(concat(
///         sha256(borsh(inner_lite)),
///         sha256(borsh(inner_rest)) // we can use inner_rest_hash as well
///     )
/// ),
/// prev_hash
///))
/// ```
fn current_block_hash<D: Digest>(
    inner_lite_hash: CryptoHash,
    inner_rest_hash: CryptoHash,
    prev_block_hash: CryptoHash,
) -> CryptoHash {
    CryptoHash(
        D::digest(
            [
                D::digest([inner_lite_hash.as_ref(), inner_rest_hash.as_ref()].concat()).as_ref(),
                prev_block_hash.as_ref(),
            ]
            .concat(),
        )
        .as_slice()
        .try_into()
        .unwrap(),
    )
}

impl BlockHeaderInnerLiteView {
    #[cfg(test)]
    pub fn new_for_test() -> Self {
        Self {
            height: 1,
            epoch_id: CryptoHash([0; 32]),
            next_epoch_id: CryptoHash([0; 32]),
            prev_state_root: CryptoHash([0; 32]),
            outcome_root: CryptoHash([0; 32]),
            timestamp: 1,
            timestamp_nanosec: 0,
            next_bp_hash: CryptoHash([0; 32]),
            block_merkle_root: CryptoHash([0; 32]),
        }
    }
}

impl From<BlockHeaderInnerLiteView> for BlockHeaderInnerLiteViewFinal {
    fn from(b: BlockHeaderInnerLiteView) -> Self {
        Self {
            height: b.height,
            epoch_id: b.epoch_id,
            next_epoch_id: b.next_epoch_id,
            prev_state_root: b.prev_state_root,
            outcome_root: b.outcome_root,
            timestamp: b.timestamp,
            next_bp_hash: b.next_bp_hash,
            block_merkle_root: b.block_merkle_root,
        }
    }
}


#[cfg(test)]
mod tests {
    use std::{io};

    use super::*;

    use near_primitives::views::LightClientBlockView as NearLightClientBlockView;

    #[derive(Debug, serde::Deserialize)]
    struct ResultFromRpc {
        pub result: NearLightClientBlockView,
    }

    fn get_client_near_block_view(
        client_block_response: &str,
    ) -> io::Result<NearLightClientBlockView> {
        Ok(serde_json::from_str::<ResultFromRpc>(client_block_response)?.result)
    }

    #[test]
    fn test_ensure_deserialization_from_block_view_near_primitives() {
        const CLIENT_BLOCK_VIEW: &str = r#"
        {
            "jsonrpc": "2.0",
            "result": {
                "approvals_after_next": [
                    null,
                    "ed25519:24pvVMA2ybxuk7fsCNAxDRnby5KQbGBM61T4Am74grDRuhiPbtYWBrubeSNWTejiAwiMZZt1zvLKSR8Djr4nDfHz",
                    "ed25519:24pvVMA2ybxuk7fsCNAxDRnby5KQbGBM61T4Am74grDRuhiPbtYWBrubeSNWTejiAwiMZZt1zvLKSR8Djr4nDfHz",
                    "ed25519:24pvVMA2ybxuk7fsCNAxDRnby5KQbGBM61T4Am74grDRuhiPbtYWBrubeSNWTejiAwiMZZt1zvLKSR8Djr4nDfHz",
                    "ed25519:c78hanGiPzZ5iq9GPQET9pTh6J8pw5YgRGjtbNq35LuCzyTa5b4vdjzcAfHuRznfbTis77nF1aL6zm4CTJTesgU",
                    "ed25519:65mYbzdjVUkWCh1wL81kZu96XphPP8X5McUVo2ScSKPgNiNBd3AsyR5XbJE7MGW5GnBwaqDPK8ft3yyRa3UMJnua",
                    "ed25519:4akzNHFaa7w1LvaBTFKir9ExKStoRo44rm7YJ7XvtrigDnWmQ41EV7SyEFqcSqbDSznoxZybLQUV8ccCbia1daNT",
                    "ed25519:4AGwZcCRk5WhCEuvEk12ANyJKHwwoLAPmGjU9Vqf7Xn7pDQcXw5sY8sPt3LazU7EYVaDUnZwWJUp2cAGHXkuLLyL",
                    "ed25519:31anmHx3XEyPCnn7Mth5oppwbXoJbDQQjw3WjcDLGs8167RBE4WgCPaHn8kfHyhQ3tWWHudi1CFhy92yjJKPdLNK",
                    "ed25519:4qLsTS1cF9ahcAnddUGjY7yFx2Sd8gJpp2dU3LjRpzT9vpG3grrGDcqCxVRAgjq7tEyuKXsbL7zyxVgpXjbicVwK",
                    "ed25519:LtwgqDVPQWvomdx2zoXmfopgRgzLjxpovmjRXgetpZvc3E19iKjHbYtcs8FgGS4b6AT9GqgtoGfuLD1qdR94i2D",
                    "ed25519:4pqiJapEEMS3czyAwM7QW4qecjT5u4EQkFa79rtCmSEACnKtxuU5PGBwsbZJkq7h8xhS24vN7d5AzszuKGWviNR9",
                    "ed25519:38FxGgLRJMoD2cC3zX93c47iD51pGMvcVpPzCX9hSYfexJg8st7Ny4vr6U4sBfiyLeToqJTuoobEUzEts2eZXBxa",
                    "ed25519:2Srg7nZ29C8ySxMKkMFXzFXj5i1RL4NGQ19GQryMmHcEYSVLFGBauhjydbEtaEQ5tzpZpMFPeGLnyV88GQSUHcPA",
                    null,
                    "ed25519:2LX76ZV8iB7ZyaVAAtpUjQDwKAshix3zLk9X5kF6gsn7oqMT6Rw6Mns3HZkD9M4mmMGEiUQqETw8P36Kymb1GUjb",
                    "ed25519:2e39nRRS97kvfkjjGohtggubeTBGX8sqGSuL83nH1PYWDGoSANcUZqYeWZKxy8dzW44HEc9ptHYgGsynf6m4RY5S",
                    "ed25519:2s6yBzV5D8VFS9hDsyqwHw3QQu4mQhjq4R6VHYMVXgbbogCe11eP4xZYUtw44gZrPawV3yxeqQH2RQFngdw5fABH",
                    "ed25519:5kZ6EbdnhfwxdzwhEBarnMeNi5ng2UujvYNkafUVEeN39Bbap3WgiNz1j697WrW9Zw1HNNu4ZEGxk4ad96Z3e6rB",
                    "ed25519:3qXYatuMPnxyRKyytzSBTtRxQ38Sm42Asf9jDo5MPoNnQVhiHBdiAydZWyrKfdqgHnibVc6Xxh6yPzSQxK67xCFu",
                    null,
                    "ed25519:5AnKpS5LaHayrW8pFoCkNugAEfSvniMJEzCSq1u4NPYTrkzsrLRiQ2SANVwA3PkXJrz6hd1abQCCWNhJMPsNNHQu",
                    "ed25519:5D867Gg5xv9XiBWXMxhzx3cfY41moU5g7E62PyQLrEmvLSL5px67ojzasVd4whdqF3CzkN8wuuzGi2vvqqPNLPkr",
                    "ed25519:5Bi9FH8gmnncJJNjpcNQx3AV12VpmF6Mk3pCGVvitBUrMMAKrYUEHh3knZpGJVWCVjP4TyxfKRwvCGVH2VNbHXMF",
                    "ed25519:2J13vtY7vzxREcYQU4micZMpNskakdbvbxC5CMUnf4BSRf6my2nQ5g77GWSH4DNC9FTSW6ZQACJHfXyy9opMqLfT",
                    "ed25519:sLUzhmxwGgVRLePVBvwMrW3Ny7E2ftWVnRAntbqF4sempASFMbwhjHvcBUfmNtJUSL9Qc5gEwbDgMMrriEuK2JK",
                    "ed25519:2547xqoEPW9hR2Jh5FDgZDvuxmacMdPUdq4mqpJt18StSLoWN3B5ojSztBMdRaRNga5DWneNL9GViB712BJqYksh",
                    "ed25519:3XKJohJC5Vr79FM5aVfjGkBP2Ck8hNEtEemmMRKfNh8NLQyVAED8rqyHhxSG2G7tnmt37tUgaZQcyNaQe9AC7zB3",
                    "ed25519:wRkBDZg2MyGHZqhGYq8Pyv3uvu15jghNVWgdhmtsE5CFqzp4ws2YTCRnS4KUe3U7canCByh9hJHGetj9EHaGUop",
                    null,
                    "ed25519:5MiiKrUGXpcZB9VDVxUSpvKumFR9yiZgWEsVjHK4erJ4JEfzd7M17KnaaDWdLWn3w23drpqBLZCLxP6d1FqcvWP2",
                    "ed25519:3WC83k8v1AqtK8QzUsNFWKSQrTNxbfxmtm19xDwnTDG5W22uzYb232eBAwALFqZjSbNifr1DXd25fyE7msM6kfjc",
                    "ed25519:3UvniTBSgJPp8Mv1b8Z39pj7DSKZ3Epfy5xC7Mo4SAFDhgTZ7rxABtjT7tKj1S73JoREkzvdW2H1zRfeCRpWYyMU",
                    "ed25519:36XryCFKF9tVv4x6FhaJT8iYfrZPCFYbjnmteNkzXQcJRiRp2MwivcYpvrkUUzFYMDuN4uSdYgozs3uPqgKha6Mb",
                    null,
                    null,
                    "ed25519:5znLSC9mJRDEt5ozPc9cBisW5fn3matgmEcNBQtvvgpNNGgXYxHzW8aJkTmrovXRDiyWDbwkpY3GYbqPy66zGNSW",
                    "ed25519:35uHJvJ8cmQGxHWsPjkBMg4SCmDEEYmLgUuqvQZBarqx5uck6apdi3SRp3AgSPDzT8tFuGCiXz8EHByHjDmoGbiz",
                    "ed25519:45Qq5tSNJbzphyGerqEKCsEBq8bmrza5aferuEEijmhdgATdt6f4RDE9PDc86AwdURTLd7UVerkTtHheofa2YJet",
                    "ed25519:3G1gba8V5YsFdKnQmwpGfcy47J6etLeBz57oZwdrRnqWboQd15TRzJxzfmrMgMn415CpwLFq3iXWBrUUA2B3ZiPe",
                    null,
                    null,
                    "ed25519:5u63DbmzPiyB1R7DeCpKUAN1fJUTUpmh7FieGm6w1JWcJtHcD3EkMQGs5eoQ4XZbStACc3f9CSeQrz13hm7B2ipN",
                    "ed25519:2vgaVE77b38bFYNJort92hRJQrtxZp13vLCK6WQqs9cbWAQfk5pMnPcUZis2z9rbk411QhmtWo9WPHHspAgMUEaf",
                    "ed25519:TdUvco7vQAXqor6fBcwPBhyDaVKffYXknRB3T7cyWwDBNJ9etJtNje7wL6oQmUkQwndqwzKscNg8nKN38M5Fzdt",
                    "ed25519:dYFyNK7uNQECEXzrj4eQZAGdkeKrVxnsR6u3rRDE43uJTBf1tXPffspeonwMuFx9DqsGg4DSRy6hPPmzdNQruNz",
                    null,
                    null,
                    null,
                    "ed25519:5vsSMabj5pz7um6fvVKwF6WyJvsaEZ8YjyeqgSxSkZGWB2Zm2yaV7QqzTnzurx4KT7Zhdvow4HjA3hBWt8Wt1ti1",
                    null,
                    null,
                    "ed25519:3HfQt71AT6iVygpeNBHUaimx3iNnApbfTSsL5u65uyzkSHPCYwwSoQ7GfUCSuMp7HAm1cvpTf8RxzimKu9WeGa88",
                    null,
                    "ed25519:5ds369kTT4eUM1gcebAuuShPtft7LEZTA5oDwsxVv3Bazpb15WmDhSRuhUztGVTpDwMXijs68Gt7kUu3bD45KJpH",
                    null,
                    null,
                    null,
                    null,
                    null,
                    "ed25519:q7c7Mu5mKvZuBfpeRChGMbL1BZuwv72k2YvF9QoQHZE1yMRYnyQxvnAuHMiYLnnqKyD9PKA9ncssJfZcoL6jV2a",
                    "ed25519:5u6PWvtS88g13Z5aS6y57uBfuDXvXTw7Hr9ZaxcicadfyHZCft9tq71heoUva9ewZLznWsCBy7JCJ7m75JG1CDPA",
                    "ed25519:xjH8MR2JGH9ofpFPaU8GcoidsAohePQjtSi4M7T6SgXC1qZzhst6WLuguBaKTBqoPZU75N2Kkztfv3SKdyJKUQm",
                    "ed25519:3nVu8XDkNep3UDWF8QVvf9NvfL37Z5DBFEXJ6VbsosRjpH8NSuU8DquYfd55rfDNHDUxY1yerk1grz1GvorhUon6",
                    "ed25519:28qtBZAsbnBiZ6wPynzgTXuRm2fB4SPGmeBgyrfY61VSCNkr7LZ5zwLUxhrDUGQnLiaVS13tU9eBECvbocGZBgXE",
                    null,
                    "ed25519:M8ybFBsk3xZuXE48RwxSCVwyZB2srJVQ85cWazneyc1SQzuHXciKzouw3NXzwussKvpvvV4jsyPyEosfVmifnMm",
                    "ed25519:38yS9p1AcoXiS7E4EMn9gpCvAppCrdvygvDQwnP5VTjHahTbyGLV67mre1k4x9TZ2JD36sffYZzh5BBgaXpNSCJF",
                    "ed25519:G9H34TNeTP5QgifK9a6Y8PQVpnM5x7V2M7zSzsYCjdc1GUVsFFrMWPiJsigKnrV5pKi6yvWFUDwhYgXUPGGKZiY",
                    "ed25519:4tC17LadtbHChDDvEJaGrsmc1Jj7F6PT7GQq9Ncd8tykG5tNYxfA9kXz57tvwRbzeZjqjPykAPY2KrN4XMs4M9sB"
                ],
                "inner_lite": {
                    "block_merkle_root": "3MBnipBo8GnqJisZN3uFjHLuvMusBSCjMaQmUmj5u4J6",
                    "epoch_id": "GHmqgUX59irTdh31mtuEs3uEaPNBY5sQTZjEX5w7ASgW",
                    "height": 86456070,
                    "next_bp_hash": "9VPzyStHi4X2T7VAbfSTbLXEd8vjFP7wFJjYyjSJxQik",
                    "next_epoch_id": "8nVTHDfxg2G8AWbKhVfFtnEb5jJeiXV2XBFsdyt2cif1",
                    "outcome_root": "56KJ7kyW7aADwfDdNE4fz7pmPccBqkmxvnJ3nR1fewop",
                    "prev_state_root": "2VJekkjBnP36c3sGo9P2YxkEu9dabK9r5VdMTt7jADLv",
                    "timestamp": 1648810204507638699,
                    "timestamp_nanosec": "1648810204507638699"
                },
                "inner_rest_hash": "GQHrWtXByznAWcawC7GoEMumZ3GUi2T82MXV56c2x8KS",
                "next_block_inner_hash": "8rHAfAgpXQKXTDWwEPvHwzKaBjG67nv4eNGSfc7A8FZ5",
                "next_bps": [
                    {
                        "account_id": "node1",
                        "public_key": "ed25519:ydgzeXHJ5Xyt7M1gXLxqLBW1Ejx6scNV5Nx2pxFM8su",
                        "stake": "22949327592242450816363151898853",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "node0",
                        "public_key": "ed25519:ydgzeXHJ5Xyt7M1gXLxqLBW1Ejx6scNV5Nx2pxFM8su",
                        "stake": "16944923507607057621836326590864",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "node2",
                        "public_key": "ed25519:ydgzeXHJ5Xyt7M1gXLxqLBW1Ejx6scNV5Nx2pxFM8su",
                        "stake": "16894243398827941870356919783063",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "node3",
                        "public_key": "ed25519:ydgzeXHJ5Xyt7M1gXLxqLBW1Ejx6scNV5Nx2pxFM8su",
                        "stake": "8577838094223400746241842212915",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "legends.pool.f863973.m0",
                        "public_key": "ed25519:AhQ6sUifJYgjqarXSAzdDZU9ZixpUesP9JEH1Vr7NbaF",
                        "stake": "5793326871499643941084500854531",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "staked.pool.f863973.m0",
                        "public_key": "ed25519:D2afKYVaKQ1LGiWbMAZRfkKLgqimTR74wvtESvjx5Ft2",
                        "stake": "4559762052294055739961541809028",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "masternode24.pool.f863973.m0",
                        "public_key": "ed25519:9E3JvrQN6VGDGg1WJ3TjBsNyfmrU6kncBcDvvJLj6qHr",
                        "stake": "3416574120678826701003147150326",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "01node.pool.f863973.m0",
                        "public_key": "ed25519:3iNqnvBgxJPXCxu6hNdvJso1PEAc1miAD35KQMBCA3aL",
                        "stake": "3061276782639300406837420592214",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "p2p.pool.f863973.m0",
                        "public_key": "ed25519:4ie5979JdSR4f7MRAG58eghRxndVoKnAYAKa1PLoMYSS",
                        "stake": "2958427611565856637171061933942",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "nodeasy.pool.f863973.m0",
                        "public_key": "ed25519:25Dhg8NBvQhsVTuugav3t1To1X1zKiomDmnh8yN9hHMb",
                        "stake": "1575068818350064235628643461649",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "tribe-pool.pool.f863973.m0",
                        "public_key": "ed25519:CRS4HTSAeiP8FKD3c3ZrCL5pC92Mu1LQaWj22keThwFY",
                        "stake": "1429199212043501677779067532132",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "chorusone.pool.f863973.m0",
                        "public_key": "ed25519:3TkUuDpzrq75KtJhkuLfNNJBPHR5QEWpDxrter3znwto",
                        "stake": "1278827676875609593894511486301",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "hotones.pool.f863973.m0",
                        "public_key": "ed25519:2fc5xtbafKiLtxHskoPL2x7BpijxSZcwcAjzXceaxxWt",
                        "stake": "1273529881837124230828073909315",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "foundryusa.pool.f863973.m0",
                        "public_key": "ed25519:ABGnMW8c87ZKWxvZLLWgvrNe72HN7UoSf4cTBxCHbEE5",
                        "stake": "1256081604638924285747937189845",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "lunanova2.pool.f863973.m0",
                        "public_key": "ed25519:9Jv6e9Kye4wM9EL1XJvXY8CYsLi1HLdRKnTzXBQY44w9",
                        "stake": "1247431491303762172509349058430",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "chorus-one.pool.f863973.m0",
                        "public_key": "ed25519:6LFwyEEsqhuDxorWfsKcPPs324zLWTaoqk4o6RDXN7Qc",
                        "stake": "1110429050842727763339891353120",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "ni.pool.f863973.m0",
                        "public_key": "ed25519:GfCfFkLk2twbAWdsS3tr7C2eaiHN3znSfbshS5e8NqBS",
                        "stake": "1076903268858699791106964347506",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "cryptogarik.pool.f863973.m0",
                        "public_key": "ed25519:FyFYc2MVwgitVf4NDLawxVoiwUZ1gYsxGesGPvaZcv6j",
                        "stake": "840652974653901124214299092043",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "pathrocknetwork.pool.f863973.m0",
                        "public_key": "ed25519:CGzLGZEMb84nRSRZ7Au1ETAoQyN7SQXQi55fYafXq736",
                        "stake": "749739988926667488225409312930",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "stakely_v2.pool.f863973.m0",
                        "public_key": "ed25519:7BanKZKGvFjK5Yy83gfJ71vPhqRwsDDyVHrV2FMJCUWr",
                        "stake": "734779467803676488422251769143",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "solidstate.pool.f863973.m0",
                        "public_key": "ed25519:DTDhqoMXDWhKedWpH7DPvR6dPDcXrk5pTHJw2bkFFvQy",
                        "stake": "715205657993906057594050568659",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "aurora.pool.f863973.m0",
                        "public_key": "ed25519:9c7mczZpNzJz98V1sDeGybfD4gMybP4JKHotH8RrrHTm",
                        "stake": "703162032315675728652111978820",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "namdokmai.pool.f863973.m0",
                        "public_key": "ed25519:9uGeeM7j1fimpG7vn6EMcBXMei8ttWCohiMf44SoTzaz",
                        "stake": "699426128043696790256527911933",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "freshtest.pool.f863973.m0",
                        "public_key": "ed25519:5cbAt8uzmRztXWXKUYivtLsT2kMC414oHYDapfSJcgwv",
                        "stake": "697072950038835725218153979145",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "optimusvalidatornetwork.pool.f863973.m0",
                        "public_key": "ed25519:BGoxGmpvN7HdUSREQXfjH6kw5G6ph7NBXVfBVfUSH85V",
                        "stake": "661182931526239970852421432715",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "baziliknear.pool.f863973.m0",
                        "public_key": "ed25519:9Rbzfkhkk6RSa1HoPnJXS4q2nn1DwYeB4HMfJBB4WQpU",
                        "stake": "651150213650042597898598894903",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "blockscope.pool.f863973.m0",
                        "public_key": "ed25519:6K6xRp88BCQX5pcyrfkXDU371awMAmdXQY4gsxgjKmZz",
                        "stake": "649506414222131713576984442889",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "tagard.pool.f863973.m0",
                        "public_key": "ed25519:3KyziFgx3PpzorJnMFifXU4KsK4nwPFaxCGWTHaFBADK",
                        "stake": "646786097203475534304943885178",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "leadnode.pool.f863973.m0",
                        "public_key": "ed25519:CdP6CBFETfWYzrEedmpeqkR6rsJNeT22oUFn2mEDGk5i",
                        "stake": "644367778886663802105399198378",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "stakesstone.pool.f863973.m0",
                        "public_key": "ed25519:3aAdsKUuzZbjW9hHnmLWFRKwXjmcxsnLNLfNL4gP1wJ8",
                        "stake": "641198519157648602505664886163",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "basilisk-stake.pool.f863973.m0",
                        "public_key": "ed25519:CFo8vxoEUZoxbs87mGtG8qWUvSBHB91Vc6qWsaEXQ5cY",
                        "stake": "639918590440004706626411243128",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "shardlabs.pool.f863973.m0",
                        "public_key": "ed25519:DxmhGQZ6oqdxw7qGBvzLuBzE6XQjEh67hk5tt66vhLqL",
                        "stake": "637803882455578964186296090355",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "al3c5.pool.f863973.m0",
                        "public_key": "ed25519:BoYixTjyBePQ1VYP3s29rZfjtz1FLQ9og4FWZB5UgWCZ",
                        "stake": "636854880374440657378246667596",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "dehashed.pool.f863973.m0",
                        "public_key": "ed25519:EmPyD1DV9ajWJxjNN8GGACMyhM9w14brwNwYA5WvVaw",
                        "stake": "635224150718459403099965806552",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "machfund.pool.f863973.m0",
                        "public_key": "ed25519:G6fJ79oM6taQGhHeQZrg8N36nkCPMEVPyQMHfFT2wAKc",
                        "stake": "634686788251976758263963874506",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "blockngine.pool.f863973.m0",
                        "public_key": "ed25519:CZrTtCP6XkkxWtr3ATnXE8FL6bcG5cHcxfmdRgN7Lm7m",
                        "stake": "633656065475669726280826427959",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "grassets.pool.f863973.m0",
                        "public_key": "ed25519:3S4967Dt1VeeKrwBdTTR5tFEUFSwh17hEFLATRmtUNYV",
                        "stake": "622722987982775798532829252304",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "bflame.pool.f863973.m0",
                        "public_key": "ed25519:4uYM5RXgR9D6VAGKHgQTVNLEmCgMVX7PzpBstT92Me6R",
                        "stake": "617234461115345372278772960093",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "shurik.pool.f863973.m0",
                        "public_key": "ed25519:9zEn7DVpvQDxWdj5jSgrqJzqsLo8T9Wv37t83NXBiWi6",
                        "stake": "616327809807619407716759066614",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "dsrvlabs.pool.f863973.m0",
                        "public_key": "ed25519:61ei2efmmLkeDR1CG6JDEC2U3oZCUuC2K1X16Vmxrud9",
                        "stake": "613792106557214713239288385761",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "zetsi.pool.f863973.m0",
                        "public_key": "ed25519:6rYx5w1Z2pw46NBHv6Wo4JEUMNtqnDGqPaHT4wm15YRw",
                        "stake": "611882168159257611258042281605",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "n0ok.pool.f863973.m0",
                        "public_key": "ed25519:D6Gq2RpUoDUojmE2vLpqQzuZwYmFPW6rMcXPrwRYhqN8",
                        "stake": "594349395199079126466241101938",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "chelovek_iz_naroda.pool.f863973.m0",
                        "public_key": "ed25519:89aWsXXytjAZxyefXuGN73efnM9ugKTjPEGV4hDco8AZ",
                        "stake": "592739793772796190513231168872",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "lavenderfive.pool.f863973.m0",
                        "public_key": "ed25519:AzwAiLDqprZKpDjhsH7dfyvFdfSasmPTjuJUAHfX1Pg4",
                        "stake": "586231008421809079867645695624",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "latenthero.pool.f863973.m0",
                        "public_key": "ed25519:EQqmjRNouRKhwGL7Hnp3vcbDywg2Boj6to2gmnXybhEM",
                        "stake": "579738101137715103577294987834",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "tayang.pool.f863973.m0",
                        "public_key": "ed25519:G9XWX55MfWEpT84ckcsJxVTKeZK4WqBGJX3xVpnPB5vv",
                        "stake": "563498889920635651950224126233",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "smcvalidator.pool.f863973.m0",
                        "public_key": "ed25519:pG4LYsyoAa8yWYG9nsTQ5yBcwke51i3VqeRcMVbE9Q7",
                        "stake": "555422197586970576403131175346",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "everstake.pool.f863973.m0",
                        "public_key": "ed25519:4LDN8tZUTRRc4siGmYCPA67tRyxStACDchdGDZYKdFsw",
                        "stake": "546400197607367519956748211889",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "rossi-validator.pool.f863973.m0",
                        "public_key": "ed25519:2eRx2c3KX9wFd3EzuuajFQoSxRTKDqSbxcF13LfkrxCR",
                        "stake": "545396693549586230215202952473",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "infiniteloop.pool.f863973.m0",
                        "public_key": "ed25519:2fbiLqksH5viWXYoteyfKP9qQawkRKw4YogRFcvG3Z7f",
                        "stake": "538321976932135835213436874121",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "lusienda.pool.f863973.m0",
                        "public_key": "ed25519:HdQb2HEiaMgvUdemTt5rkrFbxTpzZyELvg1Vov4LQAGU",
                        "stake": "509015164869674763004419847436",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "ino.pool.f863973.m0",
                        "public_key": "ed25519:B75h2eqpaMgh6WkAvgnz2FsEC9s5TwVx7zwTjqXKfRs6",
                        "stake": "494974817444468749939621071716",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "pontiff.pool.f863973.m0",
                        "public_key": "ed25519:4i8j7nwNyy18hfARtrVpckT8MiicdCXuWBX1TubdMb5Y",
                        "stake": "478587210879643963063840990682",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "kiln.pool.f863973.m0",
                        "public_key": "ed25519:Bq8fe1eUgDRexX2CYDMhMMQBiN13j8vTAVFyTNhEfh1W",
                        "stake": "96608509421037438882028377566",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "nodemeister.pool.f863973.m0",
                        "public_key": "ed25519:85EMyaNGMFuHK2RDH7KHno6fVYBR6iykUXHPPmFTGuTB",
                        "stake": "47021543808070096585479049932",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "nala.pool.f863973.m0",
                        "public_key": "ed25519:Fzwndob2h5PFdEuwo9eRFJV3BLLurcNaw2SGob5rMPEn",
                        "stake": "44766587364445748049092546945",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "happystake.pool.f863973.m0",
                        "public_key": "ed25519:3APqZiwzeZLzgfkJyGGTfepDYHA2d8NF1wZi4mCpZnaJ",
                        "stake": "43959988855512773720415910025",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "ibb.pool.f863973.m0",
                        "public_key": "ed25519:7gvdHhcMcXT1jMZoxDKy7yXnRiPVX1tAFTa7HWTHbe8C",
                        "stake": "42001690004861681144621857517",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "mateennala.pool.f863973.m0",
                        "public_key": "ed25519:9kNpQKUKzhc1AiFSEoZcTNapTnywjbXBPngH3EDpD1tw",
                        "stake": "40056014128143748170300000000",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "wolfedge-capital-testnet.pool.f863973.m0",
                        "public_key": "ed25519:CQEMcPQz6sqhAgoBm9ka9UeVcXj5NpNpRtDYYGkPggvg",
                        "stake": "37464905110868615156797728096",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "jstaking.pool.f863973.m0",
                        "public_key": "ed25519:fui1E5XwnAWGYDBSQ3168aDfsW1KDFH8A7nBHvZiqGv",
                        "stake": "36368375383183646876651257216",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "dariya.pool.f863973.m0",
                        "public_key": "ed25519:A5Rx38TsNKWXzF5o18HpaRrPeBzv3riqur51bqhU1Qbp",
                        "stake": "36211347514033914937590010268",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "4ire-pool.pool.f863973.m0",
                        "public_key": "ed25519:EWPSvYN9pGPMmCLjVxx96stWdqksXNSGnfnuWYn9iiE5",
                        "stake": "33869896086305183386478534323",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "lionstake.pool.f863973.m0",
                        "public_key": "ed25519:Fy6quR4nBhrEnDyEuPWoAdBP5tzNbuEZsEd91Q5pQnXB",
                        "stake": "33765876364623459491244697143",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "zentriav2.factory.colorpalette.testnet",
                        "public_key": "ed25519:4rCwSFzJ2e6suD5Yi7pgLidcAJ8Zt9BXieLzVedJDwmE",
                        "stake": "30596434283244809799848018489",
                        "validator_stake_struct_version": "V1"
                    },
                    {
                        "account_id": "lastnode.pool.f863973.m0",
                        "public_key": "ed25519:811gesxXYdYeThry96ZiWn8chgWYNyreiScMkmxg4U9u",
                        "stake": "24146328727357015429360981746",
                        "validator_stake_struct_version": "V1"
                    }
                ],
                "prev_block_hash": "4E2VN7cUVSb8ek761H4cRo57ERTWBKbcB9uEBDS2cWhD"
            },
            "id": "idontcare"
        }
        "#;
        let client_block_near_view_next_epoch =
            get_client_near_block_view(CLIENT_BLOCK_VIEW).unwrap();

        let near_client_serialized = client_block_near_view_next_epoch.try_to_vec().unwrap();
        let lite_client_block_view: LightClientBlockView =
            BorshDeserialize::try_from_slice(near_client_serialized.as_ref()).unwrap();

        let near_light_client_from_from_serialized = NearLightClientBlockView::try_from_slice(
            lite_client_block_view.try_to_vec().unwrap().as_ref(),
        )
        .unwrap();

        assert_eq!(
            near_light_client_from_from_serialized,
            client_block_near_view_next_epoch
        );

        // assert_eq!(
        //     near_client_serialized.try_to_vec().unwrap(),
        //     lite_client_block_view.try_to_vec().unwrap()
        // );
    }

}
