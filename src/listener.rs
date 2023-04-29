use std::env;
use std::str;
use tokio::sync::broadcast;
use zeromq::Socket;
use zeromq::SocketRecv;

const DEFAULT_ZMQ_ADDRESS: &str = "tcp://127.0.0.1:28332";

use super::thread_id;
use super::event::BitcoinEvent;
use super::event::BitcoinEventType;

type MessageSender = broadcast::Sender<BitcoinEvent>;

pub async fn listen_bitcoin_events(tx: MessageSender) {
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
