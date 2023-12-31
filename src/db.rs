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
    connection_pool: Pool,
}

impl DB {
    pub fn init() -> DB {
        let mut cfg = Config::new();
        cfg.host = Some(env::var("DB_HOST").expect("No database host defined"));
        cfg.port = Some(
            env::var("DB_PORT")
                .unwrap_or_else(|_| "5432".to_string())
               