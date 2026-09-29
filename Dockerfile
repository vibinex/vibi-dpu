# Build inside the target-platform container so Buildx can produce both amd64 and arm64 images.
FROM rust:1-bookworm AS builder

RUN apt-get update \
  && apt-get install -y --no-install-recommends libssl-dev pkg-config \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY vibi-dpu/Cargo.toml vibi-dpu/Cargo.lock ./
COPY vibi-dpu/src ./src
RUN cargo build --release --locked

# Keep the existing Ubuntu runtime and its required command-line tools/libraries.
FROM ubuntu:24.04

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates git ripgrep libssl3 \
  && rm -rf /var/lib/apt/lists/*

# Run the DPU with a stable, unprivileged identity. The fixed IDs let operators
# grant write access to bind-mounted configuration without using root.
RUN groupadd --gid 10001 dpu && \
  useradd --uid 10001 --gid dpu --home-dir /app --no-create-home --shell /usr/sbin/nologin dpu

WORKDIR /app
COPY --from=builder --chown=dpu:dpu /build/target/release/vibi-dpu /app/vibi-dpu
COPY --chown=dpu:dpu prompts /app/prompts

# The DPU persists credentials under /app/config and writes rotated logs under
# /var/log/dpu. Repository checkouts and the embedded database use /tmp.
RUN mkdir -p /app/config /var/log/dpu && \
  chown -R dpu:dpu /app /var/log/dpu

USER dpu:dpu

CMD ["/app/vibi-dpu"]
