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
                .parse::<u16>()
                .expect("Invalid database port"),
        );
        cfg.dbname = Some(env::var("DB_DATABASE").expect("No database name defined"));
        cfg.user = Some(env::var("DB_USER").expect("No database user defined"));
        cfg.password = Some(env::var("DB_PASSWORD").expect("No database password defined"));
        let schema = env::var("DB_SCHEMA").unwrap_or_else(|_| {
            warn!("Using schema public as default");
            "public".to_string()
        });
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Verified,
        });
        //