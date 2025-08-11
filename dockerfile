FROM rust:slim AS builder
WORKDIR /app

RUN apt-get update && apt-get upgrade && apt-get install --yes cmake pkg-config libssl-dev
COPY . .
RUN cargo build --bin feed2podcast --release

FROM debian:trixie-slim

ENV FEED2PODCAST_URL="http://127.0.0.1:3000"
ENV FEED2PODCAST_DISABLE_DOCS=false
ENV FEED2PODCAST_PORT="3000"
ENV FEED2PODCAST_SHARED_DIR="/app/static"
ENV FEED2PODCAST_CACHE_DIR="/app/cache"

WORKDIR /app

COPY --from=builder /app/target/release/feed2podcast ./feed2podcast
COPY static /app/static

EXPOSE 3000
CMD ["./feed2podcast"]
