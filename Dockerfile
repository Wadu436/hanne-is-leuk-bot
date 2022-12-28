FROM rust:1.59.0 AS builder

RUN USER=root cargo new --bin hanne-is-leuk-bot
WORKDIR /hanne-is-leuk-bot

# Cache dependencies
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --bin hanne-is-leuk-bot --release
RUN rm src/*.rs
RUN rm /hanne-is-leuk-bot/target/release/deps/namr1_stonks_bot*

# Build App
COPY ./src ./src
RUN cargo build --bin hanne-is-leuk-bot --release

# Final image
FROM debian:buster-slim
WORKDIR /usr/app/

# Copy the executable
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /hanne-is-leuk-bot/target/release/hanne-is-leuk-bot /usr/app/

# Start command
CMD [ "./hanne-is-leuk-bot" ]