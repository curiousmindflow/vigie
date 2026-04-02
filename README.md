# Vigie

<p align="center">
  <img src=".github/images/banner.png" alt="Vigie" width="50%"/>
</p>

<p align="center">
  <a href="https://github.com/curiousmindflow/vigie/actions/workflows/ci.yml">
    <img src="https://github.com/curiousmindflow/vigie/actions/workflows/ci.yml/badge.svg" alt="CI"/>
  </a>
  <a href="https://curiousmindflow.github.io/vigie/coverage/">
    <img src="https://img.shields.io/endpoint?url=https://curiousmindflow.github.io/vigie/coverage/badge.json" alt="Coverage"/>
  </a>
  <a href="https://curiousmindflow.github.io/vigie/mutants/">
    <img src="https://img.shields.io/endpoint?url=https://curiousmindflow.github.io/vigie/mutants/badge.json" alt="Mutation score"/>
  </a>
  <a href="LICENSE-APACHE">
    <img src="https://img.shields.io/badge/license-Apache--2.0%2FMIT-blue" alt="License"/>
  </a>
</p>

---

**Vigie** is a correctness-first SWIM protocol implementation in Rust. The protocol core (`vigie-core`) is a pure, sans-IO, `no_std`-compatible state machine. A Tokio-based realization layer (`vigie`) is provided for convenience, but anyone can build their own — with any async runtime, a custom executor, or none at all.

> **Status:** Experimental — active development, not yet production-ready.

---

## What is SWIM?

SWIM (Scalable Weakly-consistent Infection-style Membership) is a distributed membership and failure detection protocol designed for large clusters. Unlike heartbeat-based approaches, SWIM uses randomized probing and epidemic dissemination to:

- detect node failures with bounded false-positive rate,
- propagate membership updates (join, leave, suspect, confirm) via gossip piggybacked on probe messages,
- scale sub-linearly in message load with cluster size.

See the [original SWIM paper](docs/SWIM.pdf) for the formal specification.

---

## Why Vigie?

**Sans-IO architecture.** `vigie-core` never touches a socket, timer, or thread. It receives messages, advances state, and returns `Effect` values for the caller to execute. This makes the protocol logic fully deterministic and independently testable at any scale.

**`no_std`-compatible core.** `vigie-core` has no dependency on the standard library beyond `rand`. It can be embedded in custom executors, WASM runtimes, or bare-metal environments.

**Bring your own realization.** The Tokio-based `vigie` crate is one possible realization. The core is runtime-agnostic: you can wire `vigie-core` to any transport and timer mechanism — synchronous, async, embedded, or simulated.

**Correctness-first.** Every meaningful branch in `vigie-core` is subjected to unit tests, property-based tests, mutation testing, and fuzzing. The protocol is verified against its behavioral invariants, not just its happy path.

---

## Quick Start

```toml
# Cargo.toml
[dependencies]
vigie-core = { git = "https://github.com/curiousmindflow/vigie" }
```

```rust
use vigie_core::{BuiltinEffectStore, BuiltinMemberStore, Member, VigieBuilder};

let local = Member::new_ipv4([0, 0, 0, 0, 0, 0, 0, 0, 127, 0, 0, 1], 9000);
let seed  = Member::new_ipv4([0, 0, 0, 0, 0, 0, 0, 0, 127, 0, 0, 1], 9001);

let mut vigie = VigieBuilder::new(local, BuiltinMemberStore::new(), BuiltinEffectStore::new())
    .seed(seed)
    .k(3)                        // fanout for indirect probes
    .period(1000)                // probe period in ms
    .timeout(500)                // direct probe timeout in ms
    .suspicion_timeout(1500)     // time before a suspect is confirmed dead
    .build()
    .unwrap();

// Drive the protocol with your own clock and I/O loop:
//   vigie.start_period(now);
//   while let Some(effect) = vigie.pop_effect() { /* execute effect */ }
//   vigie.ingest(message);
```

---

## Architecture

Vigie is split into two crates:

| Crate        | Role                                                           | Runtime                    |
| ------------ | -------------------------------------------------------------- | -------------------------- |
| `vigie-core` | Protocol logic: state machine, membership table, gossip buffer | None (`no_std`-compatible) |
| `vigie`      | Tokio-based realization: UDP transport, timers, Kameo actors   | Tokio async                |

The core is driven by the caller. You supply the clock (`now: i64`), feed incoming messages via `ingest()`, and drain outgoing actions via `pop_effect()`. The `Effect` enum describes everything the core wants to do — send a Ping, schedule a timeout, etc. — without doing any of it.

```
                   ┌───────────────────────────────────┐
  ingest(msg) ───► │           vigie-core               │ ──► pop_effect() → Effect
  start_period() ► │  (pure state machine, no I/O)      │
                   └───────────────────────────────────┘
                                   ▲
                        your realization layer
              (Tokio · smol · bare-metal · simulation · ...)
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for a full breakdown of traits, data structures, and the effect model.

---

## Testing Strategy

The test suite is layered to catch different classes of bugs independently.

| Layer          | Tool             | What it verifies                                                                  |
| -------------- | ---------------- | --------------------------------------------------------------------------------- |
| Unit           | `rstest`         | Individual state transitions and edge cases                                       |
| Property-based | `proptest`       | Protocol invariants hold across arbitrary inputs                                  |
| Mutation       | `cargo-mutants`  | Every meaningful branch is covered by a test that would fail if the logic changes |
| Fuzz           | `cargo-fuzz`     | Safety and correctness under malformed or adversarial messages                    |
| Coverage       | `cargo-llvm-cov` | No dead code silently escaping the above layers                                   |

Reports are published automatically to GitHub Pages on every push to `master`:

- [Coverage report](https://curiousmindflow.github.io/vigie/coverage/)
- [Mutation report](https://curiousmindflow.github.io/vigie/mutants/)

### Why mutation testing?

Coverage tells you which lines were executed. Mutation testing tells you whether your tests would _catch a bug_ in those lines. `cargo-mutants` systematically introduces changes (flipped conditions, swapped operators, removed branches) and verifies that at least one test fails. A high mutation score means the test suite is sensitive to logic errors — not just present for the metric.

---

## CI Pipeline

Every push and pull request runs the full pipeline:

```
fmt ──► lint ──► build ──► test ──► coverage ──┐
                                └──► mutants   ├──► deploy (GitHub Pages)
```

The pipeline is also scheduled twice a week to catch regressions from upstream dependency changes.

---

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full roadmap, including **VigieLab** — a planned web-based cluster simulation tool with scenario authoring in a Rhai DSL.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
