pub(crate) mod logger;
pub(crate) mod timer;

// Public riker::system API (plus the pub data types in this file)
pub use self::timer::{BasicTimer, Timer};

use std::{
    fmt,
    ops::Deref,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::prelude::*;
use config::Config;
use futures::{
    channel::oneshot,
    executor::{ThreadPool, ThreadPoolBuilder},
    future::RemoteHandle,
    task::{SpawnError, SpawnExt},
    Future,
};
use rand;
use uuid::Uuid;

use crate::actor_ref::{
    ActorRef, ActorRefFactory, ActorReference, BasicActorRef, Sender, Tell, TmpActorRefFactory,
};
use crate::{
    actor::{props::ActorFactory, *},
    kernel::provider::{create_root, Provider},
    load_config,
    system::logger::*,
    system::timer::*,
    validate::{validate_name, InvalidPath},
    AnyMessage, Message,
};
use slog::{debug, Logger};

#[derive(Clone, Debug)]
pub enum SystemMsg {
    ActorInit,
    Command(SystemCmd),
    Event(SystemEvent),
    Failed(BasicActorRef),
}

unsafe impl Send for SystemMsg {}

#[derive(Clone, Debug)]
pub enum SystemCmd {
    Stop,
    Restart,
}

impl Into<SystemMsg> for SystemCmd {
    fn into(self) -> SystemMsg {
        SystemMsg::Command(self)
    }
}

#[derive(Clone, Debug)]
pub enum SystemEvent {
    /// An actor was terminated
    ActorCreated(ActorCreated),

    /// An actor was restarted
    ActorRestarted(ActorRestarted),

    /// An actor was started
    ActorTerminated(ActorTerminated),
}

impl Into<SystemMsg> for SystemEvent {
    fn into(self) -> SystemMsg {
        SystemMsg::Event(self)
    }
}

#[derive(Clone, Debug)]
pub struct ActorCreated {
    pub actor: BasicActorRef,
}

#[derive(Clone, Debug)]
pub struct ActorRestarted {
    pub actor: BasicActorRef,
}

#[derive(Clone, Debug)]
pub struct ActorTerminated {
    pub actor: BasicActorRef,
}

impl Into<SystemEvent> for ActorCreated {
    fn into(self) -> SystemEvent {
        SystemEvent::ActorCreated(self)
    }
}

impl Into<SystemEvent> for ActorRestarted {
    fn into(self) -> SystemEvent {
        SystemEvent::ActorRestarted(self)
    }
}

impl Into<SystemEvent> for ActorTerminated {
    fn into(self) -> SystemEvent {
        SystemEvent::ActorTerminated(self)
    }
}

impl Into<SystemMsg> for ActorCreated {
    fn into(self) -> SystemMsg {
        SystemMsg::Event(SystemEvent::ActorCreated(self))
    }
}

impl Into<SystemMsg> for ActorRestarted {
    fn into(self) -> SystemMsg {
        SystemMsg::Event(SystemEvent::ActorRestarted(self))
    }
}

impl Into<SystemMsg> for ActorTerminated {
    fn into(self) -> SystemMsg {
        SystemMsg::Event(SystemEvent::ActorTerminated(self))
    }
}

#[derive(Clone, Debug)]
pub enum SystemEventType {
    ActorTerminated,
    ActorRestarted,
    ActorCreated,
}

pub enum SystemError {
    ModuleFailed(String),
    InvalidName(String),
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::ModuleFailed(ref m) => f.write_str(&format!(
                "Failed to create actor system. Cause: Sub module failed to start ({})",
                m
            )),
            Self::InvalidName(ref name) => f.write_str(&format!(
                "Failed to create actor system. Cause: Invalid actor system name ({})",
                name
            )),
        }
    }
}

impl fmt::Debug for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.to_string().as_str())
    }
}

// 0. error results on any
// 1. visibility

pub struct ProtoSystem {
    id: Uuid,
    name: String,
    pub host: Arc<String>,
    config: Config,
    pub(crate) sys_settings: SystemSettings,
    started_at: DateTime<Utc>,
}

#[derive(Default)]
pub struct SystemBuilder {
    name: Option<String>,
    cfg: Option<Config>,
    log: Option<Logger>,
    exec: Option<ThreadPool>,
}

