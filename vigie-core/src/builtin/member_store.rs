use rand::{
    rngs::ThreadRng,
    seq::{IteratorRandom, SliceRandom},
};

use crate::{MemberStore, common::MembershipEntry};

#[derive(Debug, Default)]
pub struct BuiltinMemberStore {
    store: Vec<MembershipEntry>,
    pos: usize,
    rng: ThreadRng,
}

impl BuiltinMemberStore {
    pub fn new() -> Self {
        BuiltinMemberStore {
            store: Vec::default(),
            pos: 0,
            rng: rand::rng(),
        }
    }
}

impl MemberStore for BuiltinMemberStore {
    fn members(&self) -> &[crate::common::MembershipEntry] {
        &self.store
    }

    fn next(&mut self) -> Option<crate::common::MembershipEntry> {
        let element = self.store.get(self.pos).cloned();
        self.pos += 1;
        element
    }

    fn contains(&self, member: crate::Member) -> bool {
        self.store.iter().any(|e| e.member == member)
    }

    fn get(&mut self, member: crate::Member) -> Option<crate::common::MembershipEntry> {
        self.store.iter().find(|e| e.member == member).cloned()
    }

    fn get_mut(&mut self, member: crate::Member) -> Option<&mut crate::common::MembershipEntry> {
        self.store.iter_mut().find(|e| e.member == member)
    }

    fn push(&mut self, member: crate::common::MembershipEntry) {
        self.store.push(member);
    }

    fn remove(&mut self, member: crate::Member) {
        self.store.retain(|e| e.member != member);
    }

    fn clean(&mut self) {
        self.store.clear();
    }

    fn shuffle(&mut self) {
        self.store.shuffle(&mut self.rng);
    }

    fn get_randomly(
        &mut self,
        k: u16,
        except: crate::Member,
    ) -> Vec<crate::common::MembershipEntry> {
        self.store
            .iter()
            .filter(|e| e.member != except)
            .sample(&mut self.rng, k as usize)
            .into_iter()
            .cloned()
            .collect::<Vec<crate::common::MembershipEntry>>()
    }

    fn len(&self) -> u64 {
        self.store.len() as u64
    }

    fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}
