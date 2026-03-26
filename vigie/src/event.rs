use crate::Member;

#[derive(Debug)]
pub enum Event<T> {
    MemberAlive { id: Member, data: Option<T> },
    MemberSuspected { id: Member },
    MemberDead { id: Member },
}
