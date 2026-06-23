// src/torrent.rs
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::Read;
use std::path::Path;

// Definindo o tamanho do pedaço (ex: 256 KB)
pub const CHUNK_SIZE: usize = 256 * 1024; 

#[derive(Clone, Debug, serde::Serialize)] 
pub struct TorrentMeta {
    pub file_name: String,
    pub total_size: u64,
    pub file_hash: String,         // O SHA-1 do nome do arquivo (A chave que vai pro Chord)
    pub chunk_hashes: Vec<String>, // Lista com o SHA-1 de cada pedaço de 256KB
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
    let file_hash = hex::encode(name_hasher.finalize());

    Ok(TorrentMeta {
        file_name: file_name.to_string(),
        total_size,
        file_hash,
        chunk_hashes,
    })
}

#[tauri::command]
pub fn upload_file(file: String) -> Result<(), String> {
    println!("[TORRENT] Iniciando upload/processamento do arquivo: {}", file);

    // Extrai o nome do arquivo a partir do caminho completo
    let path = Path::new(&file);
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("arquivo_desconhecido");


    match create_torrent_meta(&file, file_name) {
        Ok(meta) => {
            println!("[TORRENT] Sucesso! Metadados gerados:");
            println!("  Nome: {}", meta.file_name);
            println!("  Tamanho: {} bytes", meta.total_size);
            println!("  Hash da Chave: {}", meta.file_hash);
            println!("  Quantidade de Pedaços: {}", meta.chunk_hashes.len());
            
            // TODO:Lógica de publicar no chord.
            // Fluxo no excalidraw...

            Ok(())
        }
        Err(e) => {
            let erro_msg = format!("Falha ao processar arquivo: {}", e);
            eprintln!("[TORRENT] {}", erro_msg);
            Err(erro_msg)
        }
    }
}