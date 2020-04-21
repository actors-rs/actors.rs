# Channels

Riker channels allow for messages to be published to interested subscribers.
Channels are actors so messaging a channel works the same way as any other actor.

## Starting a channel

The `actors_rs::channel` function returns a channel:

```rust
use actors_rs::*;
use std::time::Duration;

#[derive(Clone, Debug)]
struct PowerStatus;


fn main() {
    let sys = ActorSystem::new().unwrap();
    let chan: ChannelRef<PowerStatus> = channel("power-status", &sys).unwrap();

    std::thread::sleep(Duration::from_millis(500));
    sys.print_tree();
}
```

## Subscribing

To subscribe to and receive messages from a channel an actor must support the message type of the channel.

In the above example we created a channel that publishes `PowerStatus` messages,
notifying components in an autonomous vehicle of changes in available battery energy.
Let's create two actors that will subscribe to the channel so they can receive this message:

```test
struct GpsActor {
    chan: ChannelRef<PowerStatus>,
}

struct NavigationActor {
    chan: ChannelRef<PowerStatus>,
}
...

// Each actor would send a Subscribe message to the
// channel, typically in `pre_start`. E.g.:
impl Actor for GpsActor {
    type Msg = GpsActorMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        let topic = Topic::from("*");

        println!("{}: pre_start subscribe to {:?}", ctx.myself.name(), topic);
        let sub = Box::new(ctx.myself());
        self.chan.tell(
            Subscribe {
                actor: sub.clone(),
                topic,
            },
            None,
        );
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}
```

Here we have two actors that each need to receive status changes in available battery power so they can adapt their behavior.
They both must support the `PowerStatus` message that the channel publishes.
You'll notice that we need to use `Box` to create a trait object of `Tell<PowerStatus>`.

The `Subscribe` message is used to subscribe an actor to a channel, which you'll notice requires a topic.
A channel consists of one or more topics, typically that have a common theme.
When a message is published it is published to a specific topic on the channel.

<!-- prettier-ignore-start -->
!!! note
    When subscribing to a topic, if it does't already exist it will be created and any future messages published to it will be sent to the subscriber.
<!-- prettier-ignore-end -->

## Publishing

The `Publish` message is used to publish to a channel:

```test
let stat = PowerStatus { ... };
chan.tell(Publish { msg: PowerStatus, topic: Topic::from("power") }, None);
```

This message will be cloned and sent to each subscriber of `my-topic` on the channel `chan`.

In this case, it may be that the `GpsActor` will choose to lower the GPS sampling rate
if the battery level falls below a certain percentage, thus lowering the power used.
The `NavigationActor` might override any active mission and force the vehicle to return to base
if the power level drops to a critical level.
The same use of channels could be applied to e-commerce platforms, payments systems, warehouse logistics,
shipping tracking, etc.

