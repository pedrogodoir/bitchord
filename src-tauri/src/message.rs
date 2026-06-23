use serde::{Deserialize, Serialize};
use crate::node::NodeInfo;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Message {
    Ping,
    Pong,
    FindSuccessor { id: u8 },
    SuccessorResponse { node: NodeInfo },
    GetPredecessor,
    PredecessorResponse { node: Option<NodeInfo> },
    Notify { node: NodeInfo },
    Ack,
    PublishFile { file_id: u8, file_hash: String, owner_address: String },
}