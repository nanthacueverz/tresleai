# Stage 1: Create recipe for cargo chef for fast docker builds
FROM rust:latest AS chef
WORKDIR /usr/src/app
# We need to install and set as default the toolchain specified in rust-toolchain.toml
# Otherwise cargo-chef might build dependencies using wrong toolchain
# This also prevents builder steps from installing the toolchain over and over again
COPY rust-toolchain.toml rust-toolchain.toml
RUN rustup show active-toolchain || \
    (rustup install $(grep channel rust-toolchain.toml | cut -d '"' -f 2) && \
     rustup default $(grep channel rust-toolchain.toml | cut -d '"' -f 2)) && \
    cargo install cargo-chef sccache

# Prepare the recipe
COPY . .
RUN cargo chef prepare --recipe-path recipe.json
ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache

# Stage 2: Cache dependencies and build
FROM chef AS builder
WORKDIR /usr/src/app
COPY --from=chef /usr/src/app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo,from=chef,source=/usr/local/cargo \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

# Copy source code and build application
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo,from=chef,source=/usr/local/cargo \
    --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo build --release


# Stage 3: Final runtime image
FROM rust:latest as runtime
WORKDIR /usr/src/app

# Copy the built Rust binary from the builder stage
COPY --from=builder /usr/src/app/target/release/tresleai-uifacade-service tresleai-uifacade-service
# Set executable permissions during build
RUN chmod +x ./tresleai-uifacade-service
RUN wget https://truststore.pki.rds.amazonaws.com/global/global-bundle.pem -O /usr/src/app/global-bundle.pem

# Execute the default command specified in the CMD instruction
CMD ["./tresleai-uifacade-service"]
