FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY migrations ./migrations
COPY frontend ./frontend
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/cit-system /usr/local/bin/cit-system
COPY migrations ./migrations
ENV APP_HOST=0.0.0.0
ENV APP_PORT=8080
ENV ENABLE_WORKER=true
EXPOSE 8080
CMD ["cit-system"]
