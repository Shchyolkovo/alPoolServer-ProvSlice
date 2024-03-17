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
 