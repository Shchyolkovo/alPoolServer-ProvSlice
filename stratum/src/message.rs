use json_rpc_types::{Error, Id};

use crate::codec::ResponseParams;

pub enum StratumMessage {
    /// This first version doesn't support vhosts.
    /// (id, user_agent, protocol_version, session_id)
    Subscribe(Id, String, String, Option<String>),

    /// (id, worker_name, worker_password)
    Authorize(Id, String, String),

    /// This is the di