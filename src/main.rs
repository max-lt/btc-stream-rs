use std::env;
use std::error::Error;
use std::str;
use tokio::sync::broadcast;
use zeromq::Socket;
use zeromq::SocketRecv;

const DEFAULT_ZMQ_ADDRESS: &str = "tcp://127.0.0.1:28332";

mod event;
use event::BitcoinEvent;

mod wss;
use wss::listen_ws;

use crate::event::BitcoinEventType;

type MessageSender = broadcast::Sender<BitcoinEvent>;

// Macro tu get thread_id as hex string
macro_rules! thread_id {
    () => {
        format!("{:?}", std::thread::current().id())
    };
}

async fn listen_bitcoin_events(tx: MessageSender) {
    // Read the environment variables for Bitcoin Core and ZeroMQ
    let zmq_address = env::var("ZMQ_ADDRESS").unwrap_or(DEFAULT_ZMQ_ADDRESS.to_string());

    println!("Connecting to {}...", zmq_address);

    // Initialize the ZeroMQ context and socket
    let mut receiver = zeromq::SubSocket::new();
    receiver.connect(zmq_address.as_str()).await.unwrap();
    receiver.subscribe("rawtx").await.unwrap();

    println!("Connected to server");

    loop {
        match receiver.recv().await {
            Ok(msg) => {
                // let thread_id = std::thread::current().id();
                println!("{:?} -> {:?}", thread_id!(), msg.get(0).unwrap());

                let result = BitcoinEvent::try_from(msg);

                let decoded = match result {
                    Ok(e) if e.event_type() == BitcoinEventType::RawTx => e,
                    Ok(decoded) => {
                        println!("Skipping message: {:?}", decoded.event_type());
                        continue;
                    }
                    Err(e) => {
                        println!("Error decoding message: {}", e);
                        continue;
                    }
                };

                // Send the message to the channel
                if tx.send(decoded).is_err() {
                    println!("Error sending message to channel");
                }
            }
            Err(e) => {
                println!("Error receiving message: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Create a channel for sending and receiving messages between threads
    let (tx, rx) = broadcast::channel(10);

    // Start a new thread for receiving messages
    let t1 = tokio::spawn(async { listen_bitcoin_events(tx).await });

    // let (ws_tx, ws_rx) = tokio::sync::mpsc::channel(100);
    let t2 = tokio::spawn(async { listen_ws(rx).await });

    tokio::try_join!(t1, t2)?;

    Ok(())
}
