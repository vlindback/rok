# rok-jobs — Architecture

A work-stealing job system. The Engine and Target submit closures; a fixed
pool of worker threads runs them. Fences let the submitter wait for groups
of jobs to finish, optionally helping execute work while waiting instead of
parking the calling thread.

The scope is deliberately small. This is the minimum primitive surface the
rest of the engine can be built on without forcing an architectural rewrite
later — work-stealing scheduling, priority levels, fences with helping,
parallel_for, and per-job timing. Anything beyond that (continuations,
dependency graphs, dynamic priority boost, etc.) is out of scope until a
real workload demands it. See "Design decisions" below for what was
considered and rejected.

## What lives where

```
JobSystem            public entry — thin wrapper over JobScheduler
  └─ JobScheduler    holds workers, pool, panic state, the TSC timer
       ├─ JobPool                fixed arena of Job slots, lock-free free list
       ├─ Vec<JobWorkerShared>   per-worker shared state (stealers, inbox, unparker)
       └─ Vec<JobWorkerHandle>   the actual std::thread::JoinHandles

Per worker thread, on the worker's stack:
  JobWorkerLocal     owns the worker's deques + parker + steal loot buffer
                     and a *const into the matching JobWorkerShared
```

The split between `JobWorkerLocal` (thread-local, on the worker's stack) and
`JobWorkerShared` (in the scheduler's Vec, behind `CachePadded`) is the
single most important shape to remember: anything other threads need to
touch lives in Shared; anything only the owning worker touches lives in
Local. That's how we get away with no locks on the hot path.

## Submission: the path a closure takes

```
JobSystem::submit(closure)
  → JobBuilder (priority, fence, record)
    → .dispatch()
       → JobScheduler::schedule()
          → JobPool::try_push() to claim a slot       (lock-free)
          → if caller is a worker thread (TLS hit):
              push to its own local deque
          → else (external thread):
              round-robin into a worker's shared inbox
          → unpark that worker
```

A `Job` is 64 bytes, repr(C, align(16)), with the closure stored inline via
`MaybeUninit<Aligned<...>>`. The closure must fit in the remaining storage
after the run/drop fn pointers, fence pointer, and record pointer — there's
a `const` assertion in `Job::new` that fires at compile time if you try to
stuff something too big in. If a closure ever grows past the budget, the
fix is to box it at the call site, not to grow the slot.

The slot itself is owned by `JobPool`, which is a `Box<[UnsafeCell<Job>]>`
of fixed capacity plus a `crossbeam::queue::ArrayQueue<u32>` holding free
indices. `try_push` pops an index, writes the Job into the slot, returns a
`JobIndex`. `free` pushes the index back. ABA isn't a concern because
nobody holds long-lived references to slots (we don't have continuations).

## Why the inbox exists

Each worker has both:
- A `crossbeam::deque::Worker<JobIndex>` per priority — its **local deque**
  (LIFO, owned, no lock on the worker's own pushes/pops)
- An `ArrayQueue<JobIndex>` per priority — its **shared inbox** (MPSC, used
  by anyone who isn't this worker)

The reason for the inbox: a non-worker thread (the main thread, an OS
callback, the logger thread, anyone) cannot push to another thread's
`Worker<T>` deque. So external submissions land in the target worker's
inbox, and the worker drains its own inbox into its local deque at the top
of each loop iteration. Workers submitting to themselves skip the inbox
entirely and push directly to their deque (fast path).

## The worker loop

Each worker runs `job_worker_loop(local)` which is:

1. **Local deque**, highest priority first. Got one? Run it, continue.
2. **Drain own inbox** (highest to lowest priority). Found anything? Continue
   so the next iteration runs from the local deque.
3. **Steal** from other workers via `JobScheduler::try_steal`. Steals come
   back in priority order in `loot[]`; the worker runs `loot[0]` immediately
   and pushes the rest to its local deque.
4. **Backoff** if everything came up empty:
   - 32 × `spin_loop()`
   - 16 × `yield_now()`
   - `parker.park()` — only after both above fail
   
   Each step checks `has_work_anywhere()` before continuing, so a freshly
   pushed job can short-circuit out of backoff without parking.

Stealing is **batch stealing with a loot bag** — `MAX_STEAL_BATCH = 8`. The
thief walks priorities outer, workers inner, taking what's available until
the bag is full. Priority outer is what makes the priority-sort guarantee
hold: `loot[0]` is always the highest-priority item found.

## Fences

A `JobFence` is just an `AtomicI32` counter plus a `Mutex<()>`/`Condvar`
pair. The lifecycle:

```
scheduler.schedule(.., Some(&fence), ..)
  → fence.increment(1)   BEFORE the job is pushed (avoids 0→neg race)
  → job runs
  → on completion, Job::execute calls fence.decrement()
  → when decrement brings count to 0, notify_all() under the mutex
```

The mutex isn't there to protect the count — that's atomic. It's there to
close the lost-wakeup window: a thread that has just checked the count and
is about to park can't be skipped by a `notify_all` that happens after the
check but before the park.

### Why the mutex exists at all (and not just an atomic + condvar)

It's the standard "monitor" pattern. Without taking the lock on the signal
side, you can hit:

```
WAITER                          SIGNALER
checks is_complete() → false
                                fetch_sub → 0
                                notify_all()  ← no one listening
cv.wait(...)                    ← parked forever
```

Locking briefly on the signal side serializes against the waiter's
wait-while transition, so the notify either happens before the wait (and is
absorbed by the wait-while re-check) or after (and wakes the now-parked
waiter).

### Wait, with help

`JobFence::wait()` is special: if the caller is itself a worker thread (TLS
hit on `JOB_WORKER_TLS`), it doesn't block — it actively runs other jobs
until the fence completes. This is the **spin-help** model. Parking a
worker while it's waiting on a fence is bad because:

- Workers are a finite resource; one fewer worker draining queues means
  the jobs the fence is waiting on take longer to clear.
- The worst case at level-load is a wide fan-out graph with one fence at
  the bottom. Spin-help directly accelerates the unblock; park-and-wake
  adds latency and reduces throughput.

For non-worker threads (Engine main thread submitting setup work, etc.)
`wait()` parks on the condvar normally.

`wait_spin()` is the "I have a hard deadline" variant for time-sensitive
threads — pure busy loop with an acquire fence at the end. Use sparingly.

## parallel_for

`JobSystem::parallel_for(iter, f)` is conceptually:

```rust
for item in iter {
    schedule(move || f(item))
}
```

That's literally it — one job per iterator item. Granularity control is
the caller's job via `.chunks(n)` or `.step_by(n)` on the iterator before
passing it in. There is intentionally no "minimum work per job" heuristic
because:

1. Jobs are 64 bytes and dispatch is cheap (lock-free pool alloc + deque
   push, both wait-free).
2. The caller knows the per-item cost better than the scheduler does.

The integration tests (`parallel_for_no_duplicates_all_visited`,
`parallel_for_sum_of_squares_matches_serial`,
`parallel_for_chunked_no_duplicates_all_visited`) are the canonical
correctness checks — they catch duplicate-execution, lost-work, and
ordering bugs.

## Panics

If a job panics:

1. `Job::execute` runs the closure inside `catch_unwind` and returns the
   panic payload as `Option<Box<dyn Any + Send>>`.
2. `JobScheduler::run_job` stores the **first** panic payload it sees in a
   `Mutex<Option<Box<dyn Any + Send>>>`. Subsequent panics are dropped —
   we keep the first one because that's usually the root cause.
3. `JobSystem::check_panics()` (callable from the owning thread only,
   debug-asserted) takes the stored payload and `resume_unwind`s it on the
   calling thread.

The fence still decrements even when the closure panics — `Job::execute`
runs the decrement after `catch_unwind` regardless of outcome. This avoids
deadlocking waiters on a poisoned fence.

The poisoning of *future* submissions after a panic is **not yet
implemented** — currently a panicked job is recorded, but new schedules
continue normally. See "Open work" below.

## TSC timing and JobRecord

Each worker has a `TscTimer` calibrated at scheduler creation
(`tsc_per_ns` measured against `Instant`, picking the run with the highest
TSC delta to minimize OS jitter contamination).

If a `JobRecord` is attached via `.with_record(&record)`, `run_job` wraps
the execution in `timer.measure(...)` and feeds the nanosecond delta into
`JobRecord::record(ns)`. The record holds lifetime min/avg/max + count, and
optionally a `RingBuffer` for rolling stats over the last N samples.

No cross-core TSC guard — invariant TSC has been standard since ~2008
(Intel Nehalem / AMD Bulldozer). Pre-2014 hardware gets inaccurate
measurements, not a crash.

## Shutdown

`Drop for JobScheduler`:

1. Sets the `StopSource` flag.
2. Unparks every worker so they see the flag immediately rather than
   waiting out their backoff cycle.
3. Joins every worker handle.

Workers check `stop_token.is_stop_requested()` at the top of every loop
iteration. There's no attempt to drain pending work on shutdown — anything
still in the pool or deques is dropped (the Job's drop closure runs, the
closure's destructor runs, done).

## Lifetimes and the unsafe story

The thing that makes this work in Rust is that fences and records are
borrowed via `&'sys T` on the builder, and `JoinHandle<'fence>` carries the
same lifetime parameter. The borrow checker enforces that the fence
outlives the `JoinHandle`, which outlives the wait. We store raw pointers
in `Job` itself because Job lives in the pool (`'static`-like ownership)
but the borrow on the builder side proves the pointer stays valid for as
long as anything could observe it.

`SendPtr<T>` is the escape hatch for sending the `*const
CachePadded<JobWorkerShared>` to the worker thread closure. Safe because:
- The Vec<JobWorkerShared> is in the Arc<JobScheduler>
- The Arc is cloned into every worker thread
- Therefore the pointer is valid for the worker's entire lifetime

The `JobWorkerTLSGuard` ensures `JOB_WORKER_TLS` is set on entry and
cleared on exit — used by `JobFence::wait()` and the fast-path submission
in `schedule()` to detect "am I a worker?"

## Design decisions

Things that were considered and intentionally not built, so future-you
doesn't redo the analysis from scratch.

### Continuations / dependency graphs — removed

An earlier iteration of this system had continuations: a job could attach
follow-up jobs that fire on completion, building a small dependency graph
implicitly. It was removed. The exact triggering insight is lost to
memory, but the core argument was: **fences already cover the cases we
actually have.** "Fan out N jobs, wait for all" is a fence. "Run A then
B" is two submissions with a fence between them. The dependency-graph
machinery (long-lived slot references, generational ABA protection,
continuation storage in the Job struct) paid for capability nobody was
using yet.

What this buys us today:
- `JobPool` is a plain index pool, no generational tag
- `Job` is 64 bytes and holds no follow-up state
- The lifetime story is simpler: no slot reference outlives its execution

If a real workload later needs a true dependency graph (skinning →
animation → physics chains, GPU readback callbacks, etc.) the cleaner
answer will probably be a graph layer *above* the scheduler that submits
in topological waves, not putting graph state back into `Job`.

### No parking helper threads

Workers can park (when fully starved after spin + yield), but threads
calling `JobFence::wait()` from inside a worker context do **not** park —
they help. The reasoning is in "Wait, with help" above. The version where
waiters parked was simpler to reason about but stalled the queue-draining
process and made wide fan-out graphs much slower to clear.

### One ArrayQueue per priority, not one queue with priority tags

The inboxes are `[ArrayQueue<JobIndex>; JobPriority::COUNT]` rather than a
single priority-tagged queue. This trades a small memory cost for
constant-time per-priority drain and lets `drain_shared_inbox(prio)` be a
plain loop rather than a filter pass. It also keeps the data structure
choice (lock-free MPSC bounded queue) honest — a single queue with mixed
priorities would need a richer data structure.

### Closure stored inline in Job, not boxed

The 64-byte slot with a compile-time size check is deliberately
restrictive. Boxing every closure would simplify the type story but adds
an allocation per submission, which we explicitly don't want in steady
state. Closures that don't fit must box themselves at the call site — a
visible cost, not a hidden one.

### Crossbeam as a dependency

`crossbeam` is on the edge of what we'd normally write ourselves. We use
it for `Worker/Stealer` (work-stealing deques), `ArrayQueue` (lock-free
bounded MPSC), `Parker/Unparker`, and `CachePadded`. Rolling these
correctly is several months of careful concurrent-data-structure work and
they're already exhaustively tested by the broader ecosystem. The
calculus may change if we ever need behavior `crossbeam` doesn't expose
(custom stealing heuristics, NUMA-aware queues), but until then it stays.

## Open work

Loose ends that are known and intentional:

- **Panic poisoning of new submissions** — after a panic, scheduling new
  jobs probably should error out or no-op. Currently it doesn't.
- **`with_record()` is wired but `record` field on Job isn't dereffed in
  every path** — double-check that the timing path in `run_job` actually
  reads `job.record` correctly. (Code comment in scheduler reads it; needs
  test coverage.)
- **No fence support across DLL boundary yet** — `EngineApi` has the FFI
  surface for `fence_create/wait/is_complete/schedule` but they're declared
  in rok-abi, not wired to the JobSystem yet. The Engine will route them
  when the Target needs them.

## How to test changes

The `scheduler_integration.rs` tests are the smoke screen. If those pass,
the basic submit/run/fence/wait/parallel_for paths work. They are
intentionally aggressive on counts (10_000 items, 100_000 sum) to surface
deque/inbox/steal races that wouldn't show up at low N.

The job system has no microbenchmarks yet. When we add a profiling sink
in-engine, we'll start treating JobRecord history as the primary
performance signal rather than synthetic benches.
