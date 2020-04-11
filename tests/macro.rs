use std::fmt::Debug;
use std::fmt;
use std::marker::PhantomData;

use riker_macros::{actor};

#[test]
fn impls_test() {
    NewActorMsg::U32(1);

    let _actor = ActorRef::<NewActorMsg> {
        x: PhantomData
    };

    // actor.tell(5, None);
}

// #[derive(Clone, Debug)]
// enum NewActorMsg {
//     U32(u32),
//     String(String),
// }

#[actor(String, u32)]
#[derive(Clone, Default)]
struct NewActor;

impl Actor for NewActor {
    type Msg = NewActorMsg;

    fn handle(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Sender) {

        println!("handling..");
        self.receive(ctx, msg, sender);
    }
}

impl Receive<u32> for NewActor {
    type Msg = NewActorMsg;

    fn receive(&mut self,
                _ctx: &Context<Self::Msg>,
                _msg: u32,
                _sender: Sender) {
        println!("u32");
    }
}

impl Receive<String> for NewActor {
    type Msg = NewActorMsg;

    fn receive(&mut self,
                _ctx: &Context<Self::Msg>,
                _msg: String,
                _sender: Sender) {
        println!("String");
    }
}
struct BasicActorRef;
type Sender = Option<BasicActorRef>;

type Context<T> = Option<T>;

trait Actor: Send + 'static {
    type Msg: Message;

    /// Invoked when an actor is being started by the system.
    ///
    /// Any initialization inherent to the actor's role should be
    /// performed here.
    /// 
    /// Panics in `pre_start` do not invoke the
    /// supervision strategy and the actor will be terminated.
    fn pre_start(&mut self) {

    }

    /// Invoked after an actor has started.
    ///
    /// Any post initialization can be performed here, such as writing
    /// to a log file, emmitting metrics.
    /// 
    /// Panics in `post_start` follow the supervision strategy.
    fn post_start(&mut self) {

    }

    /// Invoked after an actor has been stopped.
    fn post_stop(&mut self) {

    }

    fn sys_receive(&mut self,
                    _msg: Self::Msg) {
        
    }

    fn handle(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Sender);
}

trait Receive<Msg: Message> {
    type Msg: Message;

    fn receive(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Msg,
                sender: Sender);
}

type BoxedTell<T> = Box<dyn Tell<T> + Send + 'static>;

trait Tell<T> : Send + 'static {
    fn tell(&self, msg: T, sender: Option<BasicActorRef>);
    fn box_clone(&self) -> BoxedTell<T>;
}

impl<T> fmt::Debug for BoxedTell<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tell")
    }
}

impl<T> fmt::Display for BoxedTell<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tell")
    }
}

impl<T: 'static> Clone for BoxedTell<T> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

trait Message: Debug + Clone + Send + 'static {}
impl<T: Debug + Clone + Send + 'static> Message for T {}

#[derive(Clone)]
struct ActorRef<T: Message> {
    x: PhantomData<T>,
}

// impl<T: Message> ActorRef<T> {
//     fn send_msg(&self, _msg: T, _sender: Option<BasicActorRef>) {
//         let _a = NewActor::default();
//         // a.receive(msg);
//     }
// }
