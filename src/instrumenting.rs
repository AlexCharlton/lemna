//! Requires feature "instrumented" to be active

use std::cell::UnsafeCell;
use std::time::Instant;

#[cfg(feature = "instrumented")]
use log::info;

#[allow(dead_code)]
type Inst = (String, Instant);

thread_local!(
    static INST_STACK: UnsafeCell<Vec<Inst>> = {
        UnsafeCell::new(vec![])
    }
);

#[allow(dead_code)]
fn inst_stack_push(name: &str, instant: Instant) {
    INST_STACK.with(|r| unsafe { r.get().as_mut().unwrap().push((name.to_string(), instant)) })
}

#[allow(dead_code)]
fn inst_stack_pop() -> Inst {
    INST_STACK.with(|r| unsafe { r.get().as_mut().unwrap().pop().unwrap() })
}

#[cfg(feature = "instrumented")]
pub fn inst(name: &str) {
    superluminal_perf::begin_event(name);
    let now = Instant::now();
    info!("{:?} {} START", &now, name);
    inst_stack_push(name, now);
}

#[cfg(not(feature = "instrumented"))]
pub fn inst(_name: &str) {}

#[cfg(feature = "instrumented")]
pub fn inst_end() {
    superluminal_perf::end_event();
    let (name, prev) = inst_stack_pop();
    let now = Instant::now();
    info!(
        "{:?} {} END; Took {}μs",
        now,
        name,
        now.duration_since(prev).as_micros()
    );
}

#[cfg(not(feature = "instrumented"))]
pub fn inst_end() {}

#[cfg(feature = "instrumented")]
pub fn evt(name: &str) {
    let now = Instant::now();
    info!("{:?} {}", now, name);
}

#[cfg(not(feature = "instrumented"))]
pub fn evt(_name: &str) {}
