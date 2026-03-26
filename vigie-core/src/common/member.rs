#[derive(Debug, Clone, Copy)]
pub struct MembershipEntry {
    pub member: Member,
    pub incarnation_counter: u64,
    pub kind: MembershipEventKind,
}

#[derive(Debug, Clone, Copy)]
pub struct MembershipEvent {
    pub entry: MembershipEntry,
    pub infection_number: u64,
}

impl MembershipEvent {
    pub fn new(entry: MembershipEntry, infection_number: u64) -> Self {
        Self {
            entry,
            infection_number,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MembershipEventKind {
    Alive,
    Suspect,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemberKind {
    Ipv4,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Member {
    kind: MemberKind,
    ip: [u8; 12],
    port: u16,
}

impl Member {
    pub fn new_ipv4(ip: [u8; 12], port: u16) -> Self {
        Member {
            kind: MemberKind::Ipv4,
            ip,
            port,
        }
    }
}
