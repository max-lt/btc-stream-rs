# Bitcoin transaction relay

Relay transactions from Bitcoin Core (ZeroMQ) to Websocket.

## Build

```bash
cargo build --release --bin relay
```

## Run

```bash
./target/release/relay
```

## Use with docker

```bash
docker run --rm -it -p 7070:7070 -e ZMQ_ADDRESS=tcp://127.0.0.1:28332 --name btc-relay ghcr.io/max-lt/btc-stream-rust
```
