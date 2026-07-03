// src/torrent.rs
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::Read;
use std::path::Path;
// Definindo o tamanho do pedaço (ex: 256 KB)
pub const CHUNK_SIZE: usize = 256 * 1024; 
use std::sync::{Arc, Mutex};
use crate::node::Node;
use crate::network::send_message;
use crate::message::Message;
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TorrentMeta {
    pub file_name: String,
    pub total_size: u64,
    pub file_hash: String,         // O SHA-1 do nome do arquivo (A chave que vai pro Chord)
    pub routing_id: u8,            // A chave (0-255) que vai pro Chord
    pub chunk_hashes: Vec<String>, // Lista com o SHA-1 de cada pedaço de 256KB
    pub seeders: Vec<String>,
}

// Estrutura do arquivo.bitchord
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)] 
pub struct BitChordFile {
    pub version: u32,
    pub file_name: String,
    pub file_size: u64,
    pub file_hash: String,
    pub piece_size: usize,
    pub piece_hashes: Vec<String>,
}


fn save_bitchord(meta: &TorrentMeta) -> Result<(), String> {
    let bitchord = BitChordFile {
        version: 1,
        file_name: meta.file_name.clone(),
        file_size: meta.total_size, 
        file_hash: meta.file_hash.clone(),
        piece_size: CHUNK_SIZE,     
        piece_hashes: meta.chunk_hashes.clone(), 
    };
    
    let bitchord_json = serde_json::to_string_pretty(&bitchord)
        .map_err(|e| format!("Erro ao gerar JSON do BitChord: {}", e))?;
    
    // Diretório relativo ao projeto
    let dir_path = Path::new("BitChordFiles");
    
    // Cria pasta se não existir
    std::fs::create_dir_all(dir_path)
        .map_err(|e| format!("Erro ao criar pasta BitChordFiles: {}", e))?;
    
    // Monta o caminho  "BitChordFiles/nome_do_arquivo.bitchord"
    let bitchord_path = dir_path.join(&meta.file_name).with_extension("bitchord");
    
    // Escreve o arquivo no disco
    std::fs::write(&bitchord_path, bitchord_json)
        .map_err(|e| format!("Erro ao salvar o arquivo .bitchord: {}", e))?;
    
    println!("Arquivo salvo com sucesso em {:?}", bitchord_path);
    Ok(())
}


pub fn create_torrent_meta(file_path: &str, file_name: &str) -> std::io::Result<TorrentMeta> {
    let mut file = File::open(file_path)?;
    let metadata = file.metadata()?;
    let total_size = metadata.len();

    let mut chunk_hashes = Vec::new();
    let mut buffer = vec![0; CHUNK_SIZE];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break; // Fim do arquivo
        }

        let mut hasher = Sha1::new();
        hasher.update(&buffer[..bytes_read]);
        let hash_result = hex::encode(hasher.finalize());

        chunk_hashes.push(hash_result);
    }


    let mut name_hasher = Sha1::new();
    name_hasher.update(file_name.as_bytes());
    
    let finalized_hash = name_hasher.finalize(); 

    let file_hash = hex::encode(finalized_hash); // Hexadecimal string
    let routing_id = finalized_hash[0];          // extract first byte (0-255)

    Ok(TorrentMeta {
        file_name: file_name.to_string(),
        total_size,
        file_hash,
        routing_id,
        chunk_hashes,
        seeders: Vec::new(),
    })
}

#[tauri::command]
pub fn upload_file(file: String, state: tauri::State<'_, std::sync::Arc<std::sync::Mutex<crate::node::Node>>>) -> Result<(), String> {
    println!("[TORRENT] Iniciando upload/processamento do arquivo: {}", file);

    let path = Path::new(&file);
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("arquivo_desconhecido");

    match create_torrent_meta(&file, file_name) {
        Ok(mut meta) => {
            println!("[TORRENT] Metadados gerados com sucesso!");
            println!("  Hash Completo: {}", meta.file_hash);
            
            // 1. MAPEAMENTO: Converte os 2 primeiros caracteres hex do SHA-1 para um u8 (0 a 255)
            // Exemplo: se o hash começa com "a3...", vira o ID Chord 163.
            let key_id = u8::from_str_radix(&meta.file_hash[..2], 16)
                .map_err(|e| format!("Erro ao mapear hash para ID Chord: {}", e))?;
            
            println!("[TORRENT] ID de busca gerado para o anel Chord: {}", key_id);

            // 2. BUSCA DISTRIBUÍDA: Encontra o nó responsável
            let node_arc = state.inner().clone();
            println!("[TORRENT] Roteando busca pelo nó responsável no anel...");
            let target_node = crate::network::find_successor_rpc(node_arc.clone(), key_id);
            println!("[TORRENT] Nó responsável encontrado: ID {} no endereço {}", target_node.id, target_node.address);


            // Pega o IP do nó e adiciona na lista de seeders
            let my_address = {
                let n = node_arc.lock().unwrap();
                n.info.address.clone()
            };
            meta.seeders.push(my_address);
           

            // 3. SERIALIZAÇÃO: Converte a struct TorrentMeta para uma String JSON
            let serialized_meta = serde_json::to_string(&meta)
                .map_err(|e| format!("Erro ao serializar metadados: {}", e))?;

            // 4. ENVIO: Despacha os dados para o nó correto via mensagem PutData
            let put_msg = crate::message::Message::PutData {
                key_id,
                file_hash: meta.file_hash.clone(),
                value: serialized_meta,
            };

            match crate::network::send_message(&target_node.address, &put_msg) {
                crate::message::Message::Ack => {
                    save_bitchord(&meta)?;
                    println!("[TORRENT] SUCESSO! Metadados publicados e salvos no Nó {}!", target_node.id);
                    Ok(())
                }
                _ => Err(format!("O Nó {} não confirmou o salvamento do arquivo.", target_node.id)),
            }
        }
        Err(e) => {
            let erro_msg = format!("Falha ao processar arquivo: {}", e);
            eprintln!("[TORRENT] {}", erro_msg);
            Err(erro_msg)
        }
    }
}



// Alterar para baixar do .bitchord, não mexi aqui ainda.
// O upload já está salvando o .bitchord na pasta BitchordFiles
// Temos agora que fazer a lógica de download ao receber um arquivo .bitchord
// Esta função é antiga e não sei se funciona...
#[tauri::command]
pub fn download_file(file_meta: TorrentMeta, state: tauri::State<'_, Arc<Mutex<Node>>>) -> Result<(), String> {
    println!("[DOWNLOAD] Iniciando download de: {}", file_meta.file_name);
    // print pra cada coisa de file_meta
    println!("[DOWNLOAD] Nome do arquivo: {}", file_meta.file_name);
    println!("[DOWNLOAD] Tamanho total: {} bytes", file_meta.total_size);
    println!("[DOWNLOAD] Hash do arquivo: {}", file_meta.file_hash);
    for seeder in &file_meta.seeders {
        println!("[DOWNLOAD] Seeder disponível: {}", seeder);
    }
    
    if file_meta.seeders.is_empty() {
        return Err("Nenhum seeder disponível para este arquivo.".to_string());
    }

    let target_ip = &file_meta.seeders[0];
    println!("[DOWNLOAD] Conectando direto ao seeder: {}", target_ip);
    
    // Lógica para abrir TCP e pedir os bytes 
    
    Ok(())
}

