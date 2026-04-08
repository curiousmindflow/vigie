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
    // FIXME: why not a HashSet instead ?
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
                let src_event = events.iter().find(|e| e.entry.member == src).cloned();
                self.merge_received_events(events);

                self.resurrection(src, src_event);

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

    fn resurrection(&mut self, src: Member, src_event: Option<MembershipEvent>) {
        if let Some(src_member) = self.member_store.get_mut(src)
            && matches!(src_member.status, MemberStatus::Confirm)
        {
            src_member.status = MemberStatus::Alive;
            if let Some(received_event) = src_event {
                src_member.incarnation_counter = received_event.entry.incarnation_counter;
                self.dissemination_buffer.insert(
                    src,
                    MembershipEvent {
                        entry: MembershipEntry {
                            member: src,
                            incarnation_counter: received_event.entry.incarnation_counter,
                            status: MemberStatus::Alive,
                        },
                        infection_number: Self::compute_infection_count(self.member_store.len()),
                    },
                );
            }
        }
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

        self.member_store.get_mut(suspect).unwrap().status = MemberStatus::Confirm;

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

        self.dissemination_buffer
            .retain(|_, v| v.infection_number > 0);

        values
    }

    fn merge_received_events(&mut self, events: Vec<MembershipEvent>) {
        for event in events {
            self.update_event(event);
        }
    }

    fn update_event(&mut self, mut new: MembershipEvent) {
        if self.defend_against_suspicion(&mut new) {
            return;
        }

        let Some(known) = self.member_store.get_mut(new.entry.member) else {
            self.member_store.insert(new.entry);
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
            ((MemberStatus::Confirm, inc_new), (MemberStatus::Confirm, inc_known)) => {
                known.incarnation_counter = inc_new.max(inc_known);
                return;
            }
            ((MemberStatus::Confirm, _), _) => {
                *known = new.entry;
                new.infection_number = Self::compute_infection_count(self.member_store.len());
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

    fn defend_against_suspicion(&mut self, new: &mut MembershipEvent) -> bool {
        if matches!(new.entry.status, MemberStatus::Suspect) && new.entry.member == self.id {
            new.entry.incarnation_counter += 1;
            new.entry.status = MemberStatus::Alive;
            new.infection_number = Self::compute_infection_count(self.member_store.len());
            self.member_store
                .get_mut(self.id)
                .unwrap()
                .incarnation_counter = new.entry.incarnation_counter;
            self.dissemination_buffer.insert(self.id, *new);
            true
        } else {
            false
        }
    }

    pub(super) fn seeds(&mut self) {
        for seed in &self.seeds {
            let entry = MembershipEntry {
                member: *seed,
                incarnation_counter: 0,
                status: MemberStatus::Alive,
            };
            self.member_store.insert(entry);
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
    use rstest::{fixture, rstest};

    use crate::{
        BuiltinEffectStore, BuiltinMemberStore, Effect, Member, MemberStore, Message, Vigie,
        VigieBuilder,
        common::{MemberStatus, MembershipEntry, MembershipEvent},
    };

    #[rstest]
    fn test_ingest_ping(
        #[from(setup_member)] member: Member,
        #[from(setup_vigie)]
        #[with(member)]
        mut vigie: Vigie<BuiltinMemberStore, BuiltinEffectStore>,
    ) {
        let msg = Message::Ping {
            src: Member::default(),
            dest: member,
            events: vec![],
        };

        vigie.ingest(msg).unwrap();

        assert!(vigie.pop_effect().is_some())
    }

    #[rstest]
    fn test_ingest_ping_not_the_target(
        #[from(setup_member)]
        #[with(1)]
        _member: Member,
        #[from(setup_member)] dest: Member,
        #[from(setup_vigie)]
        #[with(_member)]
        mut vigie: Vigie<BuiltinMemberStore, BuiltinEffectStore>,
    ) {
        let msg = Message::Ping {
            src: Member::default(),
            dest,
            events: vec![],
        };

        vigie.ingest(msg).unwrap();

        assert!(vigie.pop_effect().is_none())
    }

    #[rstest]
    fn test_ingest_ack(
        #[from(setup_member)] pending_ping_req_member: Member,
        #[from(setup_member)] ack_member_await: Member,
        #[from(setup_member)] member: Member,
        #[from(setup_vigie)]
        #[with(member)]
        mut vigie: Vigie<BuiltinMemberStore, BuiltinEffectStore>,
    ) {
        vigie
            .pending_ping_req
            .insert(pending_ping_req_member, pending_ping_req_member);
        vigie.ack_member_await.replace(ack_member_await);

        let msg = Message::Ack {
            src: pending_ping_req_member,
            dest: member,
            events: vec![],
        };

        vigie.ingest(msg).unwrap();

        assert!(
            !vigie
                .pending_ping_req
                .contains_key(&pending_ping_req_member)
        );
        assert!(vigie.ack_member_await.is_none());
    }

    #[rstest]
    fn test_ingest_ack_not_the_target(
        #[from(setup_member)]
        #[with(1)]
        _member: Member,
        #[from(setup_member)] dest: Member,
        #[from(setup_vigie)]
        #[with(_member)]
        mut vigie: Vigie<BuiltinMemberStore, BuiltinEffectStore>,
    ) {
        let msg = Message::Ack {
            src: Member::default(),
            dest,
            events: vec![],
        };

        vigie.ingest(msg).unwrap();

        assert!(vigie.pop_effect().is_none())
    }

    #[rstest]
    fn test_ingest_pingreq(
        #[from(setup_member)] member: Member,
        #[from(setup_member)] target: Member,
        #[from(setup_vigie)]
        #[with(member)]
        mut vigie: Vigie<BuiltinMemberStore, BuiltinEffectStore>,
    ) {
        let msg = Message::PingRequest {
            src: Member::default(),
            target,
            dest: member,
        };

        vigie.member_store.insert(MembershipEntry {
            member: target,
            incarnation_counter: 0,
            status: MemberStatus::Alive,
        });

        vigie.ingest(msg).unwrap();

        let Some(Effect::Ping {
            src,
            dest,
            events: _,
        }) = vigie.pop_effect()
        else {
            panic!("Should have a Effect::Ping effect");
        };

        assert_eq!(src, member);
        assert_eq!(dest, target);
    }

    #[rstest]
    fn test_ingest_pingreq_not_the_target(
        #[from(setup_member)]
        #[with(1)]
        dest: Member,
        #[from(setup_member)] _member: Member,
        #[from(setup_vigie)]
        #[with(_member)]
        mut vigie: Vigie<BuiltinMemberStore, BuiltinEffectStore>,
    ) {
        let msg = Message::PingRequest {
            src: Member::default(),
            target: Member::default(),
            dest,
        };

        vigie.ingest(msg).unwrap();

        assert!(vigie.pop_effect().is_none())
    }

    #[rstest]
    fn test_ingest_pingreq_not_have_target(
        #[from(setup_member)] member: Member,
        #[from(setup_vigie)]
        #[with(member)]
        mut vigie: Vigie<BuiltinMemberStore, BuiltinEffectStore>,
    ) {
        let msg = Message::PingRequest {
            src: Member::default(),
            target: Member::default(),
            dest: member,
        };

        vigie.ingest(msg).unwrap();

        assert!(vigie.pop_effect().is_none())
    }

    #[rstest]
    fn test_resurrection(
        #[from(setup_member)]
        #[with(1)]
        src: Member,
        #[from(setup_member)] _member: Member,
        #[from(setup_vigie)]
        #[with(_member)]
        mut vigie: Vigie<BuiltinMemberStore, BuiltinEffectStore>,
    ) {
        vigie.member_store.insert(MembershipEntry {
            member: src,
            incarnation_counter: 0,
            status: MemberStatus::Confirm,
        });
        let src_event = MembershipEvent {
            entry: MembershipEntry {
                member: src,
                incarnation_counter: 0,
                status: MemberStatus::Alive,
            },
            infection_number: 0,
        };

        vigie.resurrection(src, Some(src_event));

        let Some(MembershipEvent {
            entry:
                MembershipEntry {
                    member,
                    incarnation_counter: _,
                    status: MemberStatus::Alive,
                },
            infection_number: _,
        }) = vigie.dissemination_buffer.get(&src)
        else {
            panic!()
        };

        assert_eq!(*member, src);
    }

    // TODO: start_period_correct_timestamp_no_ack_member_await
    // TODO: start_period_correct_timestamp_ack_member_await
    // TODO: start_period_inccorrect_timestamp
    // TODO: indirect_probe_timestamp_ok
    // TODO: indirect_probe_timestamp_nok
    // TODO: indirect_probe_timestamp_ok_ack_member_await
    // TODO: indirect_probe_timestamp_ok_no_ack_member_await
    // TODO: confirm_suspicion_suspect_known
    // TODO: confirm_suspicion_suspect_unknown
    // TODO: select_next_member
    // TODO: select_next_member_edge
    // TODO: grab_events
    // TODO: grab_events_edge
    // TODO: defend_against_suspicion

    #[rstest]
    fn test_defend_against_suspicion(
        #[from(setup_member)] member: Member,
        #[from(setup_vigie)]
        #[with(member)]
        mut vigie: Vigie<BuiltinMemberStore, BuiltinEffectStore>,
    ) {
        let mut event = MembershipEvent {
            entry: MembershipEntry {
                member,
                incarnation_counter: 0,
                status: MemberStatus::Suspect,
            },
            infection_number: 0,
        };

        let current_member_entry = vigie.member_store.get(member).unwrap();

        vigie.defend_against_suspicion(&mut event);

        let Some(MembershipEvent {
            entry:
                MembershipEntry {
                    member: actual_member,
                    incarnation_counter,
                    status,
                },
            infection_number: _,
        }) = vigie.dissemination_buffer.get(&member)
        else {
            panic!()
        };

        assert_eq!(*actual_member, member);
        assert_eq!(*status, MemberStatus::Alive);
        assert_eq!(
            *incarnation_counter,
            current_member_entry.incarnation_counter + 1
        );
    }

    #[rstest]
    fn test_merge_received_events_confirm_confirm_suspect_confirm_suspect_confirm() {
        let events = vec![
            MembershipEvent {
                entry: MembershipEntry {
                    member: Member::new_ipv4([0, 0, 0, 1], 0),
                    incarnation_counter: 695773933846812385,
                    status: MemberStatus::Confirm,
                },
                infection_number: 1724690186970220747,
            },
            MembershipEvent {
                entry: MembershipEntry {
                    member: Member::new_ipv4([0, 0, 0, 1], 0),
                    incarnation_counter: 7581484833041126904,
                    status: MemberStatus::Confirm,
                },
                infection_number: 6132923370804885193,
            },
            MembershipEvent {
                entry: MembershipEntry {
                    member: Member::new_ipv4([0, 0, 0, 1], 0),
                    incarnation_counter: 12862825752618528021,
                    status: MemberStatus::Suspect,
                },
                infection_number: 9902733496451980313,
            },
        ];

        let shuffled = vec![
            MembershipEvent {
                entry: MembershipEntry {
                    member: Member::new_ipv4([0, 0, 0, 1], 0),
                    incarnation_counter: 7581484833041126904,
                    status: MemberStatus::Confirm,
                },
                infection_number: 6132923370804885193,
            },
            MembershipEvent {
                entry: MembershipEntry {
                    member: Member::new_ipv4([0, 0, 0, 1], 0),
                    incarnation_counter: 12862825752618528021,
                    status: MemberStatus::Suspect,
                },
                infection_number: 9902733496451980313,
            },
            MembershipEvent {
                entry: MembershipEntry {
                    member: Member::new_ipv4([0, 0, 0, 1], 0),
                    incarnation_counter: 695773933846812385,
                    status: MemberStatus::Confirm,
                },
                infection_number: 1724690186970220747,
            },
        ];

        let mut vigie_a = VigieBuilder::new(
            Member::default(),
            BuiltinMemberStore::new(),
            BuiltinEffectStore::new(),
        )
        .build()
        .unwrap();
        let mut vigie_b = VigieBuilder::new(
            Member::default(),
            BuiltinMemberStore::new(),
            BuiltinEffectStore::new(),
        )
        .build()
        .unwrap();

        vigie_a.merge_received_events(events);
        vigie_b.merge_received_events(shuffled);

        let mut vigie_a_members = vigie_a.get_members().to_vec();
        vigie_a_members.sort();
        let mut vigie_b_members = vigie_b.get_members().to_vec();
        vigie_b_members.sort();

        assert_eq!(vigie_a_members, vigie_b_members)
    }

    #[fixture]
    fn setup_vigie(
        #[default(Member::default())] member: Member,
    ) -> Vigie<BuiltinMemberStore, BuiltinEffectStore> {
        VigieBuilder::new(member, BuiltinMemberStore::new(), BuiltinEffectStore::new())
            .build()
            .unwrap()
    }

    #[fixture]
    fn setup_member(#[default(0)] ip_offset: u8) -> Member {
        Member::new_ipv4([0, 0, 0, ip_offset], 9000)
    }
}

#[cfg(test)]
mod property_tests {
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
        fn test_merge_received_events((events, shuffled) in arb_merge_received_events(20)) {
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

    proptest! {
        #[test]
        fn test_update_event(events in arb_membership_events(3)) {
            let mut vigie_a = VigieBuilder::new(Member::default(), BuiltinMemberStore::new(), BuiltinEffectStore::new()).build().unwrap();

            for event in events {
                vigie_a.merge_received_events(vec![event]);
                let membership_list_t0 = vigie_a.get_members().to_vec();
                vigie_a.merge_received_events(vec![event]);
                let membership_list_t1 = vigie_a.get_members().to_vec();
                println!(">>>>>>>>>>>>> event: {}, membership_list_t0: {:?}, membership_list_t1: {:?}", event, &membership_list_t0, &membership_list_t1);
                prop_assert_eq!(membership_list_t0, membership_list_t1)
            }
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
        fn arb_membership_events_two_batches(nb_elements: usize)
                                            (member in arb_member())
                                            (
                                                batch1 in prop::collection::vec(arb_membership_event(member), 1..=nb_elements),
                                                batch2 in prop::collection::vec(arb_membership_event(member), 1..=nb_elements)
                                            ) -> (Vec<MembershipEvent>, Vec<MembershipEvent>) {(batch1, batch2)}
    }

    prop_compose! {
        fn arb_membership_events(nb_elements: usize)(events in arb_member().prop_flat_map(move |m| prop::collection::vec(arb_membership_event(m), 1..=nb_elements) ) ) -> Vec<MembershipEvent> { events }
    }

    prop_compose! {
        fn arb_membership_event(member: Member)(entry in arb_membership_entry(member), infection_number in any::<u64>()) -> MembershipEvent {
            MembershipEvent { entry, infection_number }
        }
    }

    prop_compose! {
        fn arb_membership_entry(member: Member)(incarnation_counter in any::<u64>(), status in arb_member_status()) -> MembershipEntry {
            MembershipEntry { member, incarnation_counter, status }
        }
    }

    prop_compose! {
        fn arb_member()(ip in prop::array::uniform4(1u8..u8::MAX), port in any::<u16>()) -> Member {
                Member::new_ipv4(ip, port)
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
