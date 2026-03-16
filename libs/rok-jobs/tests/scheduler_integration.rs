use rok_jobs::{JobFence, JobSystem};
use std::{
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

#[test]
fn test_single_job_execution() {
    use std::sync::mpsc::channel;

    let system = JobSystem::new();
    let (tx, rx) = channel();
    system
        .submit(move || {
            std::thread::sleep(Duration::from_secs(2));
            tx.send("Done!").unwrap();
        })
        .dispatch()
        .detach();
    system.check_panics();
    let _result = rx.recv().unwrap();
}

#[test]
fn parallel_for_no_duplicates_all_visited() {
    use std::sync::atomic::{AtomicU32, Ordering};

    const N: usize = 10_000;
    let sys = JobSystem::new();
    let fence = JobFence::new();

    // Each slot starts at 0. The job for index i does compare_exchange(0 → 1).
    // If it sees 1 already, someone else ran this index — that's a duplicate.
    let slots: Arc<Vec<AtomicU32>> = Arc::new((0..N).map(|_| AtomicU32::new(0)).collect());
    let duplicates = Arc::new(AtomicUsize::new(0));

    let s = slots.clone();
    let d = duplicates.clone();

    sys.parallel_for(0..N, move |i| {
        if s[i]
            .compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            d.fetch_add(1, Ordering::Relaxed);
        }
    })
    .with_fence(&fence)
    .dispatch()
    .wait();

    assert_eq!(
        duplicates.load(Ordering::Relaxed),
        0,
        "duplicate iterations detected"
    );

    let missed = slots
        .iter()
        .filter(|s| s.load(Ordering::Relaxed) == 0)
        .count();
    assert_eq!(missed, 0, "{} iterations never ran", missed);
}

#[test]
fn parallel_for_sum_of_squares_matches_serial() {
    use std::sync::atomic::{AtomicU64, Ordering};

    // Sum of squares 0..N. Large enough to stress the scheduler.
    const N: u64 = 100_000;
    let expected: u64 = (0..N).map(|i| i * i).sum();

    let sys = JobSystem::new();
    let fence = JobFence::new();
    let total = Arc::new(AtomicU64::new(0));
    let t = total.clone();

    sys.parallel_for(0..N, move |i| {
        t.fetch_add(i * i, Ordering::Relaxed);
    })
    .with_fence(&fence)
    .dispatch()
    .wait();

    assert_eq!(total.load(Ordering::Relaxed), expected);
}

#[test]
fn parallel_for_chunked_no_duplicates_all_visited() {
    use std::sync::atomic::{AtomicU32, Ordering};

    const N: usize = 10_000;
    const CHUNK: usize = 64;

    let sys = JobSystem::new();
    let fence = JobFence::new();

    let slots: Arc<Vec<AtomicU32>> = Arc::new((0..N).map(|_| AtomicU32::new(0)).collect());
    let duplicates = Arc::new(AtomicUsize::new(0));

    // Use step_by to avoid the Vec<Vec<usize>> closure size issue.
    let s = slots.clone();
    let d = duplicates.clone();

    sys.parallel_for((0..N).step_by(CHUNK), move |start| {
        let end = (start + CHUNK).min(N);
        for i in start..end {
            if s[i]
                .compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed)
                .is_err()
            {
                d.fetch_add(1, Ordering::Relaxed);
            }
        }
    })
    .with_fence(&fence)
    .dispatch()
    .wait();

    assert_eq!(
        duplicates.load(Ordering::Relaxed),
        0,
        "duplicate iterations in chunked run"
    );

    let missed = slots
        .iter()
        .filter(|s| s.load(Ordering::Relaxed) == 0)
        .count();
    assert_eq!(missed, 0, "{} iterations never ran in chunked run", missed);
}
