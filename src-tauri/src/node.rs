use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeInfo {
    pub id: u8,
    pub address: String,
}

pub struct Node {
    pub id: u8,
    pub info: NodeInfo,
    pub successor: NodeInfo, // Todo nó começa conhecendo pelo menos si mesmo
    pub predecessor: Option<NodeInfo>,
    pub fingers: Vec<NodeInfo>,
    pub next_finger: usize, // Contador usado pelo fix_fingers
    pub tracker_data: HashMap<String, Vec<String>>, // Dados dos torrents
}

impl Node {
    pub fn new(id: u8, address: String) -> Self {
        let info = NodeInfo {
            id,
            address: address.clone(),
        };
        println!("[NODE] Criando nó id={} address={}", id, address);
        Self {
            id,
            info: info.clone(),
            successor: info, // Inicializa apontando para si mesmo
            predecessor: None,
            fingers: vec![],
            next_finger: 0,
            tracker_data: HashMap::new(),
        }
    }
}