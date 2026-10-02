use super::*;
use crate::remote_checks::FakeHerdr;

#[test]
fn stale_pets_become_unknown_and_expire() {
    let fake = FakeHerdr::new();
    let agents = herdr::query_session(
        &fake.binary,
        None,
        Some(("m1".into(), "Build".into())),
        agents::parse_agent_list,
        None,
    )
    .unwrap();
    update(&fake.binary, |cache| {
        cache.insert(
            "m1".into(),
            Record {
                updated: Instant::now() - Duration::from_secs(11),
                agents,
            },
        );
    });
    assert_eq!(snapshot(&fake.binary)[0].view.status, "unknown");
    update(&fake.binary, |cache| {
        cache.get_mut("m1").unwrap().updated = Instant::now() - Duration::from_secs(31);
    });
    assert!(snapshot(&fake.binary).is_empty());
}

#[test]
fn removing_profiles_clears_cached_pets() {
    let fake = FakeHerdr::new();
    refresh(&fake.binary);
    assert_eq!(snapshot(&fake.binary).len(), 2);
    std::fs::write(fake.directory.join("herdr.removed"), "").unwrap();
    refresh(&fake.binary);
    assert!(snapshot(&fake.binary).is_empty());
}
