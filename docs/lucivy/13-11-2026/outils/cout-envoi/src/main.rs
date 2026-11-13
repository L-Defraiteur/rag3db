//! CPU send/dispatch microbenchmark; identical source for both revisions.
//! Preparation only: it is not a performance result or an ingestion benchmark.
use luciole::{Actor, ActorRef, ActorStatus, Priority, Reply, Scheduler};
use luciole::scheduler::ActorContext;
use std::sync::mpsc;
use std::time::{Duration, Instant};

struct Counter {
    count: u64,
    dropped: mpsc::Sender<()>,
}
enum Msg { Add, Fence(Reply<u64>), Stop(Reply<()>) }
impl Actor for Counter {
    type Msg = Msg;
    fn name(&self) -> &'static str { "send_cost_counter" }
    fn priority(&self) -> Priority { Priority::Medium }
    fn handle(&mut self, msg: Msg, _: &ActorContext) -> ActorStatus {
        match msg {
            Msg::Add => self.count += 1,
            Msg::Fence(reply) => reply.send(self.count),
            Msg::Stop(reply) => { reply.send(()); return ActorStatus::Stop; }
        }
        ActorStatus::Continue
    }
}
impl Drop for Counter {
    fn drop(&mut self) { let _ = self.dropped.send(()); }
}
fn send(actor: &ActorRef<Msg>, msg: Msg) {
    assert!(actor.send(msg).is_ok(), "send failed");
}
fn fence(scheduler: &Scheduler, actor: &ActorRef<Msg>) -> u64 {
    let (tx, rx) = luciole::reply();
    send(actor, Msg::Fence(tx));
    scheduler.try_wait(rx, "benchmark_fence").expect("fence failed")
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.len(), 5, "usage: bench worker|pinned producers capacity messages_per_producer");
    let dispatch = &args[1];
    assert!(dispatch == "worker" || dispatch == "pinned");
    let producers: usize = args[2].parse().unwrap();
    let capacity: usize = args[3].parse().unwrap();
    let messages: u64 = args[4].parse().unwrap();
    assert!([1, 4].contains(&producers));
    assert!([0, 1024].contains(&capacity));
    assert!(messages > 0 && messages <= 100_000);
    let scheduler = Scheduler::new(2);
    let (dropped_tx, dropped_rx) = mpsc::channel();
    let (mailbox, mut actor) = luciole::mailbox(capacity);
    let counter = Counter { count: 0, dropped: dropped_tx };
    let workers = if dispatch == "pinned" {
        scheduler.spawn_pinned(counter, mailbox, &mut actor, capacity);
        None
    } else {
        scheduler.spawn(counter, mailbox, &mut actor, capacity);
        Some(scheduler.start())
    };
    let warmup = 10_000;
    for _ in 0..warmup { send(&actor, Msg::Add); }
    assert_eq!(fence(&scheduler, &actor), warmup);
    let begin = Instant::now();
    let mut handles = Vec::new();
    for _ in 0..producers {
        let actor = actor.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..messages { send(&actor, Msg::Add); }
        }));
    }
    for handle in handles { handle.join().expect("producer failed"); }
    let count = fence(&scheduler, &actor) - warmup;
    let seconds = begin.elapsed().as_secs_f64();
    let expected = producers as u64 * messages;
    assert_eq!(count, expected, "lost messages");
    // Exclude shutdown from the measured interval. Old versions acknowledge
    // before destruction: wait for an independent destructor notification.
    let (tx, rx) = luciole::reply();
    send(&actor, Msg::Stop(tx));
    scheduler.try_wait(rx, "benchmark_stop_ack").expect("stop reply lost");
    dropped_rx.recv_timeout(Duration::from_secs(5)).expect("actor not destroyed");
    drop(actor);
    drop(workers);
    println!("{{\"dispatch\":\"{}\",\"producers\":{},\"capacity\":{},\"messages\":{},\"count\":{},\"seconds\":{:.9},\"messages_per_second\":{:.3},\"status\":\"ok\"}}",
        dispatch, producers, capacity, expected, count, seconds, count as f64 / seconds);
}
