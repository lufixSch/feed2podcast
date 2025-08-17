FROM rust:slim-trixie AS base
WORKDIR /app

RUN apt-get update && \
    apt-get upgrade --yes --no-install-recommends && \
    apt-get install --yes cmake pkg-config libssl-dev --no-install-recommends && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

FROM base AS builder

COPY . .
RUN cargo build --bin feed2podcast --release

# FROM debian:trixie-slim
FROM base

ENV FEED2PODCAST_URL="http://127.0.0.1:3000"
ENV FEED2PODCAST_DISABLE_DOCS=false
ENV FEED2PODCAST_PORT="3000"
ENV FEED2PODCAST_CACHE_DIR="/app/cache"

COPY --from=builder /app/target/release/feed2podcast ./feed2podcast

EXPOSE 3000
CMD ["./feed2podcast"]
