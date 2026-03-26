use vigie_core::Member;

#[derive(Debug)]
pub struct Configuration {
    pub id: Member,
    pub seeds: Vec<Member>,
}
