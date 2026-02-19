FROM rust:1.85 AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY lumina-types/Cargo.toml lumina-types/Cargo.toml
COPY lumina-crypto/Cargo.toml lumina-crypto/Cargo.toml
COPY lumina-execution/Cargo.toml lumina-execution/Cargo.toml
COPY lumina-storage/Cargo.toml lumina-storage/Cargo.toml
COPY lumina-consensus/Cargo.toml lumina-consensus/Cargo.toml
COPY lumina-network/Cargo.toml lumina-network/Cargo.toml
COPY lumina-node/Cargo.toml lumina-node/Cargo.toml
COPY lumina-api/Cargo.toml lumina-api/Cargo.toml
COPY lumina-cli/Cargo.toml lumina-cli/Cargo.toml
COPY lumina-genesis/Cargo.toml lumina-genesis/Cargo.toml
COPY lumina-oracles/Cargo.toml lumina-oracles/Cargo.toml

RUN mkdir -p lumina-types/src lumina-crypto/src lumina-execution/src lumina-storage/src \
  lumina-consensus/src lumina-network/src lumina-node/src lumina-api/src lumina-cli/src \
  lumina-genesis/src lumina-oracles/src

RUN printf '%s\n' 'pub fn _placeholder() {}' > lumina-types/src/lib.rs \
  && printf '%s\n' 'pub fn _placeholder() {}' > lumina-crypto/src/lib.rs \
  && printf '%s\n' 'pub fn _placeholder() {}' > lumina-execution/src/lib.rs \
  && printf '%s\n' 'pub fn _placeholder() {}' > lumina-storage/src/lib.rs \
  && printf '%s\n' 'pub fn _placeholder() {}' > lumina-consensus/src/lib.rs \
  && printf '%s\n' 'pub fn _placeholder() {}' > lumina-network/src/lib.rs \
  && printf '%s\n' 'fn main() {}' > lumina-node/src/main.rs \
  && printf '%s\n' 'pub fn _placeholder() {}' > lumina-api/src/lib.rs \
  && printf '%s\n' 'fn main() {}' > lumina-cli/src/main.rs \
  && printf '%s\n' 'pub fn _placeholder() {}' > lumina-genesis/src/lib.rs \
  && printf '%s\n' 'pub fn _placeholder() {}' > lumina-oracles/src/lib.rs

RUN cargo build --release --bin lumina-node

COPY . .

RUN cargo build --release --bin lumina-node

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 10001 -g root lumina

COPY --from=builder /app/target/release/lumina-node /usr/local/bin/lumina-node

EXPOSE 3000/tcp
EXPOSE 4000/udp

USER 10001

ENTRYPOINT ["lumina-node"]
CMD ["--validator", "--data-dir", "/data"]
