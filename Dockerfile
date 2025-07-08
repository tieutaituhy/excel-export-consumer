# --- Stage 1: Build the Rust application ---
FROM rust:1.79-slim-bookworm AS builder

# Set working directory inside the container
WORKDIR /app

# Copy Cargo.toml and Cargo.lock first to leverage Docker layer caching
# This step helps speed up subsequent builds if dependencies don't change
COPY Cargo.toml .
COPY Cargo.lock .

# If you have specific build features, add them here
# For example, if you enabled 'xlsxwriter' feature
# RUN mkdir src/ && echo 'fn main() {}' > src/main.rs && cargo build --release --features xlsxwriter --dry-run
# Remove the above line and uncomment the actual build command below after testing dry-run
# The dry-run is just to download dependencies and cache them
RUN mkdir src/ && echo 'fn main() {}' > src/main.rs && \
    if grep -q 'xlsxwriter' Cargo.toml; then \
        cargo build --release --features xlsxwriter --dry-run; \
    else \
        cargo build --release --dry-run; \
    fi

# Copy all source code
COPY . .

# Build the Rust application in release mode
# Use `CARGO_NET_GIT_FETCH_WITH_CLI=true` if you encounter issues with git dependencies
# Use `CARGO_HOME=/usr/local/cargo` if you have permission issues
RUN if grep -q 'xlsxwriter' Cargo.toml; then \
        cargo build --release --features xlsxwriter; \
    else \
        cargo build --release; \
    fi

# --- Stage 2: Create the final runtime image ---
FROM debian:bookworm-slim

# Install necessary runtime dependencies
# For PostgreSQL: libpq5, For other DBs, you might need different libraries (e.g., libmysqlclient-dev for MySQL)
RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/excel-export-consumer .

# Copy the .env file (if you manage env vars inside the container)
# In production, it's better to pass environment variables directly to Docker run or Kubernetes secrets
COPY .env .

# Create directory for Excel exports (ensure it's writable)
RUN mkdir -p /app/exports && chmod -R 777 /app/exports

# Expose the metrics port (if you are using Prometheus metrics)
EXPOSE 9000

# Command to run the application
# Use `RUST_LOG=info` or adjust as needed for desired logging level
CMD ["./excel-export-consumer"]