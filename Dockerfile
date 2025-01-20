FROM rust:latest

WORKDIR /usr/src/auth-api
COPY . .

RUN cargo build --release
CMD ["./target/release/auth-api"]