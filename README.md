# scoop

A simple clean newsletter delivery service in rust.

Public Endpoints:
- **/**
- **/health_check**
- "**/login**
- **/subscriptions**

- **/admin**
  - **/dashboard**
  - **/logout**
  - **/newsletters**
  - **/password**

# Features

- Fault Tolerant: Best effort email delivery
- Concurrent Proof: Retries will not trigger duplicate newsletter entries
- Redis Store for fast session interface
- Salient Tracing and Logging 

# Pre-requisites

- Rust
- Docker

```sh
./scripts/init_db.sh      # launch the postgres docker container
./scripts/init_redis.sh   # launch the redis docker container
```

# Building

1. Using cargo
```sh
cargo build
```

2. Using the Dockerfile
```sh
docker build ./
```

# Testing

```sh
cargo test
```