#![crate_name = "actors_rs"]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![allow(clippy::fallible_impl_from)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::pub_enum_variant_names)]
#![allow(clippy::missing_errors_doc)]
mod validate;

pub mod actor;
pub mod kernel;
pub mod system;
pub use crate::actor::actor_ref::*;
pub use crate::actor::*;
pub use crate::system::ActorSystem;
pub use crate::system::SystemBuilder;

use std::any::Any;
use std::env;
use std::fmt;
use std::fmt::Debug;

use config::{Config, File};

#[must_use]
pub fn load_config() -> Config {
    let mut cfg = Config::new();

    cfg.set_default("debug", true).unwrap();
    cfg.set_default("log.level", "debug").unwrap();
    cfg.set_default("log.log_format", "{date} {time} {level} [{module}] {body}")
        .unwrap();
    cfg.set_default("log.date_format", "%Y-%m-%d").unwrap();
    cfg.set_default("log.time_format", "%H:%M:%S%:z").unwrap();
    cfg.set_default("mailbox.msg_process_limit", 1000).unwrap();
    cfg.set_default("dispatcher.pool_size", 4).unwrap();
    cfg.set_default("scheduler.frequency_millis", 50).unwrap();

    // load the system config
    // riker.toml contains settings for anything related to the actor framework and its modules
    let path = env::var("RIKER_CONF").unwrap_or_else(|_| "config/riker.toml".into());
    cfg.merge(File::with_name(&path).required(false)).unwrap();

    // load the user application config
    // app.toml or app.yaml contains settings specific to the user application
    let path = env::var("APP_CONF").unwrap_or_else(|_| "config/app".into());
    cfg.merge(File::with_name(&path).required(false)).unwrap();
    cfg
}

/// Wraps message and sender
#[derive(Debug, Clone)]
pub struct Envelope<T: Message> {
    pub sender: Option<BasicActorRef>,
    pub msg: T,
}

unsafe impl<T: Message> Send for Envelope<T> {}

pub trait Message: Debug + Clone + Send + 'static {}
impl<T: Debug + Clone + Send + 'static> Message for T {}

pub struct AnyMessage {
    pub one_time: bool,
    pub msg: Option<Box<dyn Any + Send>>,
}

impl AnyMessage {
    pub fn new<T>(msg: T, one_time: bool) -> Self
    where
        T: Any + Message,
    {
        Self {
            one_time,
            msg: Some(Box::new(msg)),
        }
    }

    pub fn take<T>(&mut self) -> Result<T, ()>
    where
        T: Any + Message,
    {
        if self.one_time {
            match self.msg.take() {
                Some(m) => {
                    if m.is::<T>() {
                        Ok(*m.downcast::<T>().unwrap())
                    } else {
                        Err(())
                    }
                }
                None => Err(()),
            }
        } else {
            match self.msg.as_ref() {
                Some(m) if m.is::<T>() => Ok(m.downcast_ref::<T>().cloned().unwrap()),
                Some(_) | None => Err(()),
            }
        }
    }
}

impl Clone for AnyMessage {
    fn clone(&self) -> Self {
        panic!("Can't clone a message of type `AnyMessage`");
    }
}

impl Debug for AnyMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("AnyMessage")
    }
}
