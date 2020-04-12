# Riker

[![Build Status](https://travis-ci.org/actors-rs/actors.rs.svg?branch=master?branch=master)](https://travis-ci.org/github/actors-rs/actors.rs)
[![Gitter](https://badges.gitter.im/actors-rs-/community.svg)](https://gitter.im/actors-rs-/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![crates.io](https://meritbadge.herokuapp.com/riker)](https://crates.io/crates/riker)
[![Released API docs](https://docs.rs/riker/badge.svg)](https://docs.rs/riker)

<!-- prettier-ignore-start -->

<!-- toc -->

+ [Attention](#attention)
+ [Overview](#overview)
+ [Example](#example)
+ [Associated Projects](#associated-projects)
+ [Roadmap & Currently in Development](#roadmap--currently-in-development)
+ [Why Riker](#why-riker)
+ [Rust Version](#rust-version)
+ [Contributing](#contributing)
  + [pre-commit](#pre-commit)

<!-- tocstop -->

<!-- prettier-ignore-end -->

## Attention

This fork is WIP at this moment. Some changes may already be out of sync with documentation.
Hopefully I will stabilize API during several weeks

## Overview

Riker is a framework for building modern, concurrent and resilient systems using the Rust language. Riker aims to make working with state and behavior in concurrent systems as easy and scalable as possible. The Actor Model has been chosen to realize this because of the familiar and inherent simplicity it provides while also providing strong guarantees that are easy to reason about. The Actor Model also provides a firm foundation for resilient systems through the use of the actor hierarchy and actor supervision.

Riker provides:

- An Actor based execution runtime
- Actor supervision to isolate and recover from failures
- A modular system
- Concurrency built on `futures::execution::ThreadPool`
- Publish/Subscribe messaging via actor channels
- Message scheduling
- Out-of-the-box, configurable, non-blocking logging
- Persistent actors using Event Sourcing
- Command Query Responsibility Segregation (CQRS)
- Easily run futures

[Website](https://riker.rs) | [API Docs](https://docs.rs/riker)

## Example

`Cargo.toml`:

```toml
[dependencies]
actors = "0.1"
```

`main.rs`:

```rust
use std::time::Duration;
use actors::actors::*;

#[derive(Default)]
struct MyActor;

// implement the Actor trait
impl Actor for MyActor {
    type Msg = String;

    fn recv(&mut self,
                _ctx: &Context<String>,
                msg: String,
                _sender: Sender) {

        println!("Received: {}", msg);
    }
}

// start the system and create an actor
fn main() {
    let sys = ActorSystem::new().unwrap();

    let my_actor = sys.actor_of::<MyActor>("my-actor").unwrap();

    my_actor.tell("Hello my actor!".to_string(), None);

    std::thread::sleep(Duration::from_millis(500));
}
```

## Associated Projects

Official crates that provide additional functionality:

- [riker-cqrs](https://github.com/riker-rs/riker-cqrs): Command Query Responsibility Separation support
- [riker-testkit](https://github.com/riker-rs/riker-testkit): Tools to make testing easier
- [riker-patterns](https://github.com/riker-rs/riker-patterns): Common actor patterns, including `transform!` and 'ask'

## Roadmap & Currently in Development

The next major theme on the project roadmap is clustering and location transparency:

- Remote actors
- Support for TCP and UDP
- Clustering (using vector clocks)
- Distributed data (CRDTs)

## Why Riker

Riker is a full-featured actor model implementation that scales to hundreds or thousands of microservices and that equally can run exceptionally well on resource limited hardware to drive drones, IoT and robotics. The Rust language makes this possible.

Rust empowers developers with control over memory management, requiring no garbage collection and runtime overhead, while also providing modern semantics and expressive syntax such as the trait system. The result is a language that can solve problems equally for Web and IoT.

Riker adds to this by providing a familiar actor model API which in turn makes concurrent, resilient systems programming easy.

## Rust Version

Riker is currently built using the [stable](https://github.com/rust-lang/rust/blob/master/RELEASES.md) release.

## Contributing

Riker is looking for contributors - join the project! You don't need to be an expert in actors, concurrent systems, or even Rust. Great ideas come from everyone.

There are multiple ways to contribute:

- Ask questions. Adding to the conversation is a great way to contribute. Find us on [Gitter](https://gitter.im/actors-rs-/community?utm_source=share-link&utm_medium=link&utm_campaign=share-link).
- Documentation. Our aim is to make concurrent, resilient systems programming available to everyone and that starts with great Documentation.
- Additions to Riker code base. Whether small or big, your Pull Request could make a difference.
- Patterns, data storage and other supporting crates. We are happy to link to and provide full credit to external projects that provide support for databases in Riker's event storage model or implementations of common actor patterns.

### pre-commit

The project is using [pre-commit](https://pre-commit.com/) git hooks to verify committed code
Make sure you install pre-commit
