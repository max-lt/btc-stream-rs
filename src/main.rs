use bitcoin::network::constants::Network;
use std::env;
use std::str;
use std::error::Error;
use tokio::select;
use zeromq::Socket;
use zeromq::SocketRecv;

const DEFAULT_NETWORK: Network = Network::Bitcoin;
const DEFAULT_ZMQ_ADDRESS: &str = "tcp://127.0.0.1:28332";

mod event;
use event::BitcoinEvent;

// Define the structure to store the Bitcoin event data

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Read the environment variables for Bitcoin Core and ZeroMQ
    let address = env::var("ZMQ_ADDRESS").unwrap_or(DEFAULT_ZMQ_ADDRESS.to_string());

    println!("Connecting to {}", address);

    // Initialize the ZeroMQ context and socket

    let mut receiver = zeromq::SubSocket::new();
    receiver.connect(address.as_str()).await?;
    receiver.subscribe("").await?;

    println!("Connected to server");

    // Process messages from Bitcoin Core
    loop {
        select! {
            message = receiver.recv() => {
                match message {
                    Ok(msg) => {
                        println!("Received message: {:?}", msg.len());

                        let event = BitcoinEvent::try_from(msg)?;

                        println!("Received {:?}: {:?}", event.get_type(), event);
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
        };
    }
}
