mod common;

use std::str::FromStr;

use rstest::rstest;
use vigie::{Configuration, Event, Member, MemberList, Vigie, VigieBuilder};

use crate::common::DummyPayload;

#[rstest]
#[tokio::test]
async fn simple_convergence() {
    let mut vigie_0 = VigieBuilder::new()
        .id("127.0.0.1:30000")
        .seed("127.0.0.1:30001")
        .k(1)
        .period(10_000)
        .timeout(1_000)
        .build::<DummyPayload>()
        .unwrap();
    let vigie_0_id = vigie_0.id();

    let mut vigie_1 = VigieBuilder::new()
        .id("127.0.0.1:30001")
        .build::<DummyPayload>()
        .unwrap();
    let vigie_1_id = vigie_0.id();

    if let Event::MemberAlive { id, data: None } = vigie_0.wait().await.unwrap()
        && id == vigie_1_id
    {
    } else {
        panic!()
    }

    if let Event::MemberAlive { id, data: None } = vigie_1.wait().await.unwrap()
        && id == vigie_0_id
    {
    } else {
        panic!()
    }

    let members_from_vigie_0: MemberList = vigie_0.members();
    let members_from_vigie_1: MemberList = vigie_1.members();

    assert_eq!(members_from_vigie_0, members_from_vigie_1)
}
