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
