use std::fmt::Display;

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

impl Display for MembershipEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} - {}", self.entry, self.infection_number))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MembershipEntry {
    pub member: Member,
    pub incarnation_counter: u64,
    pub status: MemberStatus,
}

impl Display for MembershipEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{} - {} - {}",
            self.member, self.status, self.incarnation_counter
        ))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemberStatus {
    Alive = 0,
    Suspect = 1,
    Confirm = 2,
}

impl Display for MemberStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemberStatus::Alive => f.write_str("Alive")?,
            MemberStatus::Suspect => f.write_str("Suspect")?,
            MemberStatus::Confirm => f.write_str("Confirm")?,
        };
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Member {
    ip: [u8; 4],
    port: u16,
}

impl Member {
    pub fn new_ipv4(ip: [u8; 4], port: u16) -> Self {
        Member { ip, port }
    }
}

impl Display for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}.{}.{}.{}:{}",
            self.ip[3], self.ip[2], self.ip[1], self.ip[0], self.port
        ))?;
        Ok(())
    }
}
