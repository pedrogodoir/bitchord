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
pub fn download_file(filepath: String, state: tauri::State<'_, Arc<Mutex<Node>>>) -> Result<(), String> {
    println!("[DOWNLOAD] Lendo arquivo bitchord em: {}", filepath);

    // LER O ARQUIVO LOCAL
    let file_content = std::fs::read_to_string(&filepath)
        .map_err(|e| format!("Erro ao ler o arquivo .bitchord: {}", e))?;

    let bitchord: BitChordFile = serde_json::from_str(&file_content)
        .map_err(|e| format!("Erro ao decodificar o .bitchord: {}", e))?;

    println!("[DOWNLOAD] Arquivo carregado: {} (Hash: {})", bitchord.file_name, bitchord.file_hash);

    // DESCOBRIR A CHAVE CHORD (Mapeamento do Hash)
    let key_id = u8::from_str_radix(&bitchord.file_hash[..2], 16)
        .map_err(|e| format!("Erro ao mapear hash para ID Chord: {}", e))?;

    println!("[DOWNLOAD] Buscando na DHT os metadados para a chave {}", key_id);

    // ENCONTRAR O NÓ RESPONSÁVEL (Guardião dos Metadados)
    let node_arc = state.inner().clone();
    let target_node = crate::network::find_successor_rpc(node_arc.clone(), key_id);

    println!("[DOWNLOAD] Nó responsável pela chave {} é o ID {} no endereço {}", key_id, target_node.id, target_node.address);

    // RESGATAR A TorrentMeta (Que contém os seeders)
    let get_msg = crate::message::Message::GetData {
        key_id,
        file_hash: bitchord.file_hash.clone(),
    };

    let response = crate::network::send_message(&target_node.address, &get_msg);

    // PROCESSAR A RESPOSTA DA REDE
    let meta_json = match response {
        crate::message::Message::DataResponse { value: Some(data) } => data,
        crate::message::Message::DataResponse { value: None } => {
            return Err(format!("Arquivo {} não está mais disponível na DHT.", bitchord.file_name));
        }
        _ => {
            return Err("Resposta inesperada ao consultar a DHT.".to_string());
        }
    };

    let meta: TorrentMeta = serde_json::from_str(&meta_json)
        .map_err(|e| format!("Erro ao processar metadados recebidos da rede: {}", e))?;

    // INICIAR DOWNLOAD COM OS SEEDERS
    if meta.seeders.is_empty() {
        return Err("Nenhum seeder ativo encontrado para este arquivo.".to_string());
    }

    println!("[DOWNLOAD] Metadados recuperados! Seeders disponíveis:");
    for seeder in &meta.seeders {
        println!("  -> {}", seeder);
    }

    // MEXER NISSO AQUI DEPOIS
    let target_ip = &meta.seeders[0]; // Pegando o primeiro seeder da lista (Pode ser melhorado no futuro)
    println!("[DOWNLOAD] Conectando direto ao seeder: {}", target_ip);
    
    use std::net::TcpStream;
    use std::io::{Write, Read};


    // ABRE A CONEXÃO TCP COM O SEEDER
    // O target_ip já deve ser algo como "192.168.1.50:8000"
    let mut stream = TcpStream::connect(target_ip)
        .map_err(|e| format!("Falha ao conectar no seeder {}: {}", target_ip, e))?;

    println!("[DOWNLOAD] Conectado! Pedindo o bloco 0...");

    // MONTA O PEDIDO
    let request = crate::message::Message::RequestChunk {
        file_hash: meta.file_hash.clone(),
        chunk_index: 0, // Pedindo o primeiro bloco
    };

    let req_json = serde_json::to_string(&request)
        .map_err(|e| format!("Erro ao serializar pedido de chunk: {}", e))?;

    // ENVIA O PEDIDO PARA A STREAM
    stream.write_all(req_json.as_bytes())
        .map_err(|e| format!("Erro ao enviar pedido: {}", e))?;

    // LÊ A RESPOSTA (O pedaço do arquivo)
    // O servidor vai mandar um JSON (Message::ChunkData) com os bytes, ou fechar a conexão.
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer)
        .map_err(|e| format!("Erro ao ler resposta do seeder: {}", e))?;

    // DESERIALIZA OS DADOS RECEBIDOS
    match serde_json::from_str::<crate::message::Message>(&buffer) {
        Ok(crate::message::Message::ChunkData { data }) => {
            println!("[DOWNLOAD] Sucesso! Recebidos {} bytes do chunk 0.", data.len());
            
            // salvar esses bytes em um arquivo final local usando std::fs::write
            // ou dando um 'append' (std::fs::OpenOptions) se for um arquivo muito grande.
            std::fs::write(&meta.file_name, &data)
                .map_err(|e| format!("Erro ao salvar arquivo no disco: {}", e))?;
        }
        _ => return Err("Seeder respondeu com um formato inválido ou recusou o pedido.".to_string()),
    }
    
    Ok(())
}

