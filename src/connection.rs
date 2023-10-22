use std::{
    net::SocketAddr,
    str::FromStr,
    time::{Duration, Instant},
};

use aleo_stratum::{
    codec::{BoxedType, ResponseParams, StratumCodec},
    message::StratumMessage,
};
use anyhow::{anyhow, Result};
use futures_util::SinkExt;
use semver::Version;
use snarkvm::console::account::Address;
use tokio::{
    net::TcpStream,
    sync::mpsc::{channel, Sender},
    task,
    time::timeout,
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{error, info, trace, w