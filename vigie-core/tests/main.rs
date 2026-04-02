use rstest::rstest;
use vigie_core::{BuiltinEffectStore, BuiltinMemberStore, Member, Vigie, VigieBuilder};

#[rstest]
fn main() {
    let member_store = BuiltinMemberStore::new();
    let effect_store = BuiltinEffectStore::new();
    let local = Member::new_ipv4([0, 0, 0, 0, 0, 0, 0, 0, 127, 0, 0, 1], 9000);
    let seed = Member::new_ipv4([0, 0, 0, 0, 0, 0, 0, 0, 127, 0, 0, 1], 9001);

    let mut vigie = VigieBuilder::new(local, member_store, effect_store)
        .seed(seed)
        .k(3)
        .period(1000)
        .timeout(500)
        .suspicion_timeout(1500)
        .build()
        .unwrap();

    //
}
