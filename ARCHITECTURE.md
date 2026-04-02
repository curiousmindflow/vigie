# Architecture

## Design Principles

### Sans-IO

`vigie-core` never performs I/O. It does not open sockets, set timers, spawn threads, or call `sleep`. Instead, every action the protocol wants to take is encoded as an `Effect` value, pushed into an `EffectStore`, and consumed by the caller's realization layer.

This separation has several concrete benefits:

- **Deterministic testing.** Protocol logic can be driven entirely from a test with a fake clock and inspectable effect queues — no network required.
- **Runtime independence.** The core works with Tokio, smol, a bare-metal loop, or a simulation engine. The caller chooses.
- **Isolation of concerns.** Bugs in transport, serialization, or scheduling cannot corrupt protocol state, and vice versa.

### `no_std`-compatible core

`vigie-core` depends only on `rand` (for member selection randomization) and `thiserror` (for error types). It has no dependency on the Rust standard library beyond these, making it embeddable in environments without an OS or heap allocator.

The builtin store implementations use `std` collections for convenience, but these are optional. A `no_std` user can supply their own implementations of the storage traits backed by fixed-size arrays or a custom allocator.

### Trait-based storage

Two traits abstract over storage to avoid baking in any particular memory layout:

```rust
pub trait MemberStore { /* iteration, insertion, lookup */ }
pub trait EffectStore {
    fn push(&mut self, effect: Effect);
    fn pop(&mut self) -> Option<Effect>;
}
```

The protocol core is generic over both: `Vigie<M: MemberStore, E: EffectStore>`. The caller chooses the backing store. This allows embedding the protocol in an arena allocator, storing members in a fixed-size array, or using a custom effect queue with filtering or priority.

---

## Workspace Layout

The repository contains two crates:

**`vigie-core`** — the protocol core. Pure Rust, no runtime, `no_std`-compatible. It holds the SWIM state machine, membership table, gossip dissemination buffer, and the `Effect` model. It has no awareness of networks, threads, or time sources.

**`vigie`** — a Tokio-based realization. It wires `vigie-core` to real UDP sockets and OS timers, and exposes a high-level async API to application code. It is one possible realization — not the only one.

---

## Core Protocol State Machine

### Caller-driven timing

The core does not own a clock. All protocol events are triggered by the caller passing a `now: i64` timestamp (milliseconds). This design enables deterministic unit tests, time compression for simulation, and compatibility with any hardware timer source.

The three entry points that drive protocol progress:

| Method | When to call |
|---|---|
| `start_period(now)` | At the start of each probe period |
| `indirect_probe(now)` | When the direct probe timeout expires |
| `confirm_suspicion(now, member)` | When the suspicion timeout for a member expires |

### Message ingestion

```rust
vigie.ingest(message: Message);
```

Incoming messages are processed synchronously. The core updates internal state and may push new `Effect` values as a result. Message types are `Ping`, `Ack`, and `PingRequest`.

### Effect model

After any state-advancing call, the caller drains the effect queue:

```rust
while let Some(effect) = vigie.pop_effect() {
    match effect {
        Effect::Ping { src, dest, events } => { /* send packet */ }
        Effect::Ack { src, dest, events } => { /* send packet */ }
        Effect::PingRequest { src, dest, target } => { /* send packet */ }
        Effect::ScheduleNextPeriod { delay } => { /* set timer */ }
        Effect::ScheduleIndirectProbe { delay, target } => { /* set timer */ }
        Effect::ScheduleSuspicionTimeout { delay, target } => { /* set timer */ }
    }
}
```

The `Schedule*` effects carry a `delay` in milliseconds. The caller fires the corresponding protocol method when the timer expires.

### Dissemination buffer

Membership events (`MembershipEvent`) are piggybacked on outgoing `Ping` and `Ack` messages. The core maintains a gossip buffer tracking which events still need dissemination and how many times they have been sent (infection number). Events are dropped after `ceil(log2(n))` transmissions, matching the SWIM paper's dissemination bound.

### Member lifecycle

```
         join
          │
          ▼
        Alive ──── probe timeout ────► Suspect ──── suspicion timeout ────► Confirm (dead)
          ▲                                │
          └──────── refutation ────────────┘
```

A node can refute its own suspicion by incrementing its incarnation counter and broadcasting an `Alive` event.

---

## Implementing a Custom Realization

To use `vigie-core` without Tokio — or without any async runtime — implement the two storage traits and write an I/O loop:

```rust
struct MyMemberStore { /* ... */ }
impl MemberStore for MyMemberStore { /* ... */ }

struct MyEffectStore { /* ... */ }
impl EffectStore for MyEffectStore { /* ... */ }

let mut vigie = VigieBuilder::new(local, MyMemberStore::new(), MyEffectStore::new())
    .build()
    .unwrap();

loop {
    let now = my_clock_ms();

    if period_timer_fired() {
        vigie.start_period(now);
    }
    if let Some(msg) = recv_udp() {
        vigie.ingest(msg);
    }
    while let Some(effect) = vigie.pop_effect() {
        execute(effect);
    }
}
```

No async runtime, no allocator beyond what your stores require.
