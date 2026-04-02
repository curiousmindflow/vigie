use crate::{Member, common::MembershipEvent};

pub trait EffectStore {
    fn pop(&mut self) -> Option<Effect>;
    fn push(&mut self, effect: Effect);
}

#[derive(Debug)]
pub enum Effect {
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
    ScheduleNextPeriod {
        delay: i64,
    },
    ScheduleIndirectProbe {
        delay: i64,
        target: Member,
    },
    ScheduleSuspicionTimeout {
        delay: i64,
        target: Member,
    },
}
