# Review
* README ist nicht vollständig:
```
  -> % cargo run --release --bin server -- --destination /tmp/foo
error: target `server` in package `secsnail` requires the features: `bin-deps`
Consider enabling them by passing, e.g., `--features="bin-deps"`
```
Wenn eure Anwendung ohne dieses Feature gar nicht funktionieren kann, würde ich das auf jeden Fall in die default-features aufnehmen.

* Danke für gar nichts:
```
thread 'main' (9262) panicked at src/bin/client.rs:27:9:
attempt to divide by zero``
```
Commands die ich verwendet habe:  
cargo run --release --bin server -- --destination /tmp/foo  
cargo run --release --bin client -- --file-name /tmp/foo/Cargo.toml --ip 127.0.0.1  

* Fehlende Erklärung was die Parameter machen in der README

---

## State Machine Design Review (by Claude, AI assistant)

### The Problem: Double State Representation

The current state machine implementation has a fundamental clarity issue. Each state exists in **two places**:

1. As a generic type parameter: `SndFsm<SndStateWait>`
2. As an enum variant: `FsmStateWrapper::Wait(SndFsm<SndStateWait>)`

This creates unnecessary complexity:
- Every transition must call `.wrap()` to box the typed FSM back into the enum
- The driver must unwrap the enum to call `.goto()`, then wrap the result again
- The reader must understand both representations to follow the code

### Why the State Pattern doesn't fit here

The typestate pattern (state as generic parameter) is valuable when you want **compile-time guarantees** that certain methods can only be called in certain states. For example, a `File<Open>` that only exposes `.read()` when open.

However, in this implementation, everything flows through `FsmStateWrapper` anyway. The driver does:
```rust
match cur_fsm_wrap {
    FsmStateWrapper::Start(fsm) => fsm.goto(event, ctx)?,
    FsmStateWrapper::Wait(fsm) => fsm.goto(event, ctx)?,
    // ...
}
```

The compile-time type safety is lost at this point - you're back to runtime dispatch. So you pay the complexity cost of generics without getting the benefit.

### The Symptom: Scattered Transitions

To understand the send FSM, I need to read:
- `fsm.rs` for the state structs and wrapper enum
- `start.rs` for Start state transitions
- `wait.rs` for Wait state transitions
- `send.rs` for Send state transitions
- `driver.rs` for the event loop

That's 5 files for a 4-state machine. The state diagram is fragmented across the codebase.

### Hint for a Better Approach

Consider: what if all states lived in **one enum** (with their data), and all transitions lived in **one function**?

```rust
pub enum SndState {
    Start { n: u8 },
    Wait { n: u8, retransmit_counter: u8, sndpkt: Packet },
    Send { n: u8 },
    End,
}
```

A single `transition` method could use `match (self, event)` to handle all state/event combinations. Each match arm becomes a transition arrow from your state diagram.

Benefits:
- One file contains the entire state machine
- The match block **is** the state diagram - you can read it and understand all transitions
- No wrapping/unwrapping ceremony
- Easier to verify completeness (are all transitions covered?)

Think about: How would the driver loop simplify if there's only one enum to match on?

### Secondary Issue: The `next_n` function

```rust
pub fn next_n(n: u8) -> u8 {
    match n {
        0 => 1,
        _ => 0,
    }
}
```

This toggles between 0 and 1, but uses `u8`. A `bool` would be more semantic - the alternating bit protocol is fundamentally binary. Consider whether `seq_bit: bool` with `!seq_bit` for toggling would be clearer than `n: u8` with `next_n(n)`.
