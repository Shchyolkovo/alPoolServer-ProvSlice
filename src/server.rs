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
        