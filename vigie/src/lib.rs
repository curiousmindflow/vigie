mod builtin_effect_store;
mod builtin_member_store;
mod config;
mod error;
mod event;
mod vigie;

use std::str::FromStr;

pub use config::Configuration;
pub use error::Error;
pub use event::Event;
pub use vigie::{Vigie, VigieBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Member(vigie_core::Member);

impl FromStr for Member {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemberList {}
