use std::net::{Ipv4Addr, UdpSocket};
use std::thread;
use std::time::Duration;


const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(239, 255, 0, 1);
const PORT: u16 = 5000;

pub fn main() -> std::io::Result<()> {
    if let Err(e) = find_node() {
        eprintln!("Erro ao buscar nó: {}", e);
    }
    thread::sleep(Duration::from_millis(500));
    let _listener_handle = thread::spawn(|| {
        if let Err(e) = listen_mult() {
            eprintln!("Erro no listener multicast: {}", e);
        }
    });
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}


pub fn find_node() -> std::io::Result<()> {

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;

    let msg = b"PING_CHORD_JOIN";
    let dest_addr = format!("{}:{}", MULTICAST_IP, PORT);
    
    println!("Enviando ping multicast para descobrir nós...");
    socket.send_to(msg, &dest_addr)?;

    let mut buf = [0; 1024];
    
    match socket.recv_from(&mut buf) {
        Ok((len, src)) => {
            let reply = String::from_utf8_lossy(&buf[..len]);
            println!("[FINDER] Sucesso! Nó de contato encontrado: {} - Mensagem: {}", src, reply);
            // Pedir find_successor via RPC
        }
        Err(e) => {
            println!("[FINDER] Nenhum nó respondeu... {}", e);
            println!("Iniciando anel sozinho");
            // inicializar anel sozinho
        }
    }

    Ok(())
}

// Nó já existente fica ouvindo esperando algum nó novo
pub fn listen_mult() -> std::io::Result<()> {
    let local_addr = Ipv4Addr::new(0, 0, 0, 0);
    let socket = UdpSocket::bind(format!("{}:{}", local_addr, PORT))?;
    
    socket.join_multicast_v4(&MULTICAST_IP, &local_addr)?;

    println!("[LISTENER] Escutando pedidos de entrada em {}:{}", MULTICAST_IP, PORT);

    let mut buf = [0; 1024];
    loop {
        let (len, src) = socket.recv_from(&mut buf)?;
        let msg = String::from_utf8_lossy(&buf[..len]);
        
        if msg == "PING_CHORD_JOIN" {
            println!("[LISTENER] Recebi pedido de entrada de {}", src);
            let reply = b"HELLO_I_AM_CONTACT_NODE";
            socket.send_to(reply, src)?;
        }
    }
}