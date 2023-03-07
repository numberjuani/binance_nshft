FROM rust:1.67.1-buster

WORKDIR /usr/src/app
COPY . .
RUN apt-get update && apt-get install -y libssl-dev llvm-dev libclang-dev clang
RUN cargo build --release

CMD ["target/release/binance-nshft"]