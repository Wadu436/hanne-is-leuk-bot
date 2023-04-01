FROM rust:1.68.0 AS builder

RUN USER=root cargo new --lib hanne-is-leuk-bot
WORKDIR /hanne-is-leuk-bot

ENV SQLX_OFFLINE=true

# Cache dependencies
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./.cargo ./.cargo

RUN cargo build --release
RUN rm src/*.rs
RUN rm /hanne-is-leuk-bot/target/release/deps/hanne_is_leuk_bot*

# Build App
COPY ./src ./src
COPY ./migrations ./migrations
COPY ./sqlx-data.json ./sqlx-data.json
COPY ./build.rs ./build.rs
RUN cargo build --release

# Final image
FROM debian:buster-slim
WORKDIR /usr/app/

# Copy the executable
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /hanne-is-leuk-bot/target/release/hanne-is-leuk-bot /usr/app/

# Start command
CMD [ "./hanne-is-leuk-bot" ]