impl SystemBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(self) -> Result<ActorSystem, SystemError> {
        let name = self.name.unwrap_or_else(|| "riker".into());
        let cfg = self.cfg.unwrap_or_else(load_config);
        let exec = self.exec.unwrap_or_else(|| default_exec(&cfg));
        let log = self.log.unwrap_or_else(|| default_log(&cfg));

        ActorSystem::create(&name, exec, log, &cfg)
    }

    #[must_use]
    pub fn name(self, name: &str) -> Self {
        Self {
            name: Some(name.to_string()),
            ..self
        }
    }

    #[must_use]
    pub fn cfg(self, cfg: Config) -> Self {
        Self {
            cfg: Some(cfg),
            ..self
        }
    }

    #[must_use]
    pub fn exec(self, exec: ThreadPool) -> Self {
        Self {
            exec: Some(exec),
            ..self
        }
    }

    #[must_use]
    pub fn log(self, log: Logger) -> Self {
        Self {
            log: Some(log),
            ..self
        }
    }
}

/// The actor runtime and common services coordinator
///
/// The `ActorSystem` provides a runtime on which actors are executed.
/// It also provides common services such as channels, persistence
/// and scheduling. The `ActorSystem` is the heart of a Riker application,
/// starting serveral threads when it is created. Create only one instance
/// of `ActorSystem` per application.
#[derive(Clone)]
pub struct ActorSystem {
    proto: Arc<ProtoSystem>,
    sys_actors: Option<SysActors>,
    log: Logger,
    debug: bool,
    pub exec: ThreadPool,
    pub timer: TimerRef,
    pub sys_channels: Option<SysChannels>,
    pub(crate) provider: Provider,
}

impl fmt::Display for ActorSystem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.get_tree().as_str())
    }
}

impl ActorSystem {
    /// Create a new `ActorSystem` instance
    ///
    /// Requires a type that implements the `Model` trait.
    pub fn new() -> Result<Self, SystemError> {
        let cfg = load_config();
        let exec = default_exec(&cfg);
        let log = default_log(&cfg);

        Self::create("riker", exec, log, &cfg)
    }

    /// Create a new `ActorSystem` instance with provided name
    ///
    /// Requires a type that implements the `Model` trait.
    pub fn with_name(name: &str) -> Result<Self, SystemError> {
        let cfg = load_config();
        let exec = default_exec(&cfg);
        let log = default_log(&cfg);

        Self::create(name, exec, log, &cfg)
    }

    /// Create a new `ActorSystem` instance bypassing default config behavior
    pub fn with_config(name: &str, cfg: &Config) -> Result<Self, SystemError> {
        let exec = default_exec(cfg);
        let log = default_log(cfg);

        Self::create(name, exec, log, cfg)
    }

    fn create(
        name: &str,
        exec: ThreadPool,
        log: Logger,
        cfg: &Config,
    ) -> Result<Self, SystemError> {
        validate_name(name).map_err(|_| SystemError::InvalidName(name.into()))?;
        // Process Configuration
        let debug = cfg.get_bool("debug").unwrap();

        // Until the logger has started, use println
        if debug {
            debug!(log, "Starting actor system: System[{}]", name);
        }

        let prov = Provider::new(log.clone());
        let timer = BasicTimer::start(cfg);

        // 1. create proto system
        let proto = ProtoSystem {
            id: Uuid::new_v4(),
            name: name.to_string(),
            host: Arc::new("localhost".to_string()),
            config: cfg.clone(),
            sys_settings: SystemSettings::from(cfg),
            started_at: Utc::now(),
        };

        // 2. create uninitialized system
        let mut sys = Self {
            proto: Arc::new(proto),
            debug,
            exec,
            log,
            // event_store: None,
            timer,
            sys_channels: None,
            sys_actors: None,
            provider: prov.clone(),
        };

        // 3. create initial actor hierarchy
        let sys_actors = create_root(&sys);
        sys.sys_actors = Some(sys_actors);

        // 4. start system channels
        sys.sys_channels = Some(sys_channels(&prov, &sys)?);

        // 5. start dead letter logger
        let props = DeadLetterLogger::props(sys.dead_letters(), sys.log());
        let _dl_logger = sys_actor_of_props(&prov, &sys, props, "dl_logger")?;

        sys.complete_start();

        debug!(sys.log, "Actor system [{}] [{}] started", sys.id(), name);

        Ok(sys)
    }

