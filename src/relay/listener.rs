use std::env;
use std::str;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use zeromq::Socket;
use zeromq::SocketRecv;

const DEFAULT_ZMQ_ADDRESS: &str = "tcp://127.0.0.1:28332";

use super::event::BitcoinEvent;
use super::event::BitcoinEventType;
use super::thread_id;

type MessageSender = broadcast::Sender<BitcoinEvent>;
type StateReceiver = mpsc::Receiver<bool>;

pub async fn listen_bitcoin_events(tx: MessageSender, mut rx: StateReceiver) {
    let zmq_address = env::var("ZMQ_ADDRESS").unwrap_or(DEFAULT_ZMQ_ADDRESS.to_string());

    println!("Connecting to {}...", zmq_address);

    // Initialize the ZeroMQ context and socket
    let mut receiver = zeromq::SubSocket::new();
    receiver.connect(zmq_address.as_str()).await.unwrap();

    println!("Connected to server");

    let mut listeners_count = 0 as usize;

    loop {
        tokio::select! {
              // Listen for messages from bitcoin node
              event = receiver.recv() => {
                  match event {
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
                              Err(err) => {
                                  println!("Error decoding message: {}", err);
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
                },
              // Listen for messages from the WebSocket connection
              event = rx.recv() => {
                  match event {
                      // One less listener
                      Some(false) => {
                        listeners_count -= 1;

                        if listeners_count == 0 {
                            println!("No more WebSocket connections, unsubscribing from ZeroMQ socket");
                            receiver.unsubscribe("rawtx").await.unwrap();
                        }
                      },
                      // One more listener
                      Some(true) => {
                        listeners_count += 1;

                        if listeners_count == 1 {
                          println!("First WebSocket connection, subscribing to ZeroMQ socket");
                          receiver.subscribe("rawtx").await.unwrap();
                        }
                      },
                      None => {
                          println!("Channel closed");
                          break;
                      }
                  }
              }
        }
    }
}
