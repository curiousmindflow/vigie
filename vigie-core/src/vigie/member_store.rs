use crate::common::{Member, MembershipEntry};

pub trait MemberStore {
    fn members(&self) -> &[MembershipEntry];
    fn next(&mut self) -> Option<MembershipEntry>;
    fn contains(&self, member: Member) -> bool;
    fn get(&mut self, member: Member) -> Option<MembershipEntry>;
    fn get_mut(&mut self, member: Member) -> Option<&mut MembershipEntry>;
    fn insert(&mut self, member: MembershipEntry);
    fn remove(&mut self, member: Member);
    fn clean(&mut self);
    fn shuffle(&mut self);
    fn get_randomly(&mut self, k: u16, except: Member) -> Vec<MembershipEntry>;
    fn len(&self) -> u64;
    fn is_empty(&self) -> bool;
}