    fn complete_start(&self) {
        self.sys_actors.as_ref().unwrap().user.sys_init(self);
    }

    /// Returns the system start date
    fn start_date(&self) -> &DateTime<Utc> {
        &self.proto.started_at
    }

    /// Returns the number of seconds since the system started
    #[allow(clippy::cast_sign_loss)]
    pub fn uptime(&self) -> u64 {
        let now = Utc::now();
        now.time()
            .signed_duration_since(self.start_date().time())
            .num_seconds() as u64
    }

    /// Returns the hostname used when the system started
    ///
    /// The host is used in actor addressing.
    ///
    /// Currently not used, but will be once system clustering is introduced.
    pub fn host(&self) -> Arc<String> {
        self.proto.host.clone()
    }

    /// Returns the UUID assigned to the system
    pub fn id(&self) -> Uuid {
        self.proto.id
    }

    /// Returns the name of the system
    pub fn name(&self) -> String {
        self.proto.name.clone()
    }

    pub fn print_tree(&self) {
        println!("{}", self.get_tree());
    }

    #[allow(clippy::items_after_statements)]
    pub fn get_tree(&self) -> String {
        let mut tree_str: String = String::new();
        let root = self.sys_actors.as_ref().unwrap().root.clone();

        fn get_node(
            mut tree_str: &mut String,
            sys: &ActorSystem,
            node: &BasicActorRef,
            indent: &str,
        ) -> String {
            if node.is_root() {
                tree_str.push_str(&format!("{}\n", sys.name()));

                for actor in node.children() {
                    get_node(&mut tree_str, sys, &actor, "");
                }
            } else {
                tree_str.push_str(&format!("{}\u{2514}\u{2500} {}\n", indent, node.name()));

                for actor in node.children() {
                    get_node(tree_str, sys, &actor, &(indent.to_string() + "   "));
                }
            }
            (*tree_str).to_string()
        }

        get_node(&mut tree_str, self, &root, "")
    }

    /// Returns the system root's actor reference
    // fn root(&self) -> &BasicActorRef {
    //     &self.sys_actors.as_ref().unwrap().root
    // }

    /// Returns the user root actor reference
    pub fn user_root(&self) -> &BasicActorRef {
        &self.sys_actors.as_ref().unwrap().user
    }

    /// Returns the system root actor reference
    pub fn sys_root(&self) -> &BasicActorRef {
        &self.sys_actors.as_ref().unwrap().sysm
    }

    /// Reutrns the temp root actor reference
    pub fn temp_root(&self) -> &BasicActorRef {
        &self.sys_actors.as_ref().unwrap().temp
    }

    /// Returns a reference to the system events channel
    pub fn sys_events(&self) -> &ActorRef<ChannelMsg<SystemEvent>> {
        &self.sys_channels.as_ref().unwrap().sys_events
    }

    /// Returns a reference to the dead letters channel
    pub fn dead_letters(&self) -> &ActorRef<DLChannelMsg> {
        &self.sys_channels.as_ref().unwrap().dead_letters
    }

    pub fn publish_event(&self, evt: SystemEvent) {
        let topic = Topic::from(&evt);
        self.sys_events().tell(Publish { topic, msg: evt }, None);
    }

    /// Returns the `Config` used by the system
    pub fn config(&self) -> Config {
        self.proto.config.clone()
    }

    pub(crate) fn sys_settings(&self) -> &SystemSettings {
        &self.proto.sys_settings
    }

    /// Create an actor under the system root
    pub fn sys_actor_of_props<A>(
        &self,
        props: BoxActorProd<A>,
        name: &str,
    ) -> Result<ActorRef<A::Msg>, CreateError>
    where
        A: Actor,
    {
        self.provider
            .create_actor(props, name, self.sys_root(), self)
    }

    pub fn sys_actor_of<A>(&self, name: &str) -> Result<ActorRef<<A as Actor>::Msg>, CreateError>
    where
        A: ActorFactory,
    {
        self.provider
            .create_actor(Props::new_no_args(A::create), name, self.sys_root(), self)
    }

