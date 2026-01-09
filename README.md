# Totally Ordered Multicast with Lamport Clocks

A distributed system I built for my distributed systems class in Rust that implements totally ordered multicast using Lamport logical clocks. Communication between nodes is done via gRPC using Tonic.

## What it does

- Multiple peer nodes communicate via gRPC
- Each node broadcasts random words from a CSV file at random intervals (Poisson distribution)
- Messages are ordered using Lamport clocks - ensuring causally consistent delivery
- Delivery rule: a message is only delivered when we've seen later timestamps from all other peers

## Attack protection

I implemented basic defenses against malicious nodes:
- Rejects timestamp reuse (can't send same timestamp twice)
- Rejects backwards timestamps (can't go back in time)
- Rejects far-future timestamps (drift limit of 100)

The `attack_client.rs` tests these protections.

## Running

Start peers (6 peer example):

```bash
cargo run --release --bin peer 0 127.0.0.1:50051 127.0.0.1:50052 127.0.0.1:50053 127.0.0.1:50054 127.0.0.1:50055 127.0.0.1:50056
cargo run --release --bin peer 1 127.0.0.1:50051 127.0.0.1:50052 127.0.0.1:50053 127.0.0.1:50054 127.0.0.1:50055 127.0.0.1:50056
cargo run --release --bin peer 2 127.0.0.1:50051 127.0.0.1:50052 127.0.0.1:50053 127.0.0.1:50054 127.0.0.1:50055 127.0.0.1:50056
cargo run --release --bin peer 3 127.0.0.1:50051 127.0.0.1:50052 127.0.0.1:50053 127.0.0.1:50054 127.0.0.1:50055 127.0.0.1:50056
cargo run --release --bin peer 4 127.0.0.1:50051 127.0.0.1:50052 127.0.0.1:50053 127.0.0.1:50054 127.0.0.1:50055 127.0.0.1:50056
cargo run --release --bin peer 5 127.0.0.1:50051 127.0.0.1:50052 127.0.0.1:50053 127.0.0.1:50054 127.0.0.1:50055 127.0.0.1:50056
```

Format: `peer_index address1 address2 ... addressN`

Test attacks:

```bash
cargo run --release --bin evil 127.0.0.1:50051
```

Peers broadcast random words from CSV data with Lamport timestamps. Messages are delivered in total order. Evil peer tests timestamp reuse, backdating, and future timestamp attacks.
