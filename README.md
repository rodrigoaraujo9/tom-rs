# Totally Ordered Multicast

Distributed messaging system with Lamport clocks and total ordering guarantees.

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
