#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


mod discovery;
mod node;
mod network;
mod message;

fn main() -> std::io::Result<()> {

    std::thread::spawn(|| {
        if let Err(e) = discovery::main() {
            eprintln!("Erro na descoberta de rede: {}", e);
        }
    });

    
    bitchord_lib::run();
    Ok(())
}