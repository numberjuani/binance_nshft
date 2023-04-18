FROM rust:1.68.2-buster AS builder

WORKDIR /usr/src/app
RUN apt-get update && apt-get upgrade -y
COPY . .
RUN apt-get update && apt-get install -y libssl-dev llvm-dev libclang-dev clang
RUN cargo build --release

FROM rust:1.68.2-buster AS runner
# Copy the build artifact from the build stage
COPY --from=builder /usr/src/app/target/release/binance_nshft /usr/local/bin/binance_nshft
#copy .env
COPY --from=builder /usr/src/app/.env /usr/.env
CMD ["/usr/local/bin/binance_nshft"]