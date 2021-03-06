use super::{BlockID, TendermintSign, Time};
use chrono::{DateTime, Utc};
use std::time::{SystemTime, UNIX_EPOCH};
use subtle_encoding::hex::encode_upper;
// TODO(ismail): we might not want to use this error type here
// see below: those aren't prost errors
use prost::error::DecodeError;

enum VoteType {
    PreVote,
    PreCommit,
}

fn vote_type_to_char(vt: &VoteType) -> char {
    match *vt {
        VoteType::PreVote => 0x01 as char,
        VoteType::PreCommit => 0x02 as char,
    }
}

fn u32_to_vote_type(data: u32) -> Result<VoteType, DecodeError> {
    match data {
        1 => Ok(VoteType::PreVote),
        2 => Ok(VoteType::PreCommit),
        _ => Err(DecodeError::new("Invalid vote type")),
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct Vote {
    #[prost(bytes, tag = "1")]
    validator_address: Vec<u8>,
    #[prost(sint64)]
    validator_index: i64,
    #[prost(sint64)]
    height: i64,
    #[prost(sint64)]
    round: i64,
    #[prost(message)]
    timestamp: Option<Time>,
    #[prost(uint32)]
    vote_type: u32,
    #[prost(message)]
    block_id: Option<BlockID>,
    #[prost(message)]
    signature: Option<Vec<u8>>,
}

pub const AMINO_NAME: &str = "tendermint/socketpv/SignVoteMsg";

#[derive(Clone, PartialEq, Message)]
#[amino_name = "tendermint/socketpv/SignVoteMsg"]
pub struct SignVoteMsg {
    #[prost(message, tag = "1")]
    vote: Option<Vote>,
}

impl TendermintSign for SignVoteMsg {
    fn cannonicalize(self, chain_id: &str) -> String {
        match self.vote {
            Some(vote) => {
                let empty: Vec<u8> = b"".to_vec();
                let value = json!({
            "@chain_id":chain_id,
            "@type":"vote",
            "block_id":{
                "hash":encode_upper(match &vote.block_id {
                    Some(ref block_id) => &block_id.hash,
                    None => &empty,
                }),
                "parts":{
                    "hash":encode_upper(match &vote.block_id {
                        Some(block_id) => match &block_id.parts_header {
                            Some(ref parts_header) => &parts_header.hash,
                            None => &empty,
                        },
                        None => &empty,

                    }),
                    "total":match vote.block_id {
                        Some(block_id) => match block_id.parts_header {
                            Some(parts_header) => parts_header.total,
                            None => 0,
                        },
                        None => 0,
                    }
                }
            },
            "height":vote.height,
            "round":vote.round,
            "timestamp": match vote.timestamp  {
                   Some(timestamp) =>  {
                        let ts: DateTime<Utc> =  DateTime::from(SystemTime::from(timestamp));
                        ts.to_rfc3339()
                   },
                   None => {
                    let ts: DateTime<Utc> =   DateTime::from(UNIX_EPOCH);
                    ts.to_rfc3339()
                   }
            },
            "type": match u32_to_vote_type(vote.vote_type) {
                Ok(ref vt) => vote_type_to_char(vt),
                Err(_e) => 0 as char,
            }
            });
                value.to_string()
            }
            None => "".to_owned(),
        }
    }
    fn sign(&mut self) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::super::PartsSetHeader;
    use super::*;
    use prost::Message;

    #[test]
    fn test_vote_serialization() {
        let dt = "2017-12-25T03:00:01.234Z".parse::<DateTime<Utc>>().unwrap();
        let t = Time {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        };
        let vote = Vote {
            validator_address: vec![
                0xa3, 0xb2, 0xcc, 0xdd, 0x71, 0x86, 0xf1, 0x68, 0x5f, 0x21, 0xf2, 0x48, 0x2a, 0xf4,
                0xfb, 0x34, 0x46, 0xa8, 0x4b, 0x35,
            ],
            validator_index: 56789,
            height: 12345,
            round: 2,
            timestamp: Some(t),
            vote_type: 0x01,
            block_id: Some(BlockID {
                hash: "hash".as_bytes().to_vec(),
                parts_header: Some(PartsSetHeader {
                    total: 1000000,
                    hash: "parts_hash".as_bytes().to_vec(),
                }),
            }),
            signature: None,
        };
        let sign_vote_msg = SignVoteMsg { vote: Some(vote) };
        let mut got = vec![];
        let _have = sign_vote_msg.encode(&mut got);
        // the following vector is generated via:
        //  cdc := amino.NewCodec()
        //	cdc.RegisterInterface((*privval.SocketPVMsg)(nil), nil)
        //	cdc.RegisterInterface((*crypto.Signature)(nil), nil)
        //	cdc.RegisterConcrete(crypto.SignatureEd25519{},
        //		"tendermint/SignatureEd25519", nil)
        //
        //	cdc.RegisterConcrete(&privval.PubKeyMsg{}, "tendermint/socketpv/PubKeyMsg", nil)
        //	cdc.RegisterConcrete(&privval.SignVoteMsg{}, "tendermint/socketpv/SignVoteMsg", nil)
        //	cdc.RegisterConcrete(&privval.SignProposalMsg{}, "tendermint/socketpv/SignProposalMsg", nil)
        //	cdc.RegisterConcrete(&privval.SignHeartbeatMsg{}, "tendermint/socketpv/SignHeartbeatMsg", nil)
        //	data, _ := cdc.MarshalBinary(privval.SignVoteMsg{Vote: vote})
        //
        // where vote is equal to
        //
        //  types.Vote{
        //		ValidatorAddress: []byte{0xa3, 0xb2, 0xcc, 0xdd, 0x71, 0x86, 0xf1, 0x68, 0x5f, 0x21, 0xf2, 0x48, 0x2a, 0xf4, 0xfb, 0x34, 0x46, 0xa8, 0x4b, 0x35},
        //		ValidatorIndex:   56789,
        //		Height:           12345,
        //		Round:            2,
        //		Timestamp:        stamp,
        //		Type:             byte(0x01), // pre-vote
        //		BlockID: types.BlockID{
        //			Hash: []byte("hash"),
        //			PartsHeader: types.PartSetHeader{
        //				Total: 1000000,
        //				Hash:  []byte("parts_hash"),
        //			},
        //		},
        //	}
        let want = vec![
            0x52, 0x6c, 0x1d, 0x3a, 0x35, 0xa, 0x4c, 0xa, 0x14, 0xa3, 0xb2, 0xcc, 0xdd, 0x71, 0x86,
            0xf1, 0x68, 0x5f, 0x21, 0xf2, 0x48, 0x2a, 0xf4, 0xfb, 0x34, 0x46, 0xa8, 0x4b, 0x35,
            0x10, 0xaa, 0xf7, 0x6, 0x18, 0xf2, 0xc0, 0x1, 0x20, 0x4, 0x2a, 0xe, 0x9, 0xb1, 0x69,
            0x40, 0x5a, 0x0, 0x0, 0x0, 0x0, 0x15, 0x80, 0x8e, 0xf2, 0xd, 0x30, 0x1, 0x3a, 0x18,
            0xa, 0x4, 0x68, 0x61, 0x73, 0x68, 0x12, 0x10, 0x8, 0x80, 0x89, 0x7a, 0x12, 0xa, 0x70,
            0x61, 0x72, 0x74, 0x73, 0x5f, 0x68, 0x61, 0x73, 0x68,
        ];
        assert_eq!(got, want);
    }

    #[test]
    fn test_deserialization() {
        let encoded = vec![
            0x52, 0x6c, 0x1d, 0x3a, 0x35, 0xa, 0x4c, 0xa, 0x14, 0xa3, 0xb2, 0xcc, 0xdd, 0x71, 0x86,
            0xf1, 0x68, 0x5f, 0x21, 0xf2, 0x48, 0x2a, 0xf4, 0xfb, 0x34, 0x46, 0xa8, 0x4b, 0x35,
            0x10, 0xaa, 0xf7, 0x6, 0x18, 0xf2, 0xc0, 0x1, 0x20, 0x4, 0x2a, 0xe, 0x9, 0xb1, 0x69,
            0x40, 0x5a, 0x0, 0x0, 0x0, 0x0, 0x15, 0x80, 0x8e, 0xf2, 0xd, 0x30, 0x1, 0x3a, 0x18,
            0xa, 0x4, 0x68, 0x61, 0x73, 0x68, 0x12, 0x10, 0x8, 0x80, 0x89, 0x7a, 0x12, 0xa, 0x70,
            0x61, 0x72, 0x74, 0x73, 0x5f, 0x68, 0x61, 0x73, 0x68,
        ];
        let dt = "2017-12-25T03:00:01.234Z".parse::<DateTime<Utc>>().unwrap();
        let t = Time {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32,
        };
        let vote = Vote {
            validator_address: vec![
                0xa3, 0xb2, 0xcc, 0xdd, 0x71, 0x86, 0xf1, 0x68, 0x5f, 0x21, 0xf2, 0x48, 0x2a, 0xf4,
                0xfb, 0x34, 0x46, 0xa8, 0x4b, 0x35,
            ],
            validator_index: 56789,
            height: 12345,
            round: 2,
            timestamp: Some(t),
            vote_type: 0x01,
            block_id: Some(BlockID {
                hash: "hash".as_bytes().to_vec(),
                parts_header: Some(PartsSetHeader {
                    total: 1000000,
                    hash: "parts_hash".as_bytes().to_vec(),
                }),
            }),
            signature: None,
        };
        let want = SignVoteMsg { vote: Some(vote) };
        match SignVoteMsg::decode(&encoded) {
            Ok(have) => {
                assert_eq!(have, want);
                println!("{}", have.cannonicalize("chain_iddddd"));
            }
            Err(err) => assert!(false, err.to_string()),
        }
    }
}
