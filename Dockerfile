FROM rust:slim-buster as builder

WORKDIR /build

COPY . /build

RUN cargo build --release --bin relay

FROM debian:buster-slim as runtime

COPY --from=builder /build/target/release/relay /usr/local/bin

EXPOSE 7070

CMD ["relay"]