    pub fn sys_actor_of_args<A, Args>(
        &self,
        name: &str,
        args: Args,
    ) -> Result<ActorRef<<A as Actor>::Msg>, CreateError>
    where
        Args: ActorArgs,
        A: ActorFactoryArgs<Args>,
    {
        self.provider.create_actor(
            Props::new_args(A::create_args, args),
            name,
            self.sys_root(),
            self,
        )
    }

    #[inline]
    pub fn log(&self) -> Logger {
        self.log.clone()
    }

    /// Returns a future which is completed when all
    /// actors have successfully stopped.
    ///
    /// Note! In the current implementation the future will not complete if
    /// root actor is already terminated.
    pub fn when_terminated(&self) -> Terminated {
        let (tx, rx) = oneshot::channel::<()>();
        let tx = Arc::new(Mutex::new(Some(tx)));

        self.tmp_actor_of_args::<WhenTerminatedActor, _>(tx)
            .unwrap();

        rx
    }

    /// Shutdown the actor system
    ///
    /// Attempts a graceful shutdown of the system and all actors.
    /// Actors will receive a stop message, executing `actor.post_stop`.
    ///
    /// Does not block. Returns a future which is completed when all
    /// actors have successfully stopped.
    pub fn shutdown(&self) -> Shutdown {
        let receiver = self.when_terminated();

        // todo this is prone to failing since there is no
        // confirmation that ShutdownActor has subscribed to
        // the ActorTerminated events yet.
        // It may be that the user root actor is Sterminated
        // before the subscription is complete.

        // std::thread::sleep_ms(1000);
        // send stop to all /user children
        // self.tmp_actor_of_args::<ShutdownActor, _>(tx).unwrap();

        self.stop(self.user_root());

        receiver
    }
}

unsafe impl Send for ActorSystem {}
unsafe impl Sync for ActorSystem {}

impl ActorRefFactory for ActorSystem {
    fn actor_of_props<A>(
        &self,
        props: BoxActorProd<A>,
        name: &str,
    ) -> Result<ActorRef<A::Msg>, CreateError>
    where
        A: Actor,
    {
        self.provider
            .create_actor(props, name, self.user_root(), self)
    }

    fn actor_of<A>(&self, name: &str) -> Result<ActorRef<<A as Actor>::Msg>, CreateError>
    where
        A: ActorFactory,
    {
        self.provider
            .create_actor(Props::new_no_args(A::create), name, self.user_root(), self)
    }

    fn actor_of_args<A, Args>(
        &self,
        name: &str,
        args: Args,
    ) -> Result<ActorRef<<A as Actor>::Msg>, CreateError>
    where
        Args: ActorArgs,
        A: ActorFactoryArgs<Args>,
    {
        self.provider.create_actor(
            Props::new_args(A::create_args, args),
            name,
            self.user_root(),
            self,
        )
    }

    fn stop(&self, actor: impl ActorReference) {
        actor.sys_tell(SystemCmd::Stop.into());
    }
}

impl TmpActorRefFactory for ActorSystem {
    fn tmp_actor_of_props<A>(&self, props: BoxActorProd<A>) -> Result<ActorRef<A::Msg>, CreateError>
    where
        A: Actor,
    {
        let name = format!("{}", rand::random::<u64>());
        self.provider
            .create_actor(props, &name, self.temp_root(), self)
    }

    fn tmp_actor_of<A>(&self) -> Result<ActorRef<<A as Actor>::Msg>, CreateError>
    where
        A: ActorFactory,
    {
        let name = format!("{}", rand::random::<u64>());
        self.provider
            .create_actor(Props::new_no_args(A::create), &name, self.temp_root(), self)
    }

    fn tmp_actor_of_args<A, Args>(
        &self,
        args: Args,
    ) -> Result<ActorRef<<A as Actor>::Msg>, CreateError>
    where
        Args: ActorArgs,
        A: ActorFactoryArgs<Args>,
    {
        let name = format!("{}", rand::random::<u64>());
        self.provider.create_actor(
            Props::new_args(A::create_args, args),
            &name,
            self.temp_root(),
            self,
        )
    }
}

impl ActorSelectionFactory for ActorSystem {
    fn select(&self, path: &str) -> Result<ActorSelection, InvalidPath> {
        let anchor = self.user_root();
        let (anchor, path_str) = if path.starts_with('/') {
            let anchor_path = format!("{}/", anchor.path().deref().clone());
            let path = path.to_string().replace(&anchor_path, "");

            (anchor, path)
        } else {
            (anchor, path.to_string())
        };

        ActorSelection::new(
            anchor.clone(),
            // self.dead_letters(),
            path_str,
        )
    }
}

