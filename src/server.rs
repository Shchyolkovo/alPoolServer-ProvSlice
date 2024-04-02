use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Formatter},
    net::SocketAddr,
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use aleo_stratum::{codec::ResponseParams, message::StratumMessage};
use flurry::HashSet as FlurryHashSet;
use json_rpc_types::{Error, ErrorCode, Id};
use snarkos_node_router_messages::UnconfirmedSolution;
use snarkvm::{
    circuit::AleoV0,
    console::account::Address,
    ledger::{
        narwhal::Data,
        puzzle::{PartialSolution, Puzzle, Solution},
    },
    prelude::{Network, ToBytes},
};
use snarkvm_ledger_puzzle_epoch::SynthesisPuzzle;
use speedometer::Speedometer;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{channel, Sender},
        RwLock,
    },
    task,
};
use tracing::{debug, error, info, trace, warn};

use crate::{connection::Connection, prover_peer::SnarkOSMessage, AccountingMessage, N};

type A = AleoV0;

struct ProverState {
    peer_addr: SocketAddr,
    address: Address<N>,
    speed_2m: Speedometer,
    speed_5m: Speedometer,
    speed_15m: Speedometer,
    speed_30m: Speedometer,
    speed_1h: Speedometer,
    current_target: u64,
    next_target: u64,
}

impl ProverState {
    pub fn new(peer_addr: SocketAddr, address: Address<N>) -> Self {
        Self {
            peer_addr,
            address,
            speed_2m: Speedometer::init(Duration::from_secs(120)),
            speed_5m: Speedometer::init_with_cache(Duration::from_secs(60 * 5), Duration::from_secs(30)),
            speed_15m: Speedometer::init_with_cache(Duration::from_secs(60 * 15), Duration::from_secs(30)),
            speed_30m: Speedometer::init_with_cache(Duration::from_secs(60 * 30), Duration::from_secs(30)),
            speed_1h: Speedometer::init_with_cache(Duration::from_secs(60 * 60), Duration::from_secs(30)),
            current_target: 512,
            next_target: 512,
        }
    }

    pub async fn add_share(&mut self, value: u64) {
        let now = Instant::now();
        self.speed_2m.event(value).await;
        self.speed_5m.event(value).await;
        self.speed_15m.event(value).await;
        self.speed_30m.event(value).await;
        self.speed_1h.event(value).await;
        self.next_target = ((self.speed_2m.speed().await * 20.0) as u64).max(1);
        debug!("add_share took {} us", now.elapsed().as_micros());
    }

    pub async fn next_target(&mut self) -> u64 {
        if self.next_target < ((self.current_target as f64) * 0.9) as u64
            || self.next_target > ((self.current_target as f64) * 1.1) as u64
        {
            self.current_target = self.next_target;
        }
        self.current_target
    }

    pub fn current_target(&self) -> u64 {
        self.current_target
    }

    pub fn address(&self) -> Address<N> {
        self.address
    }

    // noinspection DuplicatedCode
    pub async fn speed(&mut self) -> Vec<f64> {
        vec![
            self.speed_5m.speed().await,
            self.speed_15m.speed().await,
            self.speed_30m.speed().await,
            self.speed_1h.speed().await,
        ]
    }
}

impl Display for ProverState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let addr_str = self.address.to_string();
        write!(
            f,
            "{} ({}...{})",
            self.peer_addr,
            &addr_str[0..11],
            &addr_str[addr_str.len() - 6..]
        )
    }
}

struct PoolState {
    speed_1m: Speedometer,
    speed_5m: Speedometer,
    speed_15m: Speedometer,
    speed_30m: Speedometer,
    speed_1h: Speedometer,
    current_global_target_modifier: f64,
    next_global_target_modifier: f64,
}

impl PoolState {
    pub fn new() -> Self {
        Self {
            speed_1m: Speedometer::init(Duration::from_secs(60)),
            speed_5m: Speedometer::init_with_cache(Duration::from_secs(60 * 5), Duration::from_secs(30)),
            speed_15m: Speedometer::init_with_cache(Duration::from_secs(60 * 15), Duration::from_secs(30)),
            speed_30m: Speedometer::init_with_cache(Duration::from_secs(60 * 30), Duration::from_secs(30)),
            speed_1h: Speedometer::init_with_cache(Duration::from_secs(60 * 60), Duration::from_secs(30)),
            current_global_target_modifier: 1.0,
            next_global_target_modifier: 1.0,
        }
    }

    pub async fn add_share(&mut self, value: u64) {
        let now = Instant::now();
        self.speed_1m.event(1).await;
        self.speed_5m.event(value).await;
        self.speed_15m.event(value).await;
        self.speed_30m.event(value).await;
        self.speed_1h.event(value).await;
        self.next_global_target_modifier = (self.speed_1m.speed().await / 200.0).max(1f64);
        // todo: make adjustable through admin api
        debug!("pool state add_share took {} us", now.elapsed().as_micros());
    }

