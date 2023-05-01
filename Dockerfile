FROM rust as builder

WORKDIR /build

COPY . /build

RUN cargo build --release --bin relay

RUN ls -la /build/target/release

FROM rust as runtime

COPY --from=builder /build/target/release/relay /usr/local/bin

CMD ["relay"]