// futures::task::Spawn::spawn requires &mut self so
// we'll create a wrapper trait that requires only &self.
pub trait Run {
    fn run<Fut>(&self, future: Fut) -> Result<RemoteHandle<<Fut as Future>::Output>, SpawnError>
    where
        Fut: Future + Send + 'static,
        <Fut as Future>::Output: Send;
}

impl Run for ActorSystem {
    fn run<Fut>(&self, future: Fut) -> Result<RemoteHandle<<Fut as Future>::Output>, SpawnError>
    where
        Fut: Future + Send + 'static,
        <Fut as Future>::Output: Send,
    {
        let exec = self.exec.clone();
        exec.spawn_with_handle(future)
    }
}

impl fmt::Debug for ActorSystem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ActorSystem[Name: {}, Start Time: {}, Uptime: {} seconds]",
            self.name(),
            self.start_date(),
            self.uptime()
        )
    }
}

impl Timer for ActorSystem {
    fn schedule<T, M>(
        &self,
        initial_delay: Duration,
        interval: Duration,
        receiver: ActorRef<M>,
        sender: Sender,
        msg: T,
    ) -> Uuid
    where
        T: Message + Into<M>,
        M: Message,
    {
        let id = Uuid::new_v4();
        let msg: M = msg.into();

        let job = RepeatJob {
            id,
            send_at: Instant::now() + initial_delay,
            interval,
            receiver: receiver.into(),
            sender,
            msg: AnyMessage::new(msg, false),
        };

        let _ = self.timer.send(Job::Repeat(job));
        id
    }

    fn schedule_once<T, M>(
        &self,
        delay: Duration,
        receiver: ActorRef<M>,
        sender: Sender,
        msg: T,
    ) -> Uuid
    where
        T: Message + Into<M>,
        M: Message,
    {
        let id = Uuid::new_v4();
        let msg: M = msg.into();

        let job = OnceJob {
            id,
            send_at: Instant::now() + delay,
            receiver: receiver.into(),
            sender,
            msg: AnyMessage::new(msg, true),
        };

        let _ = self.timer.send(Job::Once(job));
        id
    }

    fn schedule_at_time<T, M>(
        &self,
        time: DateTime<Utc>,
        receiver: ActorRef<M>,
        sender: Sender,
        msg: T,
    ) -> Uuid
    where
        T: Message + Into<M>,
        M: Message,
    {
        let delay = std::cmp::max(time.timestamp() - Utc::now().timestamp(), 0 as i64);
        #[allow(clippy::cast_sign_loss)]
        let delay = Duration::from_secs(delay as u64);

        let id = Uuid::new_v4();
        let msg: M = msg.into();

        let job = OnceJob {
            id,
            send_at: Instant::now() + delay,
            receiver: receiver.into(),
            sender,
            msg: AnyMessage::new(msg, true),
        };

        let _ = self.timer.send(Job::Once(job));
        id
    }

    fn cancel_schedule(&self, id: Uuid) {
        let _ = self.timer.send(Job::Cancel(id));
    }
}

// helper functions

fn sys_actor_of_props<A>(
    prov: &Provider,
    sys: &ActorSystem,
    props: BoxActorProd<A>,
    name: &str,
) -> Result<ActorRef<A::Msg>, SystemError>
where
    A: Actor,
{
    prov.create_actor(props, name, sys.sys_root(), sys)
        .map_err(|_| SystemError::ModuleFailed(name.into()))
}

fn sys_actor_of<A>(
    prov: &Provider,
    sys: &ActorSystem,
    name: &str,
) -> Result<ActorRef<<A as Actor>::Msg>, SystemError>
where
    A: ActorFactory,
{
    prov.create_actor(Props::new_no_args(A::create), name, sys.sys_root(), sys)
        .map_err(|_| SystemError::ModuleFailed(name.into()))
}

// fn sys_actor_of_args<A, Args>(
//     prov: &Provider,
//     sys: &ActorSystem,
//     name: &str,
//     args: Args,
// ) -> Result<ActorRef<<A as Actor>::Msg>, SystemError>
// where
//     Args: ActorArgs,
//     A: ActorFactoryArgs<Args>,
// {
//     prov.create_actor(
//         Props::new_args(A::create_args, args),
//         name,
//         &sys.sys_root(),
//         sys,
//     )
//     .map_err(|_| SystemError::ModuleFailed(name.into()))
// }

