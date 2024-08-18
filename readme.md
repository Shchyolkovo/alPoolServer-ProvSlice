# alPoolServer-ProvSlice

## Introduction

This repository hosts a standalone Proving Pool Server tailored for the Aleo Network, now under the stewardship of Shchyolkovo.

## Why a Standalone Server?

This project was developed during the Testnet2 period for the following reasons

1. Improve separation between the mining pool and the network node. This ensures that ledger syncing does not interfere with mining pool operations.
2. Enable the use of an optimized network protocol that enhances pool-miner communication.
3. Avoid overhauling snarkOS code. This helps maintain sync with upstream code.
4. Facilitate testing by using a standalone server for the mining pool.
5. Simplify feature additions by utilizing a smaller codebase