    pub async fn next_global_target_modifier(&mut self) -> f64 {
        self.current_global_target_modifier = self.next_global_target_modifier;
        if self.current_global_target_modifier > 1.0 {
            info!(
                "Current global target modifier: {}",
                self.current_global_target_modifier
            );
        }
        self.current_global_target_modifier
    }

    pub fn current_global_target_modifier(&self) -> f64 {
        self.current_global_target_modifier
    }

    // noinspection DuplicatedCode
    pub async fn speed(&mut self) -> Vec<f64> {
        vec![
            self.speed_5m.speed().await,
            self.speed_15m.speed().await,
            self.speed_30m.speed().await,
            self.speed_1h.speed().await,
        ]
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum ServerMessage {
    ProverConnected(TcpStream, SocketAddr),
    ProverAuthenticated(SocketAddr, Address<N>, Sender<StratumMessage>),
    ProverDisconnected(SocketAddr),
    ProverSubmit(Id, SocketAddr, u32, u64),
    NewEpochHash(<N as Network>::BlockHash, u32, u64),
    Exit,
}

impl ServerMessage {
    fn name(&self) -> &'static str {
        match self {
            ServerMessage::ProverConnected(..) => "ProverConnected",
            ServerMessage::ProverAuthenticated(..) => "ProverAuthenticated",
            ServerMessage::ProverDisconnected(..) => "ProverDisconnected",
            ServerMessage::ProverSubmit(..) => "ProverSubmit",
            ServerMessage::NewEpochHash(..) => "NewEpochChallenge",
            ServerMessage::Exit => "Exit",
        }
    }
}

impl Display for ServerMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub struct Server {
    sender: Sender<ServerMessage>,
    prover_sender: Arc<Sender<SnarkOSMessage>>,
    accounting_sender: Sender<AccountingMessage>,
    pool_address: Address<N>,
    connected_provers: RwLock<HashSet<SocketAddr>>,
    authenticated_provers: Arc<RwLock<HashMap<SocketAddr, Sender<StratumMessage>>>>,
    pool_state: Arc<RwLock<PoolState>>,
    prover_states: Arc<RwLock<HashMap<SocketAddr, RwLock<ProverState>>>>,
    prover_address_connections: Arc<RwLock<HashMap<Address<N>, HashSet<SocketAddr>>>>,
    latest_epoch_number: AtomicU32,
    latest_epoch_hash: Arc<RwLock<Option<<N as Network>::BlockHash>>>,
    latest_proof_target: AtomicU64,
    nonce_seen: Arc<FlurryHashSet<u64>>,
    puzzle: Puzzle<N>,
}

impl Server {
    pub async fn init(
        port: u16,
        address: Address<N>,
        prover_sender: Arc<Sender<SnarkOSMessage>>,
        accounting_sender: Sender<AccountingMessage>,
    ) -> Arc<Server> {
        let (sender, mut receiver) = channel(1024);

        let (_, listener) = match TcpListener::bind(format!("0.0.0.0:{}", port)).await {
            Ok(listener) => {
                let local_ip = listener.local_addr().expect("Could not get local ip");
                info!("Listening on {}", local_ip);
                (local_ip, listener)
            }
            Err(e) => {
                panic!("Unable to start the server: {:?}", e);
            }
        };

        let puzzle = Puzzle::<N>::new::<SynthesisPuzzle<N, A>>();

        let server = Arc::new(Server {
            sender,
            prover_sender,
            accounting_sender,
            pool_address: address,
            connected_provers: Default::default(),
            authenticated_provers: Default::default(),
            pool_state: Arc::new(RwLock::new(PoolState::new())),
            prover_states: Default::default(),
            prover_address_connections: Default::default(),
            latest_epoch_number: AtomicU32::new(0),
            latest_epoch_hash: Default::default(),
            latest_proof_target: AtomicU64::new(u64::MAX),
            nonce_seen: Arc::new(FlurryHashSet::with_capacity(10 << 20)),
            puzzle,
        });

        // clear nonce
        {
            let nonce = server.nonce_seen.clone();
            let mut ticker = tokio::time::interval(Duration::from_secs(60));
            task::spawn(async move {
                loop {
                    ticker.tick().await;
                    nonce.pin().clear()
                }
            });
        }

        let s = server.clone();
        task::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, peer_addr)) => {
                        info!("New connection from: {}", peer_addr);
                        if let Err(e) = s.sender.send(ServerMessage::ProverConnected(stream, peer_addr)).await {
                            error!("Error accepting connection: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Error accepting connection: {:?}", e);
                    }
                }
            }
        });

        let s = server.clone();
        task::spawn(async move {
            let server = s.clone();
            while let Some(msg) = receiver.recv().await {
                let server = server.clone();
                task::spawn(async move {
                    server.process_message(msg).await;
                });
            }
        });

        server
    }

    fn seen_nonce(nonce_seen: Arc<FlurryHashSet<u64>>, nonce: u64) -> bool {
        !nonce_seen.pin().insert(nonce)
    }

    fn clear_nonce(&self) {
        self.nonce_seen.pin().clear()
    }

    pub fn send