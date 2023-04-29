use tokio::sync::broadcast;

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
    let (tx, rx) = broadcast::channel(10);

    // Start a new thread for receiving messages
    let t1 = tokio::spawn(listen_bitcoin_events(tx));

    // let (ws_tx, ws_rx) = tokio::sync::mpsc::channel(100);
    let t2 = tokio::spawn(listen_ws(rx));

    tokio::try_join!(t1, t2)?;

    Ok(())
}
