# Rust Execution Environment - Optimized for Cold Start
FROM rust:1.75-slim

# Set environment variables for performance
ENV CARGO_HOME=/usr/local/cargo \
    RUSTUP_HOME=/usr/local/rustup \
    PATH=/usr/local/cargo/bin:$PATH \
    RUSTFLAGS="-C opt-level=0 -C debuginfo=0"

WORKDIR /code

# Pre-install common dependencies at build time
RUN cargo install cargo-script || true

# Create non-root user for security
RUN useradd -m -u 1000 optimus && \
    chown -R optimus:optimus /code

USER optimus

# Note: Rust execution currently uses cargo-script or similar runtime compilation
# For production, consider pre-compiled execution environments
# Minimal entrypoint (no shell overhead)
ENTRYPOINT ["/bin/sh", "-c", "echo 'Rust execution not yet implemented'"]
