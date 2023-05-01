use tokio::sync::broadcast;
use tokio::sync::mpsc;

mod event;

mod wss;
use wss::listen_ws;

mod listener;
use listener::listen_bitcoin_events;

#[macro_export]
macro_rules! thread_id {
    () => {
        format!("{:?}", std::thread::current().id())
    };
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Create a channel for sending and receiving messages between threads
    let (tx, rx) = broadcast::channel(100);

    // Create a channel for sending connection updates
    let (conn_updates_tx, conn_updates_rx) = mpsc::channel(100);

    // Start a new thread for receiving messages
    let t1 = tokio::spawn(listen_bitcoin_events(tx, conn_updates_rx));

    // Start a new thread for listening for WebSocket connections
    let t2 = tokio::spawn(listen_ws(rx, conn_updates_tx));

    tokio::try_join!(t1, t2)?;

    Ok(())
}
