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
        // println!("[NETWORK] Mensagem recebida: {:?}", message);

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

            Message::PutData { key_id, file_hash, value } => {
                let mut n = node.lock().unwrap();
                println!("[STORAGE] Nó {} armazenando chave {} (ID Chord: {})", n.id, file_hash, key_id);
                n.storage.insert(file_hash, value);
                Message::Ack
            }

            Message::GetData { key_id, file_hash } => {
                let n = node.lock().unwrap();
                println!("[STORAGE] Nó {} consultando chave {} (ID Chord: {})", n.id, file_hash, key_id);
                let value = n.storage.get(&file_hash).cloned();
                Message::DataResponse { value }
            }

            Message::UpdateSuccessor { node: new_succ } => {
                println!("[LEAVE] Meu sucessor saiu! Meu novo sucessor agora é o ID {}", new_succ.id);
                let mut n = node.lock().unwrap();
                n.successor = new_succ;
                Message::Ack
            }
            Message::UpdatePredecessor { node: new_pred } => {
                println!("[LEAVE] Meu predecessor saiu! Meu novo predecessor foi atualizado.");
                let mut n = node.lock().unwrap();
                n.predecessor = new_pred;
                Message::Ack
            }
            Message::GetAllFiles { origin_id, mut files } => {
                let (my_id, succ_addr) = {
                    let n = node.lock().unwrap();
                    
                    // Coloca todos os arquivos deste nó 
                    for val in n.storage.values() {
                        files.push(val.clone());
                    }
                    (n.id, n.successor.address.clone())
                };

                // Se deu a volta completa e voltou
                if my_id == origin_id {
                    println!("[SEARCH] A varredura deu a volta completa no anel");
                    Message::AllFilesResponse { files }
                } else {
                    // Se não sou eu o dono da busca, passo a caixa pro meu sucessor
                    println!("[SEARCH] Nó {} adicionou seus arquivos e passou para o sucessor", my_id);
                    match send_message(&succ_addr, &Message::GetAllFiles { origin_id, files }) {
                        // Quando a resposta final voltar, eu devolvo pra trás na corrente
                        Message::AllFilesResponse { files: final_files } => {
                            Message::AllFilesResponse { files: final_files }
                        }
                        _ => Message::Ack, // Fallback em caso de erro
                    }
                }
            }
            
            Message::RequestChunk { file_hash, chunk_index } => {
                println!("[SEEDER] Alguém pediu o bloco {} do arquivo {}", chunk_index, file_hash);

                let file_path = {
                    let n = node.lock().unwrap();
                    n.seeding_files.get(&file_hash).cloned()
                };

                match file_path {
                    Some(path) => {
                        use std::io::{Seek, SeekFrom};
                        match std::fs::File::open(&path) {
                            Ok(mut f) => {
                                let offset = (chunk_index * crate::torrent::CHUNK_SIZE) as u64;
                                if f.seek(SeekFrom::Start(offset)).is_ok() {
                                    let mut buffer = vec![0u8; crate::torrent::CHUNK_SIZE];
                                    match f.read(&mut buffer) {
                                        Ok(bytes_read) => {
                                            buffer.truncate(bytes_read);
                                            Message::ChunkData { data: buffer }
                                        }
                                        Err(_) => Message::Ack,
                                    }
                                } else {
                                    Message::Ack
                                }
                            }
                            Err(_) => Message::Ack,
                        }
                    }
                    None => {
                        println!("[SEEDER] Não tenho esse arquivo localmente!");
                        Message::Ack
                    }
                }
            }

            Message::TransferKeys { data } => {
                let mut n = node.lock().unwrap();
                println!("[STORAGE] Nó {} recebendo {} chaves migradas de um vizinho.", n.id, data.len());
                
                // Mescla os metadados recebidos no storage local
                for (key, value) in data {
                    n.storage.insert(key, value);
                }
                Message::Ack
            }

            _ => Message::Ack,

        };

        let serialized = serde_json::to_vec(&response).unwrap();
        // println!("[NETWORK] Respondendo: {:?}", response);
        let _ = stream.write_all(&serialized);
    }
}

pub fn send_message(address: &str, message: &Message) -> Message {
    match TcpStream::connect(address) {
        Ok(mut stream) => {
            // println!("[NETWORK] Conectando em {} para enviar {:?}", address, message);
            let payload = serde_json::to_vec(message).unwrap();
            let _ = stream.write_all(&payload);

            let mut buffer = [0; 4096];
            if let Ok(size) = stream.read(&mut buffer) {
                if size > 0 {
                    // println!("[NETWORK] Recebido resposta de {}: {} bytes", address, size);
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
    // println!("[FIND] Procurando sucessor para id={} (meu id={})", id, self_id);

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
        // println!("[FIND] Closest é eu mesmo; retornando sucessor local id={}", succ.id);
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
    // println!("[STABILIZE] Perguntando predecessor do sucessor em {}", succ_address);

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
    // println!("[STABILIZE] Notificando sucessor {} sobre mim (id={})", succ_address, self_info.id);
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
    // println!("[FIX] Atualizando finger index {}", next);

    let self_id = node_arc.lock().unwrap().id;
    // Cálculo do artigo: (n + 2^i) mod 256
    let target = self_id.wrapping_add(2u8.pow(next as u32));

    let succ = find_successor_rpc(node_arc.clone(), target);
    // println!("[FIX] Sucessor para target {} é id={}", target, succ.id);

    let mut node = node_arc.lock().unwrap();
    if node.fingers.len() <= next {
        // CORREÇÃO DO BORROW CHECKER: Clonamos o valor para uma variável local primeiro
        let default_info = node.info.clone();
        node.fingers.resize(8, default_info);
    }
    node.fingers[next] = succ;
}

pub fn leave_ring(node_arc: Arc<Mutex<Node>>) {
    let (my_info, pred_opt, succ, my_storage) = {
        let node = node_arc.lock().unwrap();
        // Extraímos o storage atual para migração
        (node.info.clone(), node.predecessor.clone(), node.successor.clone(), node.storage.clone())
    };

    // Se eu não estava sozinho no anel, faço a migração e costuro as pontas
    if succ.id != my_info.id {
        println!("[LEAVE] Informando vizinhos sobre a desconexão...");

        // Transfere as chaves de metadados para o sucessor
        if !my_storage.is_empty() {
            println!("[LEAVE] Migrando {} arquivos para o sucessor ID {}...", my_storage.len(), succ.id);
            let _ = send_message(&succ.address, &Message::TransferKeys { data: my_storage });
        }
        
        // Avisa o Predecessor para pular diretamente para o meu Sucessor
        if let Some(pred) = pred_opt.clone() {
            let _ = send_message(&pred.address, &Message::UpdateSuccessor { node: succ.clone() });
        }

        // Avisa o Sucessor para olhar para o meu Predecessor
        let _ = send_message(&succ.address, &Message::UpdatePredecessor { node: pred_opt });
    }

    // Atualiza o próprio estado para isolado (standalone)
    let mut node = node_arc.lock().unwrap();
    node.successor = node.info.clone(); // Aponta o sucessor para si mesmo
    node.predecessor = None;            // Limpa o predecessor
    node.storage.clear();               // Limpa o storage local já que foi migrado
    println!("[LEAVE] Desconexão concluída. O nó agora está isolado.");
}