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
        task::spawn(Connection::run(stream, peer_addr, server_sender, pool_address));
    }

    pub async fn run(
        stream: TcpStream,
        peer_addr: SocketAddr,
        server_sender: Sender<ServerMessage>,
        pool_address: Address<N>,
    ) {
        let mut framed = Framed::new(stream, StratumCodec::default());

        let (sender, mut receiver) = channel(1024);

        let mut conn = Connection {
            user_agent: "Unknown".to_string(),
            address: None,
            version: Version::new(0, 0, 0),
            last_received: None,
        };

        // Handshake

        if let Ok((user_agent, version)) = Connection::handshake(&mut framed, pool_address.to_string()).await {
            conn.user_agent = user_agent;
            conn.version = version;
        } else {
            if let Err(e) = server_sender.send(ServerMessage::ProverDisconnected(peer_addr)).await {
                error!("Failed to send ProverDisconnected message to server: {}", e);
            }
            return;
        }

        if let Ok(address) = Connection::authorize(&mut framed).await {
            conn.address = Some(address);
            if let Err(e) = server_sender
                .send(ServerMessage::ProverAuthenticated(
                    peer_addr,
                    conn.address.unwrap(),
                    sender,
                ))
                .await
            {
                error!("Failed to send ProverAuthenticated message to server: {}", e);
            }
        } else {
            if let Err(e) = server_sender.send(ServerMessage::ProverDisconnected(peer_addr)).await {
                error!("Failed to send ProverDisconnected message to server: {}", e);
            }
            return;
        }

        conn.last_received = Some(Instant::now());

        info!("Peer {:?} authenticated as {}", peer_addr, conn.address.unwrap());

        loop {
            tokio::select! {
                Some(msg) = receiver.recv() => {
                    if let Some(instant) = conn.last_received {
                        if instant.elapsed() > PEER_COMM_TIMEOUT {
                            warn!("Peer {:?} timed out", peer_addr);
                            break;
                        }
                    }
                    trace!("Sending message {} to peer {:?}", msg.name(), peer_addr);
                    if let Err(e) = framed.send(msg).await {
                        error!("Failed to send message to peer {:?}: {:?}", peer_addr, e);
                    }
                },
                result = framed.next() => match result {
                    Some(Ok(msg)) => {
                        trace!("Received message {} from peer {:?}", msg.name(), peer_addr);
                        conn.last_received = Some(Instant::now());
                        match msg {
                            StratumMessage::Submit(id, _worker_name, job_id, counter) => {
                                let job_bytes = hex::decode(job_id.clone());
                                if job_bytes.is_err() {
                                    warn!("Failed to decode job_id {} from peer {:?}", job_id, peer_addr);
                                    break;
                                }
                                if job_bytes.clone().unwrap().len() != 4 {
                  