Here is full example that can be found in [channel.rs](https://github.com/actors-rs/actors.rs/blob/master/examples/channel.rs)

```rust
extern crate actors_rs;
use actors_rs::*;

use actors_rs::system::ActorSystem;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct PowerStatus;

#[actor(PowerStatus)]
struct GpsActor {
    chan: ChannelRef<PowerStatus>,
}

impl ActorFactoryArgs<ChannelRef<PowerStatus>> for GpsActor {
    fn create_args(chan: ChannelRef<PowerStatus>) -> Self {
        GpsActor { chan }
    }
}

impl Actor for GpsActor {
    type Msg = GpsActorMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        let topic = Topic::from("my-topic");

        println!("{}: pre_start subscribe to {:?}", ctx.myself.name(), topic);
        let sub = Box::new(ctx.myself());
        self.chan.tell(
            Subscribe {
                actor: sub.clone(),
                topic,
            },
            None,
        );
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}

impl Receive<PowerStatus> for GpsActor {
    type Msg = GpsActorMsg;

    fn receive(&mut self, ctx: &Context<Self::Msg>, msg: PowerStatus, _sender: Sender) {
        println!("{}: -> got msg: {:?}", ctx.myself.name(), msg);
    }
}

#[actor(PowerStatus)]
struct NavigationActor {
    chan: ChannelRef<PowerStatus>,
}

impl ActorFactoryArgs<ChannelRef<PowerStatus>> for NavigationActor {
    fn create_args(chan: ChannelRef<PowerStatus>) -> Self {
        NavigationActor { chan }
    }
}

impl Actor for NavigationActor {
    type Msg = NavigationActorMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        let topic = Topic::from("my-topic");

        println!("{}: pre_start subscribe to {:?}", ctx.myself.name(), topic);
        let sub = Box::new(ctx.myself());
        self.chan.tell(
            Subscribe {
                actor: sub.clone(),
                topic,
            },
            None,
        );
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}

impl Receive<PowerStatus> for NavigationActor {
    type Msg = NavigationActorMsg;

    fn receive(&mut self, ctx: &Context<Self::Msg>, msg: PowerStatus, _sender: Sender) {
        println!("{}: -> got msg: {:?}", ctx.myself.name(), msg);
    }
}

fn main() {
    let sys = ActorSystem::new().unwrap();
    let chan: ChannelRef<PowerStatus> = channel("power-status", &sys).unwrap();

    sys.actor_of_args::<GpsActor, _>("gps-actor", chan.clone())
        .unwrap();
    sys.actor_of_args::<GpsActor, _>("navigation-actor", chan.clone())
        .unwrap();

    std::thread::sleep(Duration::from_millis(500));
    // sys.print_tree();
    let topic = Topic::from("my-topic");
    println!(
        "Sending PowerStatus message to all subscribers and {:?}",
        topic
    );
    chan.tell(
        Publish {
            msg: PowerStatus,
            topic,
        },
        None,
    );
    // sleep another half seconds to process messages
    std::thread::sleep(Duration::from_millis(500));
    sys.print_tree();
}

```

## Common channels

When the actor system starts several channels are created. These channels help developers receive messages
about system events and failed messages.

### System events

The System Events channel provides events including `ActorCreated`, `ActorRestarted` and `ActorTerminated` events.
Each of these are represented as topic `actor.created`, `actor.restarted` and `actor.terminated` topics respectively.
The message type is `SystemEvent` enum

Example:

[channel_system.rs](https://github.com/actors-rs/actors.rs/blob/master/examples/channel_system.rs)

```rust
use actors_rs::*;

use actors_rs::system::{ActorSystem, SystemEvent, SystemMsg};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Panic;

#[actor(Panic)]
#[derive(Default)]
struct DumbActor;

impl Actor for DumbActor {
    type Msg = DumbActorMsg;

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}

impl Receive<Panic> for DumbActor {
    type Msg = DumbActorMsg;

    fn receive(&mut self, _ctx: &Context<Self::Msg>, _msg: Panic, _sender: Sender) {
        panic!("// TEST PANIC // TEST PANIC // TEST PANIC //");
    }
}

// *** Publish test ***
#[actor(SystemEvent)]
#[derive(Default)]
struct SystemActor;

impl Actor for SystemActor {
    type Msg = SystemActorMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        let topic = Topic::from("*");

        println!("{}: pre_start subscribe to topic {:?}", ctx.myself.name(), topic);
        let sub = Box::new(ctx.myself());

        ctx.system.sys_events().tell(
            Subscribe {
                actor: sub,
                topic: "*".into(),
            },
            None,
        );
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }

    fn sys_recv(&mut self, ctx: &Context<Self::Msg>, msg: SystemMsg, sender: Sender) {
        if let SystemMsg::Event(evt) = msg {
            self.receive(ctx, evt, sender);
        }
    }
}

impl Receive<SystemEvent> for SystemActor {
    type Msg = SystemActorMsg;

    fn receive(&mut self, ctx: &Context<Self::Msg>, msg: SystemEvent, _sender: Sender) {
        print!("{}: -> got system msg: {:?} ", ctx.myself.name(), msg);
        match msg {
            SystemEvent::ActorCreated(created) => {
                println!("path: {}", created.actor.path());
            }
            SystemEvent::ActorRestarted(restarted) => {
                println!("path: {}", restarted.actor.path());
            }
            SystemEvent::ActorTerminated(terminated) => {
                println!("path: {}", terminated.actor.path());
            }
        }
    }
}

fn main() {
    let sys = ActorSystem::new().unwrap();

    let _sub = sys.actor_of::<SystemActor>("system-actor").unwrap();

    std::thread::sleep(Duration::from_millis(500));

    println!("Creating dump actor");
    let dumb = sys.actor_of::<DumbActor>("dumb-actor").unwrap();

    // sleep another half seconds to process messages
    std::thread::sleep(Duration::from_millis(500));

    // Force restart of actor
    println!("Send Panic message to dump actor to get restart");
    dumb.tell(Panic, None);
    std::thread::sleep(Duration::from_millis(500));

    println!("Stopping dump actor");
    sys.stop(&dumb);
    std::thread::sleep(Duration::from_millis(500));
    sys.print_tree();
}
```

<!-- prettier-ignore-start -->
!!! note
    System events are considered system messages and therefore a subscriber's `Actor::sys_recv` method will be invoked instead of `Actor::recv`.
<!-- prettier-ignore-end -->

### Dead letters

The Dead Letters channel publishes messages that failed to be delivered to their destination actor. This channel can be subscribed to to handle those messages. Note: Dead letters use `Debug` representation of the original undelivered message, limiting the use of dead letters to logging of failed messages rather than actually acting upon them.

Example: [channel_dead_letters.rs](https://github.com/actors-rs/actors.rs/blob/master/examples/channel_dead_letters.rs)

```rust
use actors_rs::*;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct SomeMessage;

#[actor(SomeMessage)]
#[derive(Default)]
struct DumbActor;

impl Actor for DumbActor {
    type Msg = DumbActorMsg;

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}

impl Receive<SomeMessage> for DumbActor {
    type Msg = DumbActorMsg;

    fn receive(&mut self, ctx: &Context<Self::Msg>, msg: SomeMessage, _sender: Sender) {
        println!("{}: -> got msg: {:?} ", ctx.myself.name(), msg);
    }
}

// *** Publish test ***
#[actor(DeadLetter)]
#[derive(Default)]
struct DeadLetterActor;

impl Actor for DeadLetterActor {
    type Msg = DeadLetterActorMsg;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        let topic = Topic::from("*");

        println!("{}: pre_start subscribe to topic {:?}", ctx.myself.name(), topic);
        let sub = Box::new(ctx.myself());

        ctx.system
            .dead_letters()
            .tell(Subscribe { actor: sub, topic }, None);
    }

    fn recv(&mut self, ctx: &Context<Self::Msg>, msg: Self::Msg, sender: Sender) {
        self.receive(ctx, msg, sender);
    }
}

impl Receive<DeadLetter> for DeadLetterActor {
    type Msg = DeadLetterActorMsg;

    fn receive(&mut self, ctx: &Context<Self::Msg>, msg: DeadLetter, _sender: Sender) {
        println!("{}: -> got msg: {:?} ", ctx.myself.name(), msg);
    }
}

fn main() {
    let sys = ActorSystem::new().unwrap();

    let _sub = sys.actor_of::<DeadLetterActor>("system-actor").unwrap();

    std::thread::sleep(Duration::from_millis(500));

    println!("Creating dump actor");
    let dumb = sys.actor_of::<DumbActor>("dumb-actor").unwrap();

    println!("Stopping dump actor");
    sys.stop(&dumb);
    std::thread::sleep(Duration::from_millis(500));

    println!("Sending SomeMessage to stopped actor");
    dumb.tell(SomeMessage, None);
    std::thread::sleep(Duration::from_millis(500));
    sys.print_tree();
}
```

Channels form an integral part of the Riker system and provide essential services to creating dynamic applications where actors collaborate to achieve a common goal.

Next we'll look at scheduling messages to be sent at a time in the future.

[Scheduling Messages](scheduling.md)
