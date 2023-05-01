use bitcoin::consensus::Decodable;
use bitcoin::consensus::Encodable;
use bitcoin::network::address::Address;
use bitcoin::network::constants::Magic;
use bitcoin::network::constants::Network;
use bitcoin::network::constants::ServiceFlags;
use bitcoin::network::message::CommandString;
use bitcoin::network::message::NetworkMessage;
use bitcoin::network::message::RawNetworkMessage;
use bitcoin::network::message_network::VersionMessage;
use std::net::SocketAddr;
use tokio::net::TcpStream;

fn timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as u64
}

macro_rules! thread_id {
    () => {
        format!("{:?}", std::thread::current().id())
    };
}

const NONCE: u64 = 0x156463168584 as u64;

const USER_AGENT: &str = "/rust-bitcoin:0.1.0/";

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

type TcpRead = tokio::io::ReadHalf<TcpStream>;
type TcpWrite = tokio::io::WriteHalf<TcpStream>;
type Tx = tokio::sync::mpsc::Sender<NetworkMessage>;
type Rx = tokio::sync::mpsc::Receiver<NetworkMessage>;

async fn read_messages(mut tcp: TcpRead, magic: Magic, tx: Tx) {
    let mut event_id = 0;

    loop {
        event_id += 1;

        let header = &mut [0; 24];

        let _len = tcp.read_exact(header).await.unwrap();
        let msg_magic: Magic = match Decodable::consensus_decode(&mut &header[0..4]) {
            Ok(magic) => magic,
            Err(e) => {
                println!("Error decoding magic: {:?}", e);
                return;
            }
        };

        if msg_magic != magic {
            println!("Invalid magic value");
            return;
        }

        let mut msg_command = [0u8; 12];
        msg_command.copy_from_slice(&header[4..16]);
        let msg_command: CommandString = match Decodable::consensus_decode(&mut &msg_command[..]) {
            Ok(msg_command) => msg_command,
            Err(e) => {
                println!("Error decoding message command: {:?}", e);
                return;
            }
        };

        let msg_len: u32 = match Decodable::consensus_decode(&mut &header[16..20]) {
            Ok(msg_len) => msg_len,
            Err(e) => {
                println!("Error decoding message length: {:?}", e);
                return;
            }
        };

        println!(
            "{} {} Received {:?}, reading next {} bytes",
            thread_id!(),
            event_id,
            msg_command,
            msg_len
        );

        let msg_len = msg_len as usize;
        let mut payload_buf = vec![0u8; msg_len];
        let _len = tcp.read_exact(&mut payload_buf).await.unwrap();

        // header + payload
        let mut buf = vec![0u8; 24 + msg_len];
        buf[..24].copy_from_slice(&header[..]);
        buf[24..].copy_from_slice(&payload_buf[..]);
        let mut cursor = std::io::Cursor::new(&buf);

        let message = match RawNetworkMessage::consensus_decode(&mut cursor) {
            Ok(message) => message,
            Err(e) => {
                println!("Error decoding message: {:?}", e);
                return;
            }
        };

        if message.magic != magic {
            println!("Invalid magic value");
            return;
        }

        tx.send(message.payload).await.unwrap();
    }
}

/// Respond to messages received from the channel
async fn send_messages(mut tcp: TcpWrite, magic: Magic, mut rx: Rx) {
    while let Some(message) = rx.recv().await {
        match message {
            NetworkMessage::Ping(nonce) => {
                let message = NetworkMessage::Pong(nonce);
                send_message(&mut tcp, magic, message).await.unwrap();
            }
            NetworkMessage::Version(message) => {
                let message = NetworkMessage::Verack;
                send_message(&mut tcp, magic, message).await.unwrap();
            }
            NetworkMessage::Addr(message) => {}
            NetworkMessage::Verack => {
                println!("Received verack");
            }
            NetworkMessage::Inv(inv) => {
                println!("Received inv: {}", inv.len());
                let message = NetworkMessage::GetData(inv);
                send_message(&mut tcp, magic, message).await.unwrap();
            }
            NetworkMessage::Tx(tx) => {
                println!("Received tx: {}", tx.txid().to_string());
            }
            message => {
                println!("Received message: [{}]", message.cmd());
            }
        }
    }
}

async fn send_message(
    tcp: &mut TcpWrite,
    magic: Magic,
    payload: NetworkMessage,
) -> Result<(), std::io::Error> {
    let cmd = payload.cmd();

    let raw = RawNetworkMessage { magic, payload };

    let mut data = Vec::new();
    raw.consensus_encode(&mut data).unwrap();

    tcp.write_all(&data).await?;

    println!("-> Sent {} message", cmd);

    Ok(())
}

pub struct Peer {
    address: SocketAddr,
    services: ServiceFlags,
    last_seen: u64,
    last_attempt: u64,
    attempts: u32,
    network: Network,
}

impl Peer {
    pub fn new(address: SocketAddr, services: ServiceFlags, network: Network) -> Self {
        Self {
            address,
            services,
            last_seen: 0,
            last_attempt: 0,
            attempts: 0,
            network,
        }
    }

    pub async fn connect(&mut self) {
        self.last_attempt = timestamp();
        self.attempts += 1;

        let network = self.network;
        let magic = network.magic();

        let address = self.address;

        println!("Connecting to Bitcoin node at {}", address);

        // Establish a TCP connection
        let stream = TcpStream::connect(address).await.unwrap();

        let version_message = self.version_message();

        // Create channel exchange bitcoin messages
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        // Split the stream into a reader and a writer
        let (srx, mut stx) = tokio::io::split(stream);

        // Send the version message
        send_message(&mut stx, magic, NetworkMessage::Version(version_message))
            .await
            .unwrap();

        // Read channel messages to write into the TCP stream
        let h1 = tokio::task::spawn(read_messages(srx, magic, tx));

        // Reads TCP stream and sends messages to the channel
        let h2 = tokio::task::spawn(send_messages(stx, magic, rx));

        // Wait for both tasks to complete
        tokio::try_join!(h1, h2).unwrap();

        println!("Connection closed");
    }

    fn version_message(&self) -> VersionMessage {
        let services = ServiceFlags::NONE;
        let address = self.address.clone();

        VersionMessage {
            version: 70015,
            services,
            timestamp: timestamp() as i64,
            receiver: Address::new(&address, services),
            // sender is only dummy
            sender: Address::new(&address, services),
            nonce: NONCE,
            user_agent: USER_AGENT.to_string(),
            start_height: 787670,
            relay: true,
        }
    }
}
