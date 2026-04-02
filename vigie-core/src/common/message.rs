use crate::{Member, common::MembershipEvent};

#[derive(Debug)]
pub enum Message {
    Ping {
        src: Member,
        dest: Member,
        events: Vec<MembershipEvent>,
    },
    PingRequest {
        src: Member,
        dest: Member,
        target: Member,
    },
    Ack {
        src: Member,
        dest: Member,
        events: Vec<MembershipEvent>,
    },
}
