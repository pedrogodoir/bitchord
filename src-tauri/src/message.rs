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
    UpdateSuccessor { node: NodeInfo },
    UpdatePredecessor { node: Option<NodeInfo> },
    PutData { key_id: u8, file_hash: String, value: String },
    GetData { key_id: u8, file_hash: String },
    DataResponse { value: Option<String> },
    GetAllFiles { origin_id: u8, files: Vec<String> }, 
    AllFilesResponse { files: Vec<String> },
}