use std::io::Write;

use anyhow::anyhow;
use byteorder::{LittleEndian, ReadBytesExt};
use bytes::{Buf, BufMut, BytesMut};
use snarkvm::{
    dpc::{testnet2::Testnet2, Address, BlockTemplate, PoSWProof},
    traits::Network,
    utilities::{FromBytes, ToBytes},
};
use tokio_util::codec::{Decoder, Encoder};

/// Not being used anymore as we are migrating to "standard" stratum+tcp protocol.
#[allow(clippy::large_enum_variant)]
pub enum ProverMessage {
    // as in stratum, with an additional protocol version field
    Authorize(Address<Testnet2>, String, u16),
    AuthorizeResult(bool, Option<String>),

    // combine notify and set_difficulty to be consistent
    Notify(BlockTemplate<Testnet2>, u64),
    // include block height to detect stales faster
    Submit(u32, <Testnet2 as Network>::PoSWNonce, PoSWProof<Testnet2>),
    // miners might want to know the stale rate, optionally provide a message
    SubmitResult(bool, Option<String>),

    Canary,
}

#[allow(dead_code)]
static VERSION: u16 = 1;

impl ProverMessage {
    #[allow(dead_code)]
    pub fn version() -> &'static u16 {
        &VERSION
    }

    pub fn id(&self) -> u8 {
        match self {
            ProverMessage::Authorize(..) => 0,
            ProverMessage::AuthorizeResult(..) => 1,
            ProverMessage::Notify(..) => 2,
            ProverMessage::Submit(..) => 3,
            ProverMessage::SubmitResult(..) => 4,

            ProverMessage::Canary => 5,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ProverMessage::Authorize(..) => "Authorize",
            ProverMessage::AuthorizeResult(..) => "AuthorizeResult",
            ProverMessage::Notify(..) => "Notify",
            ProverMessage::Submit(..) => "Submit",
            ProverMessage::SubmitResult(..) => "SubmitResult",

            ProverMessage::Canary => "Canary",
        }
    }
}

impl Encoder<ProverMessage> for ProverMessage {
    type Error = anyhow::Error;

    fn encode(&mut self, item: ProverMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.extend_from_slice(&0u32.to_le_bytes());
        let mut writer = dst.writer();
        writer.write_all(&[item.id()])?;
        match item {
            ProverMessage::Authorize(addr, password, version) => {
                bincode::serialize_into(&mut writer, &addr)?;
                bincode::serialize_into(&mut writer, &password)?;
                writer.write_all(&version.to_le_bytes())?;
            }
            ProverMessage::AuthorizeResult(result, message) | ProverMessage::SubmitResult(result, message) => {
                writer.write_all(&[match result {
                    true => 1,
                    false => 0,
                }])?;
                if let Some(message) = message {
                    writer.write_all(&[1])?;
                    bincode::serialize_into(&mut writer, &message)?;
                } else {
                    writer.write_all(&[0])?;
                }
            }
            ProverMessage::Notify(template, difficulty) => {
                template.write_le(&mut writer)?;
                writer.write_all(&difficulty.to_le_bytes())?;
            }
            ProverMessage::Submit(height, nonce, proof) => {
                writer.write_all(&height.to_le_bytes())?;
                nonce.write_le(&mut writer)?;
                proof.write_le(&mut wr