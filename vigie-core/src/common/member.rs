#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MembershipEntry {
    pub member: Member,
    pub incarnation_counter: u64,
    pub status: MemberStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemberStatus {
    Alive,
    Suspect,
    Confirm,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Member {
    ip: [u8; 12],
    port: u16,
}

impl Member {
    pub fn new_ipv4(ip: [u8; 12], port: u16) -> Self {
        Member { ip, port }
    }
}
