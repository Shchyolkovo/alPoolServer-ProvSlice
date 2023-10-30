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
use tracing::{error, info, trace, warn};

use crate::{server::ServerMessage, N};

pub struct Connection {
    user_agent: String,
    address: Option<Address<N>>,
    version: Version,
    last_received: Option<Instant>,
}

static PEER_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
static PEER_COMM_TIMEOUT: Duration = Duration::from_secs(180);

static MIN_SUPPORTED_VERSION: Version = Version::new(3, 0, 0);
static MAX_SUPPORTED_VERSION: Version = Version::new(3, 0, 0);

impl Connection {
    pub async fn init(
        stream: TcpStream,
        peer_addr: SocketAddr,
        server_sender: Sender<ServerMessage>,
        pool_address: Address<N>,
    ) {
        ta