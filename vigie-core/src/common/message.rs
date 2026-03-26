use crate::{Member, common::MembershipEvent};

#[derive(Debug)]
pub enum Message {
    Ping {
        src: Member,
        relay: Option<Member>,
        dest: Member,
        events: Vec<MembershipEvent>,
    },
    PingRequest {
        src: Member,
        relay: Member,
        dest: Member,
    },
    Ack {
        src: Member,
        relay: Option<Member>,
        dest: Member,
        events: Vec<MembershipEvent>,
    },
}
