pub mod discovery;
pub mod node;
pub mod network;
pub mod message;

use std::sync::{Arc, Mutex};
use serde::Serialize;
use node::Node;

#[derive(Serialize)]
pub struct NodeView {
    id: u8,
    address: String,
    successor_id: u8,
    successor_address: String,    // <-- ADICIONADO
    predecessor_id: Option<u8>,
    predecessor_address: Option<String>, // <-- ADICIONADO
}

#[tauri::command]
fn get_node_info(state: tauri::State<'_, Arc<Mutex<Node>>>) -> Result<NodeView, String> {
    let node = state.lock().map_err(|e| format!("Erro ao travar mutex: {}", e))?;
    
    Ok(NodeView {
        id: node.id,
        address: node.info.address.clone(),
        successor_id: node.successor.id,
        successor_address: node.successor.address.clone(), // <-- ADICIONADO (pega o IP:porta real)
        predecessor_id: node.predecessor.as_ref().map(|p| p.id),
        predecessor_address: node.predecessor.as_ref().map(|p| p.address.clone()), // <-- ADICIONADO (pega o IP:porta real)
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("Iniciando protocolo Chord...");
    let chord_node = match discovery::main() {
        Ok(node) => node,
        Err(e) => {
            eprintln!("Erro crítico ao iniciar o Chord: {}", e);
            panic!("Falha ao iniciar o Chord");
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(chord_node)
        .invoke_handler(tauri::generate_handler![get_node_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}