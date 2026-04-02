use crate::{
    EffectStore, Member, MemberStore, Vigie, VigieError,
    common::{MemberStatus, MembershipEntry, MembershipEvent},
};

#[derive(Debug)]
pub struct VigieBuilder<M, E>
where
    M: MemberStore,
    E: EffectStore,
{
    id: Member,
    member_store: M,
    effect_store: E,
    seeds: Vec<Member>,
    k: u16,
    period: i64,
    timeout: i64,
    suspicion_timeout: i64,
}

impl<M, E> VigieBuilder<M, E>
where
    M: MemberStore,
    E: EffectStore,
{
    pub fn new(id: Member, member_store: M, effect_store: E) -> Self {
        Self {
            id,
            member_store,
            effect_store,
            seeds: Vec::default(),
            k: 3,
            period: 1000,
            timeout: 500,
            suspicion_timeout: 1500,
        }
    }

    pub fn seed(mut self, seed: Member) -> Self {
        self.seeds.push(seed);
        self
    }

    pub fn k(mut self, k: u16) -> Self {
        self.k = k;
        self
    }

    pub fn period(mut self, period: i64) -> Self {
        self.period = period;
        self
    }

    pub fn timeout(mut self, timeout: i64) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn suspicion_timeout(mut self, suspicion_timeout: i64) -> Self {
        self.suspicion_timeout = suspicion_timeout;
        self
    }

    pub fn build(self) -> Result<Vigie<M, E>, VigieError> {
        let VigieBuilder {
            id,
            member_store,
            effect_store,
            seeds,
            k,
            period,
            timeout,
            suspicion_timeout,
        } = self;
        let mut vigie = Vigie {
            member_store,
            effect_store,
            dissemination_buffer: Default::default(),
            pending_ping_req: Default::default(),
            seeds,
            id,
            k,
            period,
            timeout,
            suspicion_timeout,
            period_start_timestamp: Default::default(),
            ack_member_await: None,
            event_slots: ((u16::MAX - 1024) / 32) as u64,
        };
        Self::join(&mut vigie)?;
        Ok(vigie)
    }

    pub(super) fn join(vigie: &mut Vigie<M, E>) -> Result<(), VigieError> {
        let entry = MembershipEntry {
            member: vigie.id,
            incarnation_counter: 0,
            status: MemberStatus::Alive,
        };
        vigie.member_store.push(entry);
        vigie.dissemination_buffer.insert(
            entry.member,
            MembershipEvent::new(
                entry,
                Vigie::<M, E>::compute_infection_count(vigie.member_store.len()),
            ),
        );

        vigie.seeds();

        Ok(())
    }
}
