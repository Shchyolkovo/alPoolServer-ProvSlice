
use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::sink::SinkExt;
use rand::{rngs::OsRng, Rng};
use snarkos_account::Account;
use snarkos_node_router_messages::{
    ChallengeRequest,
    ChallengeResponse,
    MessageCodec,
    NodeType,
    Ping,
    Pong,
    PuzzleRequest,
    PuzzleResponse,
};
use snarkvm::prelude::{Block, Field, FromBytes, Network};
use snarkvm_ledger_narwhal_data::Data;
use tokio::{
    net::TcpStream,
    sync::{
        mpsc,
        mpsc::{Receiver, Sender},
        Mutex,
        RwLock,
    },
    task,
    time::{sleep, timeout},
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, trace, warn};

use crate::{ServerMessage, N};

pub struct Node {
    operator: String,
    sender: Arc<Sender<SnarkOSMessage>>,
    receiver: Arc<Mutex<Receiver<SnarkOSMessage>>>,
    pending_solutions: Arc<RwLock<Vec<SnarkOSMessage>>>,
}

pub(crate) type SnarkOSMessage = snarkos_node_router_messages::Message<N>;

impl Node {
    pub fn init(operator: String) -> Self {
        let (sender, receiver) = mpsc::channel(1024);
        Self {
            operator,
            sender: Arc::new(sender),
            receiver: Arc::new(Mutex::new(receiver)),
            pending_solutions: Default::default(),
        }
    }

    pub fn receiver(&self) -> Arc<Mutex<Receiver<SnarkOSMessage>>> {
        self.receiver.clone()
    }

    pub fn sender(&self) -> Arc<Sender<SnarkOSMessage>> {
        self.sender.clone()
    }
}

pub fn start(node: Node, server_sender: Sender<ServerMessage>, genesis_path: Option<String>) {
    let receiver = node.receiver();
    let sender = node.sender();
    task::spawn(async move {
        let genesis_header = match genesis_path {
            Some(path) => {
                let bytes = std::fs::read(path).unwrap();
                *Block::<N>::from_bytes_le(&bytes).unwrap().header()
            }
            None => *Block::<N>::from_bytes_le(N::genesis_bytes()).unwrap().header(),
        };
        let connected = Arc::new(AtomicBool::new(false));
        let peer_sender = sender.clone();
        let peer_sender_ping = sender.clone();

        let connected_req = connected.clone();
        let connected_ping = connected.clone();
        let pending_req = node.pending_solutions.clone();
        task::spawn(async move {
            loop {
                sleep(Duration::from_secs(15)).await;
                if connected_req.load(Ordering::SeqCst) {
                    if let Err(e) = peer_sender.send(SnarkOSMessage::PuzzleRequest(PuzzleRequest {})).await {
                        error!("Failed to send puzzle request: {}", e);
                    }
                    let mut pending_solutions = pending_req.write().await.clone();
                    let mut failed_solutions: Vec<SnarkOSMessage> = vec![];
                    while let Some(message) = pending_solutions.pop() {
                        if let Err(e) = peer_sender.send(message.clone()).await {
                            failed_solutions.push(message);
                            error!("Failed to send puzzle request: {}", e);
                        }
                    }
                    pending_solutions.extend(failed_solutions);
                }
            }
        });
        task::spawn(async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                if connected_ping.load(Ordering::SeqCst) {
                    if let Err(e) = peer_sender_ping
                        .send(SnarkOSMessage::Ping(Ping {
                            version: SnarkOSMessage::VERSION,
                            node_type: NodeType::Prover,
                            block_locators: None,
                        }))
                        .await