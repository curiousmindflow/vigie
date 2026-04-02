# Roadmap

## Current Status

`vigie-core` implements the SWIM protocol state machine including direct probing, indirect probing, the suspicion mechanism, and infection-style dissemination. The builtin store implementations cover standard use cases.

The `vigie` realization layer (Tokio-based) is in early development. The public API is defined; the implementation is in progress.

---

## Protocol Correctness

- [ ] Complete property test suite covering all protocol invariants
- [ ] Fuzz corpus for `ingest()` under malformed and adversarial messages
- [ ] 100% code coverage on `vigie-core`
- [ ] 100% mutation score on `vigie-core`
- [ ] Validate `no_std` compatibility with a bare-metal target

---

## Realization Layer (`vigie`)

- [ ] UDP transport: serialize/deserialize `Message` over the wire
- [ ] Timer integration: fire `start_period`, `indirect_probe`, `confirm_suspicion` from Tokio timers
- [ ] High-level async API: expose `Event` (member joined, member failed, member recovered) to application code
- [ ] Graceful leave: notify peers before shutdown

---

## VigieLab

VigieLab is a planned web-based simulation environment for the SWIM protocol. Its goal is to make the protocol's convergence dynamics observable and explorable without writing any code.

### What it does

- Simulate a cluster of N nodes running the Vigie protocol
- Visualize membership state, gossip propagation, and convergence over time
- Let users inject failures, partitions, and recoveries interactively or via a script

### Scenario DSL (Rhai)

Scenarios are authored in [Rhai](https://rhai.rs/) — an embeddable scripting language with a familiar syntax. Rhai was chosen because it is sandboxed, has no unsafe code, and integrates cleanly into Rust without a heavy runtime.

A scenario script controls:

- cluster topology (node count, initial members)
- protocol parameters (period, timeout, fanout k)
- events over time (crash a node at T=5s, partition nodes A–C at T=10s, heal at T=20s)

Example:

```rhai
let cluster = Cluster::new(20);
cluster.set_k(3);
cluster.set_period(1000);

at(5000, || cluster.crash("node-7"));
at(10000, || cluster.partition(["node-1", "node-2", "node-3"]));
at(20000, || cluster.heal());
```

### Planned features

- [ ] Core simulation engine backed by `vigie-core` (no real networking — effects are simulated)
- [ ] Rhai DSL interpreter for scenario authoring
- [ ] Web UI: cluster graph, membership state timeline, convergence metrics
- [ ] Preset scenarios: cascading failures, split-brain, flapping nodes
- [ ] Export: scenario replay as a reproducible seed + script pair

---

## Not Planned

- Built-in service discovery or key-value storage — Vigie is a membership and fault detection layer; higher-level concerns belong in the application
- Compatibility shims with other membership protocols (Serf, Consul, etc.)

## Open Questions

- **User-defined dissemination payloads.** The SWIM paper allows arbitrary application data to be piggybacked alongside membership events. Vigie may expose this as a typed extension point, letting callers attach metadata to the gossip stream without modifying the protocol core.
