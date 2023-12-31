use std::{collections::HashMap, env};

use anyhow::Result;
use deadpool_postgres::{
    ClientWrapper,
    Config,
    Hook,
    HookError,
    Manager,
    ManagerConfig,
    Pool,
    RecyclingMethod,
    Runtime,
};
use snarkvm::ledger::puzzle::SolutionID;
use tokio_postgres::NoTls;
use tracing::warn;

use crate::N;

pub struct DB {
    connectio