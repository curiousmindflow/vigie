use std::marker::PhantomData;

use crate::{
    Error, Event, Member, MemberList, builtin_effect_store::BuiltinEffectStore,
    builtin_member_store::BuiltinMemberStore, config::Configuration,
};

#[derive(Debug, Default)]
pub struct VigieBuilder<'builder> {
    local: Option<&'builder str>,
    seeds: Vec<&'builder str>,
    k: Option<u64>,
    period: Option<i64>,
    timeout: Option<i64>,
}

impl<'builder> VigieBuilder<'builder> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, local: &'builder str) -> Self {
        unimplemented!()
    }

    pub fn seed(mut self, seed: &'builder str) -> Self {
        unimplemented!()
    }

    pub fn k(mut self, k: u64) -> Self {
        unimplemented!()
    }

    pub fn period(mut self, period: i64) -> Self {
        unimplemented!()
    }

    pub fn timeout(mut self, timeout: i64) -> Self {
        unimplemented!()
    }

    pub fn build<T>(self) -> Result<Vigie<T>, Error> {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct Vigie<T> {
    vigie: vigie_core::Vigie<BuiltinMemberStore, BuiltinEffectStore>,
    _phantom: PhantomData<T>,
}

impl<T> Vigie<T> {
    pub fn id(&self) -> Member {
        unimplemented!()
    }

    pub fn members(&self) -> MemberList {
        unimplemented!()
    }

    pub async fn wait(&mut self) -> Result<Event<T>, Error> {
        unimplemented!()
    }
}

impl<T> Drop for Vigie<T> {
    fn drop(&mut self) {
        // TODO: call vigie-core::Vigie::leave()
    }
}
