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
    Authorize(Address<Testne