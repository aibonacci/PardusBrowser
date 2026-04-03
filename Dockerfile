# ── Build stage ──────────────────────────────────────────────────────────────
FROM debian:bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    cmake \
    ninja-build \
    python3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Cache dependencies
COPY Cargo.toml Cargo.lock* ./
COPY crates/pardus-core/Cargo.toml crates/pardus-core/Cargo.toml
COPY crates/pardus-cli/Cargo.toml crates/pardus-cli/Cargo.toml
COPY crates/pardus-debug/Cargo.toml crates/pardus-debug/Cargo.toml
COPY crates/pardus-cdp/Cargo.toml crates/pardus-cdp/Cargo.toml
COPY crates/pardus-kg/Cargo.toml crates/pardus-kg/Cargo.toml

RUN mkdir -p crates/pardus-core/src && echo "" > crates/pardus-core/src/lib.rs && \
    mkdir -p crates/pardus-cli/src && echo "fn main() {}" > crates/pardus-cli/src/main.rs && \
    mkdir -p crates/pardus-debug/src && echo "" > crates/pardus-debug/src/lib.rs && \
    mkdir -p crates/pardus-cdp/src && echo "" > crates/pardus-cdp/src/lib.rs && \
    mkdir -p crates/pardus-kg/src && echo "" > crates/pardus-kg/src/lib.rs

RUN cargo +nightly build --release 2>/dev/null || true

COPY . .
RUN touch crates/pardus-core/src/lib.rs crates/pardus-cli/src/main.rs \
      crates/pardus-debug/src/lib.rs crates/pardus-cdp/src/lib.rs crates/pardus-kg/src/lib.rs
RUN cargo +nightly build --release --bin pardus-browser

# ── Runtime stage ────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libstdc++6 \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1000 pardus \
    && useradd --uid 1000 --gid pardus --create-home pardus

WORKDIR /home/pardus

COPY --from=builder /app/target/release/pardus-browser /usr/local/bin/pardus-browser

# CDP server port
EXPOSE 9222

# Health check via CDP HTTP discovery endpoint
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -sf http://127.0.0.1:${PORT:-9222}/json/version || exit 1

USER pardus

# Default: start CDP server bound to all interfaces. Override with `docker run ... <subcommand> [args]`
ENTRYPOINT ["pardus-browser"]
CMD ["serve", "--host", "0.0.0.0"]
