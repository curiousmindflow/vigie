# TODO

## Refactoring

### Tombstone collection for Confirmed members

Currently `MemberStore` implementors must filter `Confirm` members in `next()` and `get_randomly()` — a business rule leaking into the storage layer.

**Proposal:** introduce an internal `tombstones: HashMap<Member, MembershipEntry>` field in `Vigie` (not a trait — purely internal). `MemberStore` only holds Alive/Suspect members.

**Impact:**
- `MemberStore` trait becomes purely structural, no status filtering required
- `next()` and `get_randomly()` simplify — no Confirm guards
- `confirm_suspicion`: `member_store.remove` + `tombstones.insert`
- `update_event`: must check tombstones when looking up a known member (Confirm-on-Confirm max-incarnation merge needs tombstone incarnation; unknown-member path must check tombstones before treating as truly new)
- Resurrection in `ingest`: `tombstones.remove` + `member_store.insert(Alive)` — cleaner than in-place mutation
- `len()` correctly reflects live cluster size, which matters for `compute_infection_count`
- `get_members()` can expose tombstones separately if needed

## Property Tests

Strategies `arb_membership_entry`, `arb_member_status`, `arb_member` already exist and are reusable for all tests below.

### Done
- `compute_infection_count` bounds — result always ≥ 1
- Convergence of `merge_received_events` — same events in any order → identical member_store state

### To do

- **`update_event` monotonicity** — generate `(new_status, new_inc, known_status, known_inc)`, call `update_event`, assert member_store incarnation never decreases. Catches `>` vs `>=` boundary bugs. Lives in-file.

- **Self-defense** — generate any `Suspect{self, inc}`, assert dissemination buffer contains `Alive{self, inc+1}`. Lives in-file.

- **`grab_events` infection_number never negative** — arbitrary dissemination buffer state, call `grab_events` N times, assert every remaining entry has `infection_number > 0` and return count ≤ `event_slots`. Lives in-file.
