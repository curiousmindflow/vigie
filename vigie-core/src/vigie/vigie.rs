use std::collections::{HashMap, hash_map::Entry};

use crate::{
    common::{
        Effect, EffectStore, Member, MemberStatus, MembershipEntry, MembershipEvent, Message,
        VigieError,
    },
    vigie::member_store::MemberStore,
};

#[derive(Debug)]
pub struct Vigie<M, E>
where
    M: MemberStore,
    E: EffectStore,
{
    pub(super) member_store: M,
    pub(super) effect_store: E,
    pub(super) dissemination_buffer: HashMap<Member, MembershipEvent>,
    pub(super) pending_ping_req: HashMap<Member, Member>,
    pub(super) seeds: Vec<Member>,
    pub(super) id: Member,
    pub(super) k: u16,
    pub(super) period: i64,
    pub(super) timeout: i64,
    pub(super) suspicion_timeout: i64,
    pub(super) period_start_timestamp: i64,
    pub(super) ack_member_await: Option<Member>,
    pub(super) event_slots: u64,
}

impl<M, E> Vigie<M, E>
where
    M: MemberStore,
    E: EffectStore,
{
    pub fn get_members(&self) -> &[MembershipEntry] {
        self.member_store.members()
    }

    pub fn ingest(&mut self, message: Message) -> Result<(), VigieError> {
        match message {
            Message::Ping { src, dest, events } if dest == self.id => {
                self.merge_received_events(events);
                let events = self.grab_events();

                self.effect_store.push(Effect::Ack {
                    src: dest,
                    dest: src,
                    events,
                });
            }
            Message::Ack { src, dest, events } if dest == self.id => {
                self.merge_received_events(events);

                if let Some(pending) = self.pending_ping_req.remove(&src) {
                    let events = self.grab_events();
                    self.effect_store.push(Effect::Ack {
                        src: self.id,
                        dest: pending,
                        events,
                    });
                }

                if let Some(waiting) = self.ack_member_await
                    && waiting == src
                {
                    self.ack_member_await.take();
                }
            }
            Message::PingRequest { src, target, dest }
                if dest == self.id && self.member_store.contains(target) =>
            {
                self.pending_ping_req.insert(target, src);
                let events = self.grab_events();
                self.effect_store.push(Effect::Ping {
                    src: self.id,
                    dest: target,
                    events,
                });
            }
            _ => (),
        }

        Ok(())
    }

    pub fn start_period(&mut self, now: i64) -> Result<(), VigieError> {
        if self.period_start_timestamp + self.period > now {
            return Err(VigieError::PeriodNotOver);
        }
        self.period_start_timestamp = now;

        let dest = self.select_next_member();
        let events = self.grab_events();

        self.effect_store.push(Effect::Ping {
            src: self.id,
            dest: dest.member,
            events,
        });

        if let Some(waiting) = self.ack_member_await.replace(dest.member) {
            let waiting_member = self.member_store.get_mut(waiting).unwrap();
            waiting_member.status = MemberStatus::Suspect;
            match self.dissemination_buffer.entry(waiting) {
                Entry::Occupied(mut occupied_entry) => {
                    occupied_entry.get_mut().entry.status = MemberStatus::Suspect;
                }
                Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(MembershipEvent {
                        entry: MembershipEntry {
                            member: waiting,
                            incarnation_counter: waiting_member.incarnation_counter,
                            status: MemberStatus::Suspect,
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
            delay: self.timeout,
            target: dest.member,
        });

        self.effect_store
            .push(Effect::ScheduleNextPeriod { delay: self.period });

        Ok(())
    }

    pub fn indirect_probe(&mut self, now: i64) -> Result<(), VigieError> {
        if self.period_start_timestamp + self.timeout > now {
            return Err(VigieError::TimeoutNotReached);
        }

        let Some(awaiting_member) = self.ack_member_await else {
            return Ok(());
        };

        for member in self.member_store.get_randomly(self.k, awaiting_member) {
            self.effect_store.push(Effect::PingRequest {
                src: self.id,
                target: member.member,
                dest: awaiting_member,
            });
        }

        Ok(())
    }

    pub fn confirm_suspicion(&mut self, now: i64, suspect: Member) -> Result<(), VigieError> {
        let Some(
            entry @ MembershipEntry {
                status: MemberStatus::Suspect,
                ..
            },
        ) = self.member_store.get(suspect)
        else {
            return Ok(());
        };

        self.member_store.remove(suspect);
        let event = MembershipEvent {
            entry: MembershipEntry {
                status: MemberStatus::Confirm,
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
            self.update_event(event);
        }
    }

    fn update_event(&mut self, mut new: MembershipEvent) {
        if matches!(new.entry.status, MemberStatus::Suspect) && new.entry.member == self.id {
            new.entry.incarnation_counter += 1;
            new.entry.status = MemberStatus::Alive;
            new.infection_number = Self::compute_infection_count(self.member_store.len());
            self.member_store
                .get_mut(self.id)
                .unwrap()
                .incarnation_counter = new.entry.incarnation_counter;
            self.dissemination_buffer.insert(self.id, new);
            return;
        }

        let Some(known) = self.member_store.get_mut(new.entry.member) else {
            if !matches!(new.entry.status, MemberStatus::Confirm) {
                self.member_store.push(new.entry);
            }
            self.dissemination_buffer.insert(
                new.entry.member,
                MembershipEvent {
                    entry: new.entry,
                    infection_number: Self::compute_infection_count(self.member_store.len()),
                },
            );
            return;
        };

        match (
            (new.entry.status, new.entry.incarnation_counter),
            (known.status, known.incarnation_counter),
        ) {
            ((MemberStatus::Confirm, _), _) => {
                *known = new.entry;
                new.infection_number = Self::compute_infection_count(self.member_store.len());
                self.member_store.remove(new.entry.member);
                self.dissemination_buffer.insert(new.entry.member, new);
                return;
            }
            ((MemberStatus::Suspect, i), (MemberStatus::Suspect, j)) if i > j => (),
            ((MemberStatus::Suspect, i), (MemberStatus::Alive, j)) if i >= j => (),
            ((MemberStatus::Alive, i), (MemberStatus::Suspect, j)) if i > j => (),
            ((MemberStatus::Alive, i), (MemberStatus::Alive, j)) if i > j => (),
            _ => return,
        };

        *known = new.entry;
        new.infection_number = Self::compute_infection_count(self.member_store.len());
        self.dissemination_buffer.insert(new.entry.member, new);
    }

    pub(super) fn seeds(&mut self) {
        for seed in &self.seeds {
            let entry = MembershipEntry {
                member: *seed,
                incarnation_counter: 0,
                status: MemberStatus::Alive,
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

    pub(super) fn compute_infection_count(member_len: u64) -> u64 {
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

    use crate::{
        BuiltinEffectStore, BuiltinMemberStore, Member, Vigie, VigieBuilder,
        common::{MemberStatus, MembershipEntry, MembershipEvent},
    };

    proptest! {
        #[test]
        fn test_compute_infection_count(l in 0u64..u64::MAX) {
            let result = Vigie::<BuiltinMemberStore, BuiltinEffectStore>::compute_infection_count(l);
            println!("{}", l);
            prop_assert!(result >= 1);
        }
    }

    proptest! {
        #[test]
        fn test_merge_received_events((events, shuffled) in arb_merge_received_events(3)) {
            let mut vigie_a = VigieBuilder::new(Member::default(), BuiltinMemberStore::new(), BuiltinEffectStore::new()).build().unwrap();
            let mut vigie_b = VigieBuilder::new(Member::default(), BuiltinMemberStore::new(), BuiltinEffectStore::new()).build().unwrap();

            vigie_a.merge_received_events(events);
            vigie_b.merge_received_events(shuffled);

            let mut vigie_a_members = vigie_a.get_members().to_vec();
            vigie_a_members.sort();
            let mut vigie_b_members = vigie_b.get_members().to_vec();
            vigie_b_members.sort();

            prop_assert_eq!(vigie_a_members, vigie_b_members)
        }
    }

    prop_compose! {
        fn arb_merge_received_events(nb_elements: usize)((events, shuffled) in arb_member()
                                                .prop_flat_map(move |m| {
                                                    prop::collection::vec(arb_membership_event(m), 1..=nb_elements)
                                                    .prop_flat_map(|events| {
                                                        let len = events.len();
                                                        let perm = prop::collection::vec(any::<proptest::sample::Index>(), len..=len);
                                                        (Just(events), perm)
                                                    })
                                                    .prop_map(|(events, indices)| {
                                                        let mut shuffled = events.clone();
                                                        for i in 0..shuffled.len() {
                                                            let j = indices[i].index(shuffled.len());
                                                            shuffled.swap(i, j);
                                                        }
                                                        (events, shuffled)
                                                    })
                                                })) -> (Vec<MembershipEvent>, Vec<MembershipEvent>) { (events, shuffled) }
    }

    prop_compose! {
        fn arb_member()(ip in prop::array::uniform12(1u8..u8::MAX), port in any::<u16>()) -> Member {
                Member::new_ipv4(ip, port)
            }
    }

    prop_compose! {
        fn arb_membership_entry(member: Member)(incarnation_counter in any::<u64>(), status in arb_member_status()) -> MembershipEntry {
            MembershipEntry { member, incarnation_counter, status }
        }
    }

    prop_compose! {
        fn arb_membership_event(member: Member)(entry in arb_membership_entry(member), infection_number in any::<u64>()) -> MembershipEvent {
            MembershipEvent { entry, infection_number }
        }
    }

    fn arb_member_status() -> impl Strategy<Value = MemberStatus> {
        prop_oneof![
            Just(MemberStatus::Alive),
            Just(MemberStatus::Suspect),
            Just(MemberStatus::Confirm),
        ]
    }
}
