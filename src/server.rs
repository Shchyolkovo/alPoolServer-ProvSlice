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
            speed_2m: Speed