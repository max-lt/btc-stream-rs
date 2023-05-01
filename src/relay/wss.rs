use bitcoin::consensus::Decodable;
use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::Result;

use super::event::BitcoinEvent;
use super::thread_id;

type MessageReceiver = tokio::sync::broadcast::Receiver<BitcoinEvent>;
type StateSender = tokio::sync::mpsc::Sender<bool>;

pub async fn listen_ws(rx: MessageReceiver, tx: StateSender) {
    let addr = "127.0.0.1:7070".to_string();

    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let rx = rx.resubscribe();
        let tx = tx.clone();
        tokio::spawn(async move {
            tx.send(true).await.unwrap();
            accept_connection(stream, rx).await;
            tx.send(false).await.unwrap();
        });
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
    let (mut tx_ws, mut rx_ws) = ws_stream.split();

    loop {
        tokio::select! {
            // Listen for messages from the client
            msg = rx_ws.next() => {
                match msg {
                    Some(Ok(_)) => {
                        // Handle or ignore the client message here if needed
                    },
                    Some(Err(e)) => {
                        eprintln!("WebSocket error: {}", e);
                        break;
                    },
                    None => {
                        println!("Client closed the connection");
                        break;
                    },
                }
            },
            // Listen for messages from the broadcast receiver
            msg = rx.recv() => {
                match msg {
                    Ok(BitcoinEvent::RawTx(data)) => {
                        let mut reader = std::io::Cursor::new(data);
                        let transaction = match bitcoin::Transaction::consensus_decode(&mut reader) {
                            Ok(tx) => tx,
                            Err(e) => {
                                println!("Error decoding transaction: {}", e);
                                continue;
                            }
                        };

                        println!("{:?} Sending tx: {}", thread_id!(), transaction.txid().to_string());
                        tx_ws.send(tungstenite::Message::Text(transaction.txid().to_string())).await?;
                    },
                    Ok(event) => {
                        println!("{:?} Skipping message: {:?}", thread_id!(), event.event_type());
                    },
                    Err(e) => {
                        println!("{:?} Error receiving message: {}", thread_id!(), e);
                    },
                }
            },
        }
    }

    Ok(())
}
