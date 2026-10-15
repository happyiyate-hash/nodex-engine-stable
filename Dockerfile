FROM rust:1.98-bookworm AS builder

WORKDIR /app
COPY . .

RUN cargo build --release --bin dev_ui

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/dev_ui /app/dev_ui
COPY --from=builder /app/static /app/static

ENV RUST_LOG=info
ENV PORT=3030

EXPOSE 3030

CMD ["/app/dev_ui"]
