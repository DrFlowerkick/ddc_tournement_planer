# Builder stage
# We use the latest Rust stable release as base image
FROM lukemathwalker/cargo-chef:latest-rust-1.78.0 AS chef
# Let's switch our working directory to `app` (equivalent to `cd app`)
# The `app` folder will be created for us by Docker in case it does not
# exist already.
WORKDIR /app
# Install the required system dependencies for our linking configuration
RUN apt update && apt install lld clang -y

FROM chef as planner
# Copy all files from our working environment to our Docker image
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependenciy tree stays the same,
# all layers should be cached
COPY . .

# Use prepared .sqlx files for sqlx data base checks
ENV SQLX_OFFLINE true
# Let's build our binary!
# We'll use the release profile to make it faaaast
RUN cargo build --release --bin ddc_tournement_planer

# Runtime stage
FROM debian:bookworm-slim AS runtime
# set workdir again
WORKDIR /app
# Install OpenSSL - it is dynamically linked by some of our dependencies
# Install ca-certificates - it is needed to verify TLS certificates
# when establishing HTTPS connections
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
# Copy the compiled binary from builder environment to our runtime environment
COPY --from=builder /app/target/release/ddc_tournement_planer ddc_tournement_planer
# We need the configuration files at runtime!
COPY configuration configuration
# We need the migration files at runtime!
COPY migrations migrations
# set environment to production
ENV APP_ENVIRONMENT production
# When `docker run` is executed, launch the binary!
ENTRYPOINT ["./ddc_tournement_planer"]
