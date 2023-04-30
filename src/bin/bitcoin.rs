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
use tokio::net::TcpStream;

fn timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

macro_rules! thread_id {
    () => {
        format!("{:?}", std::thread::current().id())
    };
}

const NONCE: u64 = 0x15646316584 as u64;

const USER_AGENT: &str = "/rust-bitcoin:0.1.0/";

async fn connect_to_bitcoin_node() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // let dns = super::dns::Dns::new();

    let network = Network::Bitcoin;
    let magic = network.magic();
    let address = "127.0.0.1:8333";
    let address = std::env::var("BTC_ADDRESS").unwrap_or(address.to_string());

    println!("Connecting to Bitcoin node at {}", address);

    // Establish a TCP connection
    let stream = TcpStream::connect(address).await.unwrap();

    let address = std::net::SocketAddr::from(stream.peer_addr().unwrap());

    let services = ServiceFlags::NONE;

    // Send a version message
    let version_message = NetworkMessage::Version(VersionMessage {
        version: 70015,
        services,
        timestamp: timestamp(),
        receiver: Address::new(&address, ServiceFlags::NETWORK),
        // sender is only dummy
        sender: Address::new(&address, ServiceFlags::NETWORK),
        nonce: NONCE,
        user_agent: USER_AGENT.to_string(),
        start_height: 787478,
        relay: false,
    });

    // Create channel exchange bitcoin messages
    let (tx, rx) = tokio::sync::mpsc::channel(32);

    // Split the stream into a reader and a writer
    let (srx, mut stx) = tokio::io::split(stream);

    send_message(&mut stx, magic, version_message).await.unwrap();

    println!("-> Sent version message");

    // Read channel messages to write into the TCP stream
    let h1 = tokio::task::spawn(read_messages(srx, magic, tx));

    // Reads TCP stream and sends messages to the channel
    let h2 = tokio::task::spawn(send_messages(stx, magic, rx));

    tokio::try_join!(h1, h2).unwrap();

    println!("Connection closed");
}

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

        println!("{} {} Received message, reading header...", thread_id!(), event_id);
        let len = tcp.read_exact(header).await.unwrap();

        if len == 0 {
            return;
        }

        let msg_magic: Magic = match Decodable::consensus_decode(&mut &header[0..4]) {
            Ok(magic) => magic,
            Err(e) => {
                println!("Error decoding magic: {:?}", e);
                return;
            }
        };

        println!("{} {} Magic: {:?} {:?}", thread_id!(), event_id, msg_magic, magic);

        let mut msg_command = [0u8; 12];
        msg_command.copy_from_slice(&header[4..16]);
        let msg_command: CommandString = match Decodable::consensus_decode(&mut &msg_command[..]) {
            Ok(msg_command) => msg_command,
            Err(e) => {
                println!("Error decoding message command: {:?}", e);
                return;
            }
        };

        println!("{} {} Message command: {}", thread_id!(), event_id, msg_command);

        let msg_len: u32 = match Decodable::consensus_decode(&mut &header[16..20]) {
            Ok(msg_len) => msg_len,
            Err(e) => {
                println!("Error decoding message length: {:?}", e);
                return;
            }
        };

        println!("{} {} Message length: {}", thread_id!(), event_id, msg_len);

        let msg_len = msg_len as usize;
        let mut payload_buf = vec![0u8; msg_len];
        let len = tcp.read_exact(&mut payload_buf).await.unwrap();

        println!("{} {} Reading next {} bytes", thread_id!(), event_id, len);

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

        println!("{} {} Received message: {:?}", thread_id!(), event_id, message);

        tx.send(message.payload).await.unwrap();
    }
}

async fn send_messages(mut tcp: TcpWrite, magic: Magic, mut rx: Rx) {
    while let Some(message) = rx.recv().await {
        match message {
            NetworkMessage::Ping(nonce) => {
                // println!("Received Ping message with nonce: {}", nonce);

                let message = NetworkMessage::Pong(nonce);
                send_message(&mut tcp, magic, message).await.unwrap();

                println!("-> Sent Pong message");
            }
            NetworkMessage::Version(message) => {
                // println!("Received version message: {:?}", message);

                let message = NetworkMessage::Verack;
                send_message(&mut tcp, magic, message).await.unwrap();

                println!("-> Sent Verack message");
            }
            message => {
                // println!("Received message: [{}] {:?}", message.cmd(), message);
            }
        }
    }
}

async fn send_message(
    tcp: &mut TcpWrite,
    magic: Magic,
    payload: NetworkMessage,
) -> Result<(), std::io::Error> {
    let raw = RawNetworkMessage { magic, payload };

    let mut data = Vec::new();
    raw.consensus_encode(&mut data).unwrap();

    return tcp.write_all(&data).await;
}

#[tokio::main]
async fn main() {
    connect_to_bitcoin_node().await;
}
