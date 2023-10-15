
use std::{
    collections::{HashMap, VecDeque},
    fs::create_dir_all,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Error, Result};
use cache::Cache;
use dirs::home_dir;
use parking_lot::RwLock;
use savefile::{load_file, save_file};
use savefile_derive::Savefile;