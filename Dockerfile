# Leveraging the pre-built Docker images with
# cargo-chef and the Rust toolchain
FROM lukemathwalker/cargo-chef:latest-rust-1.68.0 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release


FROM debian:bullseye-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/postrs /app/target/release/postrs

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates

EXPOSE 4000
CMD ["/app/target/release/postrs", "serve", "--bind", "0.0.0.0:4000"]
