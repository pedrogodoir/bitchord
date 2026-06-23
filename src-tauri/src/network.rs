use crate::message::Message;
use crate::node::{Node, NodeInfo};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

// Função matemática para gerenciar o anel circular modular do Chord (0-255)
pub fn is_between(val: u8, start: u8, end: u8, inclusive_end: bool) -> bool {
    if inclusive_end && val == end {
        return true;
    }
    if start == end {
        return true; // Se start == end, o anel só tem 1 nó, então engloba tudo
    }
    if start < end {
        val > start && val < end
    } else {
        val > start || val < end
    }
}

pub fn start_server(address: String, node: Arc<Mutex<Node>>) {
    let listener = TcpListener::bind(&address).expect("Erro ao dar bind");
    println!("[NETWORK] TCP server escutando em {}", address);

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let node_clone = Arc::clone(&node);
            thread::spawn(move || {
                handle_connection(stream, node_clone);
            });
        }
    }
}

fn handle_connection(mut stream: TcpStream, node: Arc<Mutex<Node>>) {
    let mut buffer = [0; 4096];
    if let Ok(size) = stream.read(&mut buffer) {
        if size == 0 {
            return;
        }
        let message: Message = match serde_json::from_slice(&buffer[..size]) {
            Ok(msg) => msg,
            Err(_) => return,
        };
        println!("[NETWORK] Mensagem recebida: {:?}", message);

        let response = match message {
            Message::Ping => Message::Pong,

            Message::FindSuccessor { id } => {
                let succ = find_successor_rpc(Arc::clone(&node), id);
                Message::SuccessorResponse { node: succ }
            }

            Message::GetPredecessor => {
                let node_lock = node.lock().unwrap();
                Message::PredecessorResponse {
                    node: node_lock.predecessor.clone(),
                }
            }

            Message::Notify { node: n_prime } => {
                let mut node_lock = node.lock().unwrap();
                // n' pensa que é nosso predecessor (Algoritmo da seção 4.3)
                if node_lock.predecessor.is_none()
                    || is_between(
                        n_prime.id,
                        node_lock.predecessor.as_ref().unwrap().id,
                        node_lock.id,
                        false,
                    )
                {
                    node_lock.predecessor = Some(n_prime);
                }
                Message::Ack
            }

            Message::PublishFile { file_id, file_hash, owner_address } => {
                println!("[TRACKER] Guardando arquivo {} (Chave {}) do dono {}", file_hash, file_id, owner_address);
                
                let mut node_lock = node.lock().unwrap();
                
                // Pega a lista de IPs que têm esse arquivo (ou cria uma lista nova se for o primeiro)
                let entry = node_lock.tracker_data.entry(file_hash).or_insert_with(Vec::new);
                
                // Tenta adicionar ip na lista
                if !entry.contains(&owner_address) {
                    entry.push(owner_address);
                }
                
                Message::Ack 
            }

            _ => Message::Ack,




        };

        let serialized = serde_json::to_vec(&response).unwrap();
        println!("[NETWORK] Respondendo: {:?}", response);
        let _ = stream.write_all(&serialized);
    }
}

pub fn send_message(address: &str, message: &Message) -> Message {
    match TcpStream::connect(address) {
        Ok(mut stream) => {
            println!("[NETWORK] Conectando em {} para enviar {:?}", address, message);
            let payload = serde_json::to_vec(message).unwrap();
            let _ = stream.write_all(&payload);

            let mut buffer = [0; 4096];
            if let Ok(size) = stream.read(&mut buffer) {
                if size > 0 {
                    println!("[NETWORK] Recebido resposta de {}: {} bytes", address, size);
                    return serde_json::from_slice(&buffer[..size]).unwrap_or(Message::Ack);
                }
            }
            Message::Ack
        }
        Err(_) => Message::Ack, // Retorna Ack genérico caso o nó esteja offline temporalmente
    }
}

// Algoritmo Central de Busca do Artigo: Encontra o sucessor de um ID de forma distribuída
pub fn find_successor_rpc(node_arc: Arc<Mutex<Node>>, id: u8) -> NodeInfo {
    let (self_id, succ) = {
        let node = node_arc.lock().unwrap();
        (node.id, node.successor.clone())
    };
    println!("[FIND] Procurando sucessor para id={} (meu id={})", id, self_id);

    // Se o id está entre mim e meu sucessor, o responsável é o meu sucessor!
    if is_between(id, self_id, succ.id, true) {
        return succ;
    }

    // Caso contrário, busca na Finger Table pelo nó mais próximo que antecede o ID procurado
    let closest = {
        let node = node_arc.lock().unwrap();
        let mut found = node.info.clone();
        for finger in node.fingers.iter().rev() {
            if is_between(finger.id, node.id, id, false) {
                found = finger.clone();
                break;
            }
        }
        found
    };

    if closest.id == self_id {
        println!("[FIND] Closest é eu mesmo; retornando sucessor local id={}", succ.id);
        return succ;
    }

    // Encaminha a requisição via rede TCP para o nó mais próximo encontrado
    match send_message(&closest.address, &Message::FindSuccessor { id }) {
        Message::SuccessorResponse { node } => node,
        _ => succ,
    }
}

// Executado periodicamente para verificar o sucessor imediato e se apresentar a ele
pub fn stabilize(node_arc: Arc<Mutex<Node>>) {
    let succ_address = {
        let node = node_arc.lock().unwrap();
        node.successor.address.clone() // <-- Removido o self_id que não estava sendo usado aqui
    };
    println!("[STABILIZE] Perguntando predecessor do sucessor em {}", succ_address);

    // Pergunta ao sucessor quem é o predecessor dele
    let response = send_message(&succ_address, &Message::GetPredecessor);
    if let Message::PredecessorResponse { node: Some(x) } = response {
        let mut node = node_arc.lock().unwrap();
        // Se o predecessor do meu sucessor estiver mais perto de mim, ele vira meu novo sucessor
        if is_between(x.id, node.id, node.successor.id, false) {
            node.successor = x;
        }
    }

    // Avisa o sucessor da nossa existência para ele se atualizar
    let (succ_address, self_info) = {
        let node = node_arc.lock().unwrap();
        (node.successor.address.clone(), node.info.clone())
    };
    println!("[STABILIZE] Notificando sucessor {} sobre mim (id={})", succ_address, self_info.id);
    send_message(&succ_address, &Message::Notify { node: self_info });
}

// Executado periodicamente para atualizar uma entrada aleatória/sequencial da finger table
pub fn fix_fingers(node_arc: Arc<Mutex<Node>>) {
    let next = {
        // <-- Removido o 'mut' daqui para eliminar o warning
        let mut node = node_arc.lock().unwrap();
        node.next_finger = (node.next_finger + 1) % 8; // Anel de tamanho 8 bits (0-255)
        node.next_finger
    };
    println!("[FIX] Atualizando finger index {}", next);

    let self_id = node_arc.lock().unwrap().id;
    // Cálculo do artigo: (n + 2^i) mod 256
    let target = self_id.wrapping_add(2u8.pow(next as u32));

    let succ = find_successor_rpc(node_arc.clone(), target);
    println!("[FIX] Sucessor para target {} é id={}", target, succ.id);

    let mut node = node_arc.lock().unwrap();
    if node.fingers.len() <= next {
        // CORREÇÃO DO BORROW CHECKER: Clonamos o valor para uma variável local primeiro
        let default_info = node.info.clone();
        node.fingers.resize(8, default_info);
    }
    node.fingers[next] = succ;
}