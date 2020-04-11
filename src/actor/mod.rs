pub(crate) mod actor;
pub(crate) mod actor_cell;
pub(crate) mod actor_ref;
pub(crate) mod channel;
pub(crate) mod macros;
pub(crate) mod props;
pub(crate) mod selection;
pub(crate) mod uri;

use std::{fmt};

use crate::validate::InvalidName;

// Public riker::actor API (plus the pub data types in this file)
pub use self::{
    actor::{Actor, BoxActor, Receive, Strategy},
    actor_cell::Context,
    actor_ref::{
        ActorRef, ActorRefFactory, ActorReference, BasicActorRef, BoxedTell, Sender, Tell,
        TmpActorRefFactory,
    },
    channel::{
        channel, All, Channel, ChannelMsg, ChannelRef, DLChannelMsg, DeadLetter, EventsChannel,
        Publish, Subscribe, SysTopic, Topic, Unsubscribe, UnsubscribeAll,
    },
    macros::actor,
    props::{ActorArgs, ActorProducer, BoxActorProd, Props},
    selection::{ActorSelection, ActorSelectionFactory},
    uri::{ActorId, ActorPath, ActorUri},
};

#[allow(unused)]
pub type MsgResult<T> = Result<(), MsgError<T>>;

/// Internal message error when a message can't be added to an actor's mailbox
#[doc(hidden)]
#[derive(Clone,Debug)]
pub struct MsgError<T> {
    pub msg: T,
}

impl<T> MsgError<T> {
    pub fn new(msg: T) -> Self {
        MsgError { msg }
    }
}

impl<T> fmt::Display for MsgError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("The actor does not exist. It may have been terminated")
    }
}

/// Error type when an `try_tell` fails on `Option<ActorRef<Msg>>`
#[derive(Debug)]
pub struct TryMsgError<T> {
    pub msg: T,
}

impl<T> TryMsgError<T> {
    pub fn new(msg: T) -> Self {
        TryMsgError { msg }
    }
}

impl<T> fmt::Display for TryMsgError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Option<ActorRef> is None")
    }
}


/// Error type when an actor fails to start during `actor_of`.
#[derive(Debug)]
pub enum CreateError {
    Panicked,
    System,
    InvalidName(String),
    AlreadyExists(ActorPath),
}

impl fmt::Display for CreateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CreateError::Panicked => f.write_str("Failed to create actor. Cause: Actor panicked while starting"),
            CreateError::System => f.write_str("Failed to create actor. Cause: System failure"),
            CreateError::InvalidName(ref name) => {
                f.write_str(&format!("Failed to create actor. Cause: Invalid actor name ({})", name))
            }
            CreateError::AlreadyExists(ref path) => {
                f.write_str(&format!("Failed to create actor. Cause: An actor at the same path already exists ({})", path))
            }
        }
    }
}


impl From<InvalidName> for CreateError {
    fn from(err: InvalidName) -> CreateError {
        CreateError::InvalidName(err.name)
    }
}

/// Error type when an actor fails to restart.
#[derive(Debug)]
pub struct RestartError;

impl fmt::Display for RestartError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Failed to restart actor. Cause: Actor panicked while starting")
    }
}
