use super::*;
use crate::remote::parse_machines;

fn machines(count: usize) -> Vec<Machine> {
    parse_machines(&serde_json::to_string(&(0..count).map(|index| {
        serde_json::json!({"id": format!("m{index:02}"), "label": "Test", "enabled": true})
    }).collect::<Vec<_>>()).unwrap()).unwrap()
}

#[test]
fn healthy_machine_keeps_polling_while_twenty_four_hosts_time_out() {
    let mut schedule = Schedule::default();
    let started = Instant::now();
    schedule.reconcile(machines(25), started);
    let mut active: Vec<(String, u64, u64)> = Vec::new();
    let mut healthy_polls = Vec::new();
    for second in 0..40 {
        let now = started + Duration::from_secs(second);
        for index in (0..active.len()).rev() {
            let (ref id, generation, ready) = active[index];
            if ready <= second {
                assert!(schedule.complete(id, generation, false, now));
                active.swap_remove(index);
            }
        }
        while let Some((machine, generation)) = schedule.next(now, active.len()) {
            if machine.id == "m00" {
                healthy_polls.push(second);
                assert!(schedule.complete(&machine.id, generation, true, now));
            } else {
                active.push((machine.id, generation, second + 8));
            }
        }
        assert!(active.len() <= MAX_CONCURRENT_QUERIES);
    }
    assert_eq!(healthy_polls, vec![0, 5, 10, 15, 20, 25, 30, 35]);
}

#[test]
fn failed_machines_back_off_without_overlapping_queries() {
    let mut schedule = Schedule::default();
    let mut now = Instant::now();
    schedule.reconcile(machines(1), now);
    for delay in [10, 20, 40, 60, 60] {
        let (machine, generation) = schedule.next(now, 0).unwrap();
        assert!(schedule.next(now + Duration::from_secs(100), 1).is_none());
        assert!(schedule.complete(&machine.id, generation, false, now));
        assert!(schedule
            .next(now + Duration::from_secs(delay - 1), 0)
            .is_none());
        now += Duration::from_secs(delay);
    }
    let (machine, generation) = schedule.next(now, 0).unwrap();
    schedule.complete(&machine.id, generation, true, now);
    assert!(schedule.next(now + POLL_INTERVAL, 0).is_some());
}

#[test]
fn removed_machine_results_cannot_revive_a_reenabled_profile() {
    let mut schedule = Schedule::default();
    let now = Instant::now();
    schedule.reconcile(machines(1), now);
    let (machine, generation) = schedule.next(now, 0).unwrap();
    schedule.reconcile(Vec::new(), now);
    assert!(!schedule.complete(&machine.id, generation, true, now));
    schedule.reconcile(machines(1), now);
    assert!(!schedule.complete(&machine.id, generation, true, now));
    assert!(schedule.next(now, MAX_CONCURRENT_QUERIES).is_none());
    assert!(schedule.next(now, 0).is_some());
}

#[test]
fn slow_successful_queries_do_not_add_another_poll_interval() {
    let mut schedule = Schedule::default();
    let now = Instant::now();
    schedule.reconcile(machines(1), now);
    let (machine, generation) = schedule.next(now, 0).unwrap();
    let finished = now + Duration::from_secs(7);
    assert!(schedule.complete(&machine.id, generation, true, finished));
    assert!(schedule.next(finished, 0).is_some());
}
