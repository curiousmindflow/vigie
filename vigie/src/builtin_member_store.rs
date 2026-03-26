use vigie_core::{Member, MemberStore};

#[derive(Debug)]
pub struct BuiltinMemberStore {}

impl MemberStore for BuiltinMemberStore {
    fn members(&self) -> &[Member] {
        todo!()
    }

    fn next(&mut self) -> Option<Member> {
        unimplemented!()
    }

    fn contains(&self, member: Member) -> bool {
        unimplemented!()
    }

    fn remove(&mut self, member: Member) {
        unimplemented!()
    }

    fn shuffle(&mut self) {
        todo!()
    }

    fn get_randomly(&mut self, k: u16) -> Vec<Member> {
        unimplemented!()
    }
}
