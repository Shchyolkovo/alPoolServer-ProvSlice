#[macro_use]
extern crate criterion;

use std::sync::Arc;
use criterion::Criterion;
use flurry::HashSet;
use rand::thread_rng;
use snarkvm::dpc::testnet2::Testnet2;
use snarkvm::prelude::Network;
use snarkvm::utilities::UniformRand;

fn fake_nonce() -> String {
    let nonce: <Testnet2 as Network>::PoSWNonce = UniformRand::rand(&mut thread_rng());
    nonce.to_string()
}

fn se