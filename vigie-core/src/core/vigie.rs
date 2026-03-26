use std::collections::{HashMap, hash_map::Entry};

use crate::{
    common::{
        Effect, EffectStore, Error, Member, MembershipEntry, MembershipEvent, MembershipEventKind,
        Message,
    },
    core::member_store::MemberStore,
};

#[derive(Debug)]
pub struct Vigie<M, E>
where
    M: MemberStore,
    E: EffectStore,
{
    member_store: M,
    effect_store: E,
    dissemination_buffer: HashMap<Member, MembershipEvent>,
    seeds: Vec<Member>,
    id: Member,
    k: u16,
    period: i64,
    indirect_probe_timeout: i64,
    suspicion_timeout: i64,
    period_start_timestamp: i64,
    ack_member_await: Option<Member>,
    event_slots: u64,
}

impl<M, E> Vigie<M, E>
where
    M: MemberStore,
    E: EffectStore,
{
    // TODO: seeds must contains at least one Member
    pub fn new(
        member_store: impl MemberStore,
        effect_store: impl EffectStore,
        local: Member,
        seeds: Vec<Member>,
        k: u16,
        period: i64,
        timeout: i64,
    ) -> Result<Self, Error> {
        let event_slots = ((u16::MAX - 1024) / 32) as u64;
        unimplemented!()
    }

    // FIXME: should be in constructor method, the Member is automatically 'started'
    pub fn join(&mut self) -> Result<(), Error> {
        let entry = MembershipEntry {
            member: self.id,
            incarnation_counter: 0,
            kind: MembershipEventKind::Alive,
        };
        self.member_store.push(entry);
        self.dissemination_buffer.insert(
            entry.member,
            MembershipEvent::new(
                entry,
                Self::compute_infection_count(self.member_store.len()),
            ),
        );

        self.seeds();

        Ok(())
    }

    pub fn get_members(&self) -> &[MembershipEntry] {
        self.member_store.members()
    }

    pub fn ingest(&mut self, message: Message) -> Result<(), Error> {
        match message {
            Message::Ping {
                src,
                relay: None,
                dest,
                events,
            } if dest == self.id => {
                self.merge_received_events(events);
                let events = self.grab_events();

                self.effect_store.push(Effect::Ack {
                    src: dest,
                    relay: None,
                    dest: src,
                    events,
                });
            }
            Message::Ping {
                src,
                relay: Some(relay),
                dest,
                events,
            } if dest == self.id => {
                self.merge_received_events(events);
                let events = self.grab_events();

                self.effect_store.push(Effect::Ack {
                    src: dest,
                    relay: Some(relay),
                    dest: src,
                    events,
                });
            }
            Message::PingRequest { src, relay, dest }
                if relay == self.id && self.member_store.contains(dest) =>
            {
                let events = self.grab_events();

                self.effect_store.push(Effect::Ping {
                    src,
                    relay: Some(relay),
                    dest,
                    events,
                });
            }
            Message::Ack {
                src,
                relay: None,
                dest,
                events,
            } if dest == self.id => {
                if let Some(waiting) = self.ack_member_await
                    && waiting == src
                {
                    self.merge_received_events(events);
                    self.ack_member_await.take();
                }
            }
            Message::Ack {
                src,
                relay: Some(relay),
                dest,
                events,
            } if relay == self.id => {
                self.merge_received_events(events);
                let events = self.grab_events();

                self.effect_store.push(Effect::Ack {
                    src,
                    relay: None,
                    dest,
                    events,
                });
            }
            _ => (),
        }

        Ok(())
    }

    pub fn start_period(&mut self, now: i64) -> Result<(), Error> {
        if self.period_start_timestamp + self.period > now {
            return Err(Error::PeriodNotOver);
        }
        self.period_start_timestamp = now;

        let dest = self.select_next_member();
        let events = self.grab_events();

        self.effect_store.push(Effect::Ping {
            src: self.id,
            relay: None,
            dest: dest.member,
            events,
        });

        if let Some(waiting) = self.ack_member_await.replace(dest.member) {
            let waiting_member = self.member_store.get_mut(waiting).unwrap();
            waiting_member.kind = MembershipEventKind::Suspect;
            match self.dissemination_buffer.entry(waiting) {
                Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.get_mut().entry.kind = MembershipEventKind::Suspect;
                }
                Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(MembershipEvent {
                        entry: MembershipEntry {
                            member: waiting,
                            incarnation_counter: waiting_member.incarnation_counter,
                            kind: MembershipEventKind::Suspect,
                        },
                        infection_number: Self::compute_infection_count(self.member_store.len()),
                    });
                }
            }
            self.effect_store.push(Effect::ScheduleSuspicionTimeout {
                delay: self.suspicion_timeout,
                target: waiting,
            });
            // TODO: add an Effect saying a member is Suspected
        }

        self.effect_store.push(Effect::ScheduleIndirectProbe {
            delay: self.indirect_probe_timeout,
            target: dest.member,
        });

        self.effect_store
            .push(Effect::ScheduleNextPeriod { delay: self.period });

        Ok(())
    }

    pub fn indirect_probe(&mut self, now: i64) -> Result<(), Error> {
        if self.period_start_timestamp + self.indirect_probe_timeout > now {
            return Err(Error::TimeoutNotReached);
        }

        let Some(awaiting_member) = self.ack_member_await else {
            return Ok(());
        };

        for member in self.member_store.get_randomly(self.k, awaiting_member) {
            self.effect_store.push(Effect::PingRequest {
                src: self.id,
                relay: member.member,
                dest: awaiting_member,
            });
        }

        Ok(())
    }

    pub fn confirm_suspicion(&mut self, now: i64, suspect: Member) -> Result<(), Error> {
        let Some(
            entry @ MembershipEntry {
                kind: MembershipEventKind::Suspect,
                ..
            },
        ) = self.member_store.get(suspect)
        else {
            return Ok(());
        };

        self.member_store.remove(suspect);
        let event = MembershipEvent {
            entry: MembershipEntry {
                kind: MembershipEventKind::Confirm,
                ..entry
            },
            infection_number: Self::compute_infection_count(self.member_store.len()),
        };
        self.dissemination_buffer.insert(entry.member, event);
        Ok(())
    }

    pub fn pop_effect(&mut self) -> Option<Effect> {
        self.effect_store.pop()
    }

    fn select_next_member(&mut self) -> MembershipEntry {
        if self.member_store.len() <= 1 {
            self.seeds();
        }
        loop {
            if let Some(dest @ MembershipEntry { member, .. }) = self.member_store.next() {
                if member == self.id {
                    continue;
                } else {
                    break dest;
                }
            }
            self.member_store.shuffle();
        }
    }

    fn grab_events(&mut self) -> Vec<MembershipEvent> {
        let mut values = self
            .dissemination_buffer
            .values()
            .copied()
            .collect::<Vec<_>>();
        values.sort_by(|a, b| a.infection_number.cmp(&b.infection_number));
        let values = values
            .iter()
            .take(self.event_slots as usize)
            .copied()
            .collect::<Vec<_>>();

        for event in values.iter() {
            self.dissemination_buffer
                .get_mut(&event.entry.member)
                .unwrap()
                .infection_number -= 1;
        }

        values
    }

    fn merge_received_events(&mut self, events: Vec<MembershipEvent>) {
        for event in events {
            if let Some(member) = self.member_store.get(event.entry.member) {
                if matches!(event.entry.kind, MembershipEventKind::Confirm) {
                    self.member_store.remove(event.entry.member);
                    match self.dissemination_buffer.entry(event.entry.member) {
                        Entry::Occupied(mut occupied_entry) => {
                            occupied_entry.get_mut().entry.kind = MembershipEventKind::Confirm;
                        }
                        Entry::Vacant(vacant_entry) => {
                            vacant_entry.insert(event);
                        }
                    }
                }

                if matches!(event.entry.kind, MembershipEventKind::Suspect)
                    && ((matches!(member.kind, MembershipEventKind::Alive)
                        && event.entry.incarnation_counter >= member.incarnation_counter)
                        || (matches!(member.kind, MembershipEventKind::Suspect)
                            && event.entry.incarnation_counter > member.incarnation_counter))
                {
                    self.member_store.get_mut(member.member).unwrap().kind = event.entry.kind;
                    match self.dissemination_buffer.entry(event.entry.member) {
                        Entry::Occupied(mut occupied_entry) => {
                            occupied_entry.get_mut().entry.kind = event.entry.kind;
                        }
                        Entry::Vacant(vacant_entry) => {
                            vacant_entry.insert(event);
                        }
                    }
                }

                if matches!(event.entry.kind, MembershipEventKind::Alive)
                    && !matches!(member.kind, MembershipEventKind::Confirm)
                    && event.entry.incarnation_counter > member.incarnation_counter
                {
                    self.member_store.get_mut(member.member).unwrap().kind = event.entry.kind;
                    match self.dissemination_buffer.entry(event.entry.member) {
                        Entry::Occupied(mut occupied_entry) => {
                            occupied_entry.get_mut().entry.kind = event.entry.kind;
                        }
                        Entry::Vacant(vacant_entry) => {
                            vacant_entry.insert(event);
                        }
                    }
                }
            }
        }
    }

    fn seeds(&mut self) {
        for seed in &self.seeds {
            let entry = MembershipEntry {
                member: *seed,
                incarnation_counter: 0,
                kind: MembershipEventKind::Alive,
            };
            self.member_store.push(entry);
            self.dissemination_buffer.insert(
                entry.member,
                MembershipEvent::new(
                    entry,
                    Self::compute_infection_count(self.member_store.len()),
                ),
            );
        }
    }

    fn compute_infection_count(member_len: u64) -> u64 {
        if member_len == 0 {
            1
        } else {
            let n = member_len.ilog2() as u64;
            if n == 0 { 1 } else { n }
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use crate::{EffectStore, MemberStore, Vigie};

    // proptest! {
    //     #[test]
    //     fn test_compute_infection_count(l in 0u64..u64::MAX) {
    //         let result = Vigie::<TestMemberStore, TestEffectStore>::compute_infection_count(l);
    //         println!("{}", l);
    //         prop_assert!(result >= 1);
    //     }
    // }

    struct TestMemberStore;

    impl MemberStore for TestMemberStore {
        fn members(&self) -> &[crate::common::MembershipEntry] {
            todo!()
        }

        fn next(&mut self) -> Option<crate::common::MembershipEntry> {
            todo!()
        }

        fn contains(&self, member: crate::Member) -> bool {
            todo!()
        }

        fn get(&mut self, member: crate::Member) -> Option<crate::common::MembershipEntry> {
            todo!()
        }

        fn get_mut(
            &mut self,
            member: crate::Member,
        ) -> Option<&mut crate::common::MembershipEntry> {
            todo!()
        }

        fn push(&mut self, member: crate::common::MembershipEntry) {
            todo!()
        }

        fn remove(&mut self, member: crate::Member) {
            todo!()
        }

        fn clean(&mut self) {
            todo!()
        }

        fn shuffle(&mut self) {
            todo!()
        }

        fn get_randomly(
            &mut self,
            k: u16,
            except: crate::Member,
        ) -> Vec<crate::common::MembershipEntry> {
            todo!()
        }

        fn len(&self) -> u64 {
            todo!()
        }

        fn is_empty(&self) -> bool {
            todo!()
        }
    }

    struct TestEffectStore;

    impl EffectStore for TestEffectStore {
        fn pop(&mut self) -> Option<crate::Effect> {
            todo!()
        }

        fn push(&mut self, effect: crate::Effect) {
            todo!()
        }
    }
}
