use json_rpc_types::{Error, Id};

use crate::codec::ResponseParams;

pub enum StratumMessage {
    /// This first version doesn't support vhosts.
    /// (id, user_agent, protocol_version, session_id)
    Subscribe(Id, String, String, Option<String>),

    /// (id, worker_name, worker_password)
    Authorize(Id, String, String),

    /// This is the difficulty target for the next job.
    /// (difficulty_target)
    SetTarget(u64),

    /// New job from the proving pool.
    /// See protocol specification for details about the fields.
    /// (job_id, epoch_hash, address, clean_jobs)
    Notify(String, String, Option<String>, bool)