fn sys_channels(prov: &Provider, sys: &ActorSystem) -> Result<SysChannels, SystemError> {
    let sys_events = sys_actor_of::<EventsChannel>(prov, sys, "sys_events")?;
    let dead_letters = sys_actor_of::<Channel<DeadLetter>>(prov, sys, "dead_letters")?;

    // subscribe the dead_letters channel to actor terminated events
    // so that any future subscribed actors that terminate are automatically
    // unsubscribed from the dead_letters channel
    // let msg = ChannelMsg::Subscribe(SysTopic::ActorTerminated.into(), dl.clone());
    // es.tell(msg, None);

    Ok(SysChannels {
        sys_events,
        dead_letters,
    })
}

pub struct SystemSettings {
    pub msg_process_limit: u32,
}

impl<'a> From<&'a Config> for SystemSettings {
    fn from(config: &Config) -> Self {
        Self {
            msg_process_limit: config.get::<u32>("mailbox.msg_process_limit").unwrap(),
        }
    }
}

struct ThreadPoolConfig {
    pool_size: usize,
}

impl<'a> From<&'a Config> for ThreadPoolConfig {
    fn from(config: &Config) -> Self {
        Self {
            pool_size: config.get::<usize>("dispatcher.pool_size").unwrap(),
        }
    }
}

fn default_exec(cfg: &Config) -> ThreadPool {
    let exec_cfg = ThreadPoolConfig::from(cfg);
    ThreadPoolBuilder::new()
        .pool_size(exec_cfg.pool_size)
        .name_prefix("pool-thread-#")
        .create()
        .unwrap()
}

#[derive(Clone)]
pub struct SysActors {
    pub root: BasicActorRef,
    pub user: BasicActorRef,
    pub sysm: BasicActorRef,
    pub temp: BasicActorRef,
}

#[derive(Clone)]
pub struct SysChannels {
    pub sys_events: ActorRef<ChannelMsg<SystemEvent>>,
    pub dead_letters: ActorRef<DLChannelMsg>,
}

pub type Shutdown = oneshot::Receiver<()>;
pub type Terminated = oneshot::Receiver<()>;

#[derive(Clone)]
struct WhenTerminatedActor {
    tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl ActorFactoryArgs<Arc<Mutex<Option<oneshot::Sender<()>>>>> for WhenTerminatedActor {
    fn create_args(tx: Arc<Mutex<Option<oneshot::Sender<()>>>>) -> Self {
        Self::new(tx)
    }
}

impl WhenTerminatedActor {
    fn new(tx: Arc<Mutex<Option<oneshot::Sender<()>>>>) -> Self {
        Self { tx }
    }
}

impl Actor for WhenTerminatedActor {
    type Msg = SystemEvent;

    fn pre_start(&mut self, ctx: &Context<Self::Msg>) {
        let sub = Subscribe {
            topic: SysTopic::ActorTerminated.into(),
            actor: Box::new(ctx.myself.clone()),
        };
        ctx.system.sys_events().tell(sub, None);
    }

    fn recv(&mut self, _: &Context<Self::Msg>, _: Self::Msg, _: Option<BasicActorRef>) {}

    fn sys_recv(
        &mut self,
        ctx: &Context<Self::Msg>,
        msg: SystemMsg,
        sender: Option<BasicActorRef>,
    ) {
        if let SystemMsg::Event(evt) = msg {
            if let SystemEvent::ActorTerminated(terminated) = evt {
                self.receive(ctx, terminated, sender);
            }
        }
    }
}

impl Receive<ActorTerminated> for WhenTerminatedActor {
    type Msg = SystemEvent;

    //noinspection RsBorrowChecker
    fn receive(
        &mut self,
        ctx: &Context<Self::Msg>,
        msg: ActorTerminated,
        _sender: Option<BasicActorRef>,
    ) {
        if &msg.actor == ctx.system.user_root() {
            if let Ok(ref mut tx) = self.tx.lock() {
                if let Some(tx) = tx.take() {
                    tx.send(()).unwrap();
                }
            }
        }
    }
}
