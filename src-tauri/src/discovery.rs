use std::net::{Ipv4Addr, UdpSocket};
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use crate::node::Node;
use crate::network::{start_server, stabilize, fix_fingers, send_message}; // Importa o que estava esquecido!
use sha1::{Digest, Sha1};
use crate::message::Message;

const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(239, 255, 0, 1);
const PORT: u16 = 5000;

pub fn main() -> std::io::Result<Arc<Mutex<Node>>> {
    let node = find_node()?;

    let node_tcp = Arc::clone(&node);
    let node_bg = Arc::clone(&node);

    let my_tcp_address = {
        let node_lock = node.lock().unwrap();
        node_lock.info.address.clone()
    };

    thread::spawn(move || {
        start_server(my_tcp_address, node_tcp)
    });

    println!("[DISCOVERY] Servidor TCP iniciado e threads de manutenção lançadas");

    thread::spawn(move || {
      loop {
        thread::sleep(Duration::from_millis(500));
        stabilize(Arc::clone(&node_bg));
        thread::sleep(Duration::from_millis(500));
        fix_fingers(Arc::clone(&node_bg));
      }
    });

    thread::spawn(|| {
        if let Err(e) = listen_mult() {
            eprintln!("Erro no listener multicast: {}", e);
        }
    });

    Ok(node)
}

pub fn get_local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    
    let local_addr = socket.local_addr().ok()?;
    Some(local_addr.ip().to_string())
}

/// Aplica o SHA-1 na string do IP e retorna um u8 (0-255)
pub fn generate_id_from_ip(ip: &str) -> u8 {
    let mut hasher = Sha1::new();
    hasher.update(ip.as_bytes());
    
    // O SHA-1 retorna um array de 20 bytes. 
    // Pegamos o primeiro byte para usar no nosso anel de 8 bits.
    let result = hasher.finalize();
    result[0]
}

pub fn find_node() -> std::io::Result<Arc<Mutex<Node>>> {

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;

    let msg = b"PING_CHORD_JOIN";
    let dest_addr = format!("{}:{}", MULTICAST_IP, PORT);

    let my_ip = crate::discovery::get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());
    let my_id = crate::discovery::generate_id_from_ip(&my_ip);
    let my_tcp_addr = format!("{}:8000", my_ip);
    
    println!("Enviando ping multicast para descobrir nós...");
    socket.send_to(msg, &dest_addr)?;

    let mut buf = [0; 1024];
    
    match socket.recv_from(&mut buf) {
        Ok((len, src)) => {
            let reply = String::from_utf8_lossy(&buf[..len]);
            println!("[FINDER] Nó de contato encontrado: {} - {}", src, reply);
            
            let contact_tcp_addr = format!("{}:8000", src.ip());
            
            let node = Arc::new(Mutex::new(Node::new(my_id, my_tcp_addr)));
            
            if let Message::SuccessorResponse { node: succ } = send_message(&contact_tcp_addr, &Message::FindSuccessor { id: my_id }) {
                println!("[FINDER] O meu sucessor correto no anel será o Nó {}", succ.id);
                let mut node_lock = node.lock().unwrap();
                node_lock.successor = succ; // Atualizamos o nosso sucessor real!
            }

            Ok(node)
        }
        Err(e) => {
            println!("[FINDER] Nenhum nó respondeu... {}", e);
            println!("Iniciando anel sozinho");

            let node = Arc::new(Mutex::new(Node::new(my_id, my_tcp_addr)));
            println!("[FINDER] Nó inicializado sozinho id={}", my_id);
            Ok(node)
        }
    }
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
            println!("[LISTENER] Respondi para {}", src);
        }
    }
}