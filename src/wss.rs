use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::Result;
use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::sync::broadcast;

use crate::event::BitcoinEvent;

type MessageReceiver = broadcast::Receiver<BitcoinEvent>;

pub async fn listen_ws(rx: MessageReceiver) {
    let addr = "127.0.0.1:7070".to_string();

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream, rx.resubscribe()));
    }
}

async fn accept_connection(stream: TcpStream, rx: MessageReceiver) {
    if let Err(e) = handle_connection(stream, rx).await {
        match e {
            tungstenite::Error::ConnectionClosed => println!("Connection closed"),
            e => println!("Error: {}", e),
        }
    }
}

async fn handle_connection(stream: TcpStream, mut rx: MessageReceiver) -> Result<()> {
    println!("New WebSocket connection: {}", stream.peer_addr()?);

    let ws_stream = accept_async(stream).await.expect("Failed to accept");

    // WebSocket message sender
    let (mut tx, _) = ws_stream.split();

    while let Ok(msg) = rx.recv().await {
        println!("Received a message to broadcast: {:?}", msg);
        tx.send(tungstenite::Message::Text("Hello".to_string())).await?;
    }
    
    Ok(())
}
