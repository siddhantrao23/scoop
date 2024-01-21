# scoop

A simple clean newsletter delivery service in rust.

Public Endpoints:
- '/': Homepage
- '/health_check'
- '/login'
- '/subscriptions' 

- '/admin'
  - '/dashboard'
  - '/logout'
  - '/newsletters'
  - '/password'

# Features

- Fault Tolerant: Best effort email delivery
- Concurrent Proof: Retries will not trigger duplicate newsletter entries
- Redis Store for fast session interface
- Salient Tracing and Logging 