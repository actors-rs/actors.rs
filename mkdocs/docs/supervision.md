# Fault Tolerance

Riker applications exhibit fault tolerant behavior through self-healing.
This is achieved by supervision - each actor has a supervisor that is responsible for determining what to do if the actor panics.
In Riker, an actor's parent is its supervisor. This 'parental supervision' is a natural fit since the actor system is a hierarchy.

When an actor fails we can't guarantee its state is not corrupted. Its parent has three choices (strategies):

- `Restart` the actor
- `Escalate` up to the next supervisor
- `Stop` the actor

Supervision isolates failures and errors don't leak or cascade. Instead the system can decide the best way
to restore to a clean, working state, or to gracefully stop.

The supervision strategy an actor should use to supervise its children can be set in its `supervisor_strategy` method:

```test
fn supervisor_strategy(&self) -> Strategy { Strategy::Stop }
```

In this case, if a child fails it will choose to stop it.

<!-- prettier-ignore-start -->
!!! note
    If `supervisor_strategy` is not set, the default implementation is `Strategy::Restart`.
<!-- prettier-ignore-end -->

## Mailboxes

An actor has its own mailbox that messages are queued to during message delivery.
When a message is sent to an actor it is added to the actor's mailbox and the actor is then scheduled to run.
If during handling of a message the actor fails (panics) messages can still continue to be sent to the actor
since the mailbox is separate.
This allows the supervisor to handle the failure without losing messages - a restarted actor
will then continue handling the queued messages once it restarts.

An actor's mailbox continues to exist until its actor is stopped or the system is stopped.

## Restart Strategy

```test
fn supervisor_strategy(&self) -> Strategy { Strategy::Restart }
```

The restart strategy attempts to restart the actor in its initial state, which is considered to be uncorrupted.

The sequence followed is:

1. The actor's mailbox is suspended. Messages can be received but they won't be handled
1. All children of the failed actor are sent termination requests
1. Wait for all children to terminate - a non-blocking operation
1. Restart the failed actor
1. Resume the actor's mailbox and message handling

[supervision_restart.rs](https://github.com/actors-rs/actors.rs/blob/master/examples/supervision_restart.rs)

```rust
use actors::actors::*;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Panic;

#[derive(Default)]
struct DumbActor;

impl Actor for DumbActor {
    type Msg = ();

    fn recv(&mut self, _: &Context<Self::Msg>, _: Self::Msg, _: Sender) {}
}

#[actor(Panic)]
#[derive(Default)]
struct PanicActor;

impl Actor for PanicActor {
    type Msg = PanicActorMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        ctx.actor_of::<DumbActor>("child_a").unwrap();

        ctx.actor_of::<DumbActor>("child_b").unwrap();

        ctx.actor_of::<DumbActor>("child_c").unwrap();

        ctx.actor_of::<DumbActor>("child_d").unwrap();
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}

impl Receive<Panic> for PanicActor {
    type Msg = PanicActorMsg;

    fn receive(&mut self, _ctx: &Context<Self::Msg>, _msg: Panic, _sender: Sender) {
        panic!("// TEST PANIC // TEST PANIC // TEST PANIC //");
    }
}

// Test Restart Strategy
#[actor(Panic)]
#[derive(Default)]
struct RestartSup {
    actor_to_fail: Option<ActorRef<PanicActorMsg>>,
}

impl Actor for RestartSup {
    type Msg = RestartSupMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        self.actor_to_fail = ctx.actor_of::<PanicActor>("actor-to-fail").ok();
    }

    fn supervisor_strategy(&self) -> Strategy {
        Strategy::Restart
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender)
    }
}

impl Receive<Panic> for RestartSup {
    type Msg = RestartSupMsg;

    fn receive(&mut self, _ctx: &Context<Self::Msg>, _msg: Panic, _sender: Sender) {
        self.actor_to_fail.as_ref().unwrap().tell(Panic, None);
    }
}

fn main() {
    let sys = ActorSystem::new().unwrap();

    let sup = sys.actor_of::<RestartSup>("supervisor").unwrap();
    // println!("Child not added yet");
    // sys.print_tree();

    println!("Before panic we see supervisor and actor that will panic!");
    std::thread::sleep(Duration::from_millis(500));
    sys.print_tree();

    sup.tell(Panic, None);
    std::thread::sleep(Duration::from_millis(500));
    println!("We should see panic printed, but we still alive and panic actor still here!");
    sys.print_tree();
}
```

## Escalate Strategy

```test
fn supervisor_strategy(&self) -> Strategy { Strategy::Escalate }
```

The escalate strategy moves the decison of how to handle the failure up to the supervisor's parent. This works by failing the current supervisor and its parent will determine how to handle the failure.

The sequence followed is:

1. The actor's mailbox is suspended. Messages can be received but they won't be handled
1. The supervisor escalates and its mailbox is suspended
1. The new supervisor decides which supervision strategy to follow

[supervision_escalate.rs](https://github.com/actors-rs/actors.rs/blob/master/examples/supervision_escalate.rs)

```rust
use actors::actors::*;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Panic;

#[derive(Default)]
struct DumbActor;

impl Actor for DumbActor {
    type Msg = ();

    fn recv(&mut self, _: &Context<Self::Msg>, _: Self::Msg, _: Sender) {}
}

#[actor(Panic)]
#[derive(Default)]
struct PanicActor;

impl Actor for PanicActor {
    type Msg = PanicActorMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        ctx.actor_of::<DumbActor>("child_a").unwrap();

        ctx.actor_of::<DumbActor>("child_b").unwrap();

        ctx.actor_of::<DumbActor>("child_c").unwrap();

        ctx.actor_of::<DumbActor>("child_d").unwrap();
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}

impl Receive<Panic> for PanicActor {
    type Msg = PanicActorMsg;

    fn receive(&mut self, _ctx: &Context<Self::Msg>, _msg: Panic, _sender: Sender) {
        panic!("// TEST PANIC // TEST PANIC // TEST PANIC //");
    }
}

#[actor(Panic)]
#[derive(Default)]
struct EscalateSup {
    actor_to_fail: Option<ActorRef<PanicActorMsg>>,
}

impl Actor for EscalateSup {
    type Msg = EscalateSupMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        self.actor_to_fail = ctx.actor_of::<PanicActor>("actor-to-fail").ok();
    }

    fn supervisor_strategy(&self) -> Strategy {
        Strategy::Escalate
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
        // match msg {
        //     // We just resend the messages to the actor that we're concerned about testing
        //     TestMsg::Panic => self.actor_to_fail.try_tell(msg, None).unwrap(),
        //     TestMsg::Probe(_) => self.actor_to_fail.try_tell(msg, None).unwrap(),
        // };
    }
}

impl Receive<Panic> for EscalateSup {
    type Msg = EscalateSupMsg;

    fn receive(&mut self, _ctx: &Context<Self::Msg>, _msg: Panic, _sender: Sender) {
        self.actor_to_fail.as_ref().unwrap().tell(Panic, None);
    }
}

#[actor(Panic)]
#[derive(Default)]
struct EscRestartSup {
    escalator: Option<ActorRef<EscalateSupMsg>>,
}

impl Actor for EscRestartSup {
    type Msg = EscRestartSupMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        self.escalator = ctx.actor_of::<EscalateSup>("escalate-supervisor").ok();
    }

    fn supervisor_strategy(&self) -> Strategy {
        Strategy::Restart
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
        // match msg {
        //     // We resend the messages to the parent of the actor that is/has panicked
        //     TestMsg::Panic => self.escalator.try_tell(msg, None).unwrap(),
        //     TestMsg::Probe(_) => self.escalator.try_tell(msg, None).unwrap(),
        // };
    }
}

impl Receive<Panic> for EscRestartSup {
    type Msg = EscRestartSupMsg;

    fn receive(&mut self, _ctx: &Context<Self::Msg>, _msg: Panic, _sender: Sender) {
        self.escalator.as_ref().unwrap().tell(Panic, None);
    }
}

fn main() {
    let sys = ActorSystem::new().unwrap();

    let sup = sys.actor_of::<EscRestartSup>("supervisor").unwrap();

    println!("Before panic we see supervisor and actor that will panic!");
    std::thread::sleep(Duration::from_millis(500));
    sys.print_tree();

    sup.tell(Panic, None);
    std::thread::sleep(Duration::from_millis(500));
    println!("We should see panic printed, but we still alive and panic actor still here!");
    sys.print_tree();
}
```

## Stop Strategy

```test
fn supervisor_strategy(&self) -> Strategy { Strategy::Stop }
```

The stop strategy stops the failed actor, removing it and its mailbox from the system.

[supervision_stop.rs](https://github.com/actors-rs/actors.rs/blob/master/examples/supervision_stop.rs)

```rust
use actors::actors::*;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Panic;

#[derive(Default)]
struct DumbActor;

impl Actor for DumbActor {
    type Msg = ();

    fn recv(&mut self, _: &Context<Self::Msg>, _: Self::Msg, _: Sender) {}
}

#[actor(Panic)]
#[derive(Default)]
struct PanicActor;

impl Actor for PanicActor {
    type Msg = PanicActorMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        ctx.actor_of::<DumbActor>("child_a").unwrap();

        ctx.actor_of::<DumbActor>("child_b").unwrap();

        ctx.actor_of::<DumbActor>("child_c").unwrap();

        ctx.actor_of::<DumbActor>("child_d").unwrap();
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}

impl Receive<Panic> for PanicActor {
    type Msg = PanicActorMsg;

    fn receive(&mut self, _ctx: &Context<Self::Msg>, _msg: Panic, _sender: Sender) {
        panic!("// TEST PANIC // TEST PANIC // TEST PANIC //");
    }
}

// Test Restart Strategy
#[actor(Panic)]
#[derive(Default)]
struct RestartSup {
    actor_to_fail: Option<ActorRef<PanicActorMsg>>,
}

impl Actor for RestartSup {
    type Msg = RestartSupMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        self.actor_to_fail = ctx.actor_of::<PanicActor>("actor-to-fail").ok();
    }

    fn supervisor_strategy(&self) -> Strategy {
        Strategy::Stop
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender)
    }
}

impl Receive<Panic> for RestartSup {
    type Msg = RestartSupMsg;

    fn receive(&mut self, _ctx: &Context<Self::Msg>, _msg: Panic, _sender: Sender) {
        self.actor_to_fail.as_ref().unwrap().tell(Panic, None);
    }
}

fn main() {
    let sys = ActorSystem::new().unwrap();

    let sup = sys.actor_of::<RestartSup>("supervisor").unwrap();
    // println!("Child not added yet");
    // sys.print_tree();

    println!("Before panic we see supervisor and actor that will panic!");
    std::thread::sleep(Duration::from_millis(500));
    sys.print_tree();

    sup.tell(Panic, None);
    std::thread::sleep(Duration::from_millis(500));
    println!("We should see panic printed, but we still alive and panic actor gone!");
    sys.print_tree();
}
```

The output will be

```buildoutcfg
Before panic we see supervisor and actor that will panic!
riker
└─ system
   └─ sys_events
   └─ dead_letters
   └─ dl_logger
└─ temp
└─ user
   └─ supervisor
      └─ actor-to-fail
         └─ child_b
         └─ child_c
         └─ child_d
         └─ child_a

thread 'pool-thread-#2' panicked at '// TEST PANIC // TEST PANIC // TEST PANIC //', examples/supervision_stop.rs:42:9
...

We should see panic printed, but we still alive and panic actor gone!
riker
└─ system
   └─ sys_events
   └─ dead_letters
   └─ dl_logger
└─ temp
└─ user
   └─ supervisor


Process finished with exit code 0
```

## Dead letters

When an actor is terminated all existing `ActorRef`s are invalidated.
Messages sent (using `tell`) are instead rerouted to dead letters, a dedicated channel that publishes undeliverable messages to any interested actors.
Riker has a default subscriber, `dl_logger`, that simply logs dead letter messages using `info!`.

## Supervisor Design

Good supervisor design is key to designing resilient, fault tolerant systems.
At the core of this is creating an actor hierarchy that matches message flow and dependency.

Next we'll see how actor paths can be utilized to message actors without an actor reference and
broadcast to entire segments of the actor hierarchy.

[Actor Selection](selection.md)
