FROM rust:1.88.0 AS chef
RUN cargo install cargo-chef
WORKDIR /usr/src/app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /usr/src/app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
ARG EXTRA_CARGO_BUILD_FLAGS
RUN cargo build --bin my-site-web --release $EXTRA_CARGO_BUILD_FLAGS

FROM ubuntu:24.04 AS runner
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/my-site-web /usr/local/bin/my-site-web
ENTRYPOINT ["my-site-web"]
