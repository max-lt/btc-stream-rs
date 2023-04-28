use bitcoin::consensus::Decodable;
use bitcoin::network::constants::Network;
use bitcoin::Address;
use bitcoin::Transaction;
use std::env;
use std::error::Error;
use std::str;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use zeromq::Socket;
use zeromq::SocketRecv;
use zeromq::ZmqMessage;

const DEFAULT_NETWORK: Network = Network::Bitcoin;
const DEFAULT_ZMQ_ADDRESS: &str = "tcp://127.0.0.1:28332";

mod event;
use event::BitcoinEvent;

// Macro tu get thread_id as hex string
macro_rules! thread_id {
    () => {
        format!("{:?}", std::thread::current().id())
    };
}

async fn handle_message(msg: zeromq::ZmqMessage) -> Result<(), Box<dyn Error>> {
    let event = BitcoinEvent::try_from(msg)?;

    match event {
        BitcoinEvent::RawTx(raw_tx) => {
            let mut reader = std::io::Cursor::new(raw_tx);
            let tx = Transaction::consensus_decode(&mut reader)?;
            println!(
                "{:?} Tx: {:?}",
                thread_id!(),
                format!(
                    "txid: {}, vsize: {}, vout: {}, vin: {}, addresses: {}",
                    tx.txid(),
                    tx.weight() / 4,
                    tx.output.len(),
                    tx.input.len(),
                    tx.output
                        .iter()
                        .map(|output| {
                            match Address::from_script(&output.script_pubkey, DEFAULT_NETWORK) {
                                Ok(address) => address.to_string(),
                                Err(_) => String::from("[undecodable]"),
                            }
                        })
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            );
        }
        BitcoinEvent::HashTx(hash_tx) => {
            println!("{:?} HashTx: {:?}", thread_id!(), hash_tx);
        }
        BitcoinEvent::RawBlock(raw_block) => {
            println!("{:?} RawBlock: {:?}", thread_id!(), raw_block);
        }
        BitcoinEvent::HashBlock(hash_block) => {
            println!("{:?} HashBlock: {:?}", thread_id!(), hash_block);
        }
        BitcoinEvent::Unknown(unknown) => {
            println!("{:?} Unknown: {:?}", thread_id!(), unknown);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Read the environment variables for Bitcoin Core and ZeroMQ
    let address = env::var("ZMQ_ADDRESS").unwrap_or(DEFAULT_ZMQ_ADDRESS.to_string());

    println!("Connecting to {}...", address);

    // Initialize the ZeroMQ context and socket
    let mut receiver = zeromq::SubSocket::new();
    receiver.connect(address.as_str()).await?;
    receiver.subscribe("rawtx").await?;

    // Create a channel for sending and receiving messages between threads
    let (tx, mut rx): (Sender<ZmqMessage>, Receiver<ZmqMessage>) = mpsc::channel(100);

    // Start a new thread for receiving messages
    tokio::spawn(async move {
        println!("Connected to server");

        loop {
            match receiver.recv().await {
                Ok(msg) => {
                    // let thread_id = std::thread::current().id();
                    println!("{:?} -> {:?}", thread_id!(), msg.get(0).unwrap());

                    // Send the message to the channel
                    if tx.try_send(msg).is_err() {
                        println!("Error sending message to channel");
                    }
                }
                Err(e) => {
                    println!("Error receiving message: {}", e);
                }
            }
        }
    });

    // Process messages from the channel
    while let Some(msg) = rx.recv().await {
        tokio::spawn(async move {
            if let Err(e) = handle_message(msg).await {
                println!("Error handling message: {}", e);
            }
        });
    }

    Ok(())
}
