# alPoolServer-ProvSlice

## Introduction

This repository hosts a standalone Proving Pool Server tailored for the Aleo Network, now under the stewardship of Shchyolkovo.

## Why a Standalone Server?

This project was developed during the Testnet2 period for the following reasons

1. Improve separation between the mining pool and the network node. This ensures that ledger syncing does not interfere with mining pool operations.
2. Enable the use of an optimized network protocol that enhances pool-miner communication.
3. Avoid overhauling snarkOS code. This helps maintain sync with upstream code.
4. Facilitate testing by using a standalone server for the mining pool.
5. Simplify feature additions by utilizing a smaller codebase.

## Features

This server provides key functionalities, including:

1. An implementation of the Stratum protocol. [Refer to the Specs](stratum/spec.md).
2. An adept automatic difficulty targeting system (More testing needed under high load).
3. Comprehensive stats for the pool and provers.

## Project Status

Continuous improvements and added features:

- RDBMS utilization instead of RocksDB for most data storage.
- Implementation of a proper stratum protocol for pool-miner communication.
- Modification of the difficulty targeting system.
- Evaluation of the need for more A