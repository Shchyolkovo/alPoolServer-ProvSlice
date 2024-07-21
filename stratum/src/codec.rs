
use std::io;

use bytes::BytesMut;
use downcast_rs::{impl_downcast, DowncastSync};
use erased_serde::Serialize as ErasedSerialize;
use json_rpc_types::{Id, Request, Response, Version};
use serde::{ser::SerializeSeq, Deserialize, Serialize};
use serde_json::Value;
use tokio_util::codec::{AnyDelimiterCodec, Decoder, Encoder};

use crate::message::StratumMessage;

pub struct StratumCodec {
    codec: AnyDelimiterCodec,
}

impl Default for StratumCodec {
    fn default() -> Self {
        Self {
            // Notify is ~400 bytes and submit is ~1750 bytes. 4096 should be enough for all messages
            // TODO: verify again
            codec: AnyDelimiterCodec::new_with_max_length(vec![b'\n'], vec![b'\n'], 4096),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct NotifyParams(String, String, Option<String>, bool);

#[derive(Serialize, Deserialize)]
struct SubscribeParams(String, String, Option<String>);

pub trait BoxedType: ErasedSerialize + Send + DowncastSync {}
erased_serde::serialize_trait_object!(BoxedType);
impl_downcast!(sync BoxedType);

impl BoxedType for String {}
impl BoxedType for Option<u64> {}
impl BoxedType for Option<String> {}
