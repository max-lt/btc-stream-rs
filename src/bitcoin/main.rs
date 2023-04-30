mod peer;

use bitcoin::network::constants::ServiceFlags;
use peer::Peer;

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    let address = "127.0.0.1:8333";
    let address = std::env::var("BTC_ADDRESS").unwrap_or(address.to_string());
    let address = std::net::ToSocketAddrs::to_socket_addrs(&address)
        .unwrap()
        .next()
        .unwrap();

    let services = ServiceFlags::NETWORK;
    let network = bitcoin::Network::Bitcoin;

    // connect_to_bitcoin_node().await;
    let mut peer = Peer::new(address, services, network);

    peer.connect().await;
}
