# scoop

A simple clean newsletter delivery service in Rust.
Done as a part of reading ["Zero To Production In Rust"](HTTP://www.zero2prod.com), a book on API development using Rust.

Public Endpoints:
| Endpoint            |     Method        |
| --------------------| ----------------- |
| /                   | **GET**           |
| /health_check       | **GET**           |
| /login              | **GET**           |
| /subscriptions      | **GET**/**POST**  |
| /unsubscribe        | **POST**          |
| /admin/dashboard    | **GET**           |
| /admin/logout       | **POST**          |
| /admin/newsletters  | **GET**/**POST**  |
| /admin/password     | **GET**/**POST**  |

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

# ToDo
- fix broken css rendering and file serving
- 
# Testing

```sh
cargo test
```
