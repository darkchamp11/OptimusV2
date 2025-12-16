# Optimus

A scalable code execution engine with auto-scaling capabilities using Rust, Redis, Docker, and KEDA.

## Architecture

- **optimus-api**: HTTP gateway for submitting code execution jobs (Axum)
- **optimus-worker**: Execution engine that processes jobs from Redis queue (Bollard + Docker)
- **optimus-cli**: Management tool for adding languages and generating templates
- **optimus-common**: Shared types, Redis logic, and configuration

## Getting Started

### Prerequisites

- Rust toolchain (via rustup)
- Docker
- kubectl
- Redis instance (local or cluster)
- Optional: cargo-watch

### Build

```bash
cargo build --release
```

### Run

```bash
# API Server
cargo run -p optimus-api

# Worker
cargo run -p optimus-worker

# CLI
cargo run -p optimus-cli -- --help
```

## Project Structure

```
optimus/
├── bins/       # Binary crates (api, worker, cli)
├── libs/       # Shared library crates
├── config/     # Configuration files
├── k8s/        # Kubernetes manifests
└── dockerfiles/ # Language execution environments
```

## License

TBD
