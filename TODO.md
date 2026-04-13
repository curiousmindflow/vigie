# TODO

## Refactoring

### ~~Tombstone collection for Confirmed members~~ — DONE

### ~~Remove self-filtering from MemberStore~~ — DONE

### Traits for dissemination buffer and tombstones — no_std compatibility

Currently both are concrete `HashMap` fields. Extract traits consistent with `MemberStore`/`EffectStore`.

**Motivation:** no_std support — users targeting embedded or kernel environments need allocator-free backing stores. Trait definitions with no `std` dependencies make the architecture genuinely portable. A spy implementation in tests can also assert insertions/removals without inspecting internal state directly.

## Property Tests

Strategies `arb_membership_entry`, `arb_member_status`, `arb_member` already exist and are reusable.

### Done
- `compute_infection_count` bounds — result always ≥ 1
- Convergence of `merge_received_events` — same events in any order → identical member_store state
- `grab_events` — return count ≤ `event_slots`, remaining buffer entries have `infection_number > 0`
- `update_event` idempotence — applying same event twice leaves member_store unchanged

### To do
- **`update_event` monotonicity** — for a given member, lattice position `(status_rank, incarnation)` never decreases after applying any event. Requires `lattice_rank` helper. Lives in-file.
