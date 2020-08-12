pub use crate::messages;
use {
    actix::prelude::*,
    rand::{self, rngs::ThreadRng, Rng},
    std::collections::HashMap,
};

/// `RemoteServer` is responsible for managing encoder websocket endpoints
/// and dispatching client messages.
pub struct RemoteServer {
    sessions: HashMap<usize, Recipient<messages::Message>>,
    rng: ThreadRng,
}

impl Default for RemoteServer {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
}

impl RemoteServer {
    /// Dispatch message to target
    fn send_message(&self, message: &str, target: usize) {
        if let Some(addr) = self.sessions.get(&target) {
            let _ = addr.do_send(messages::Message(message.to_owned()));
        }
    }
}

impl Actor for RemoteServer {
    type Context = Context<Self>;
}

impl Handler<messages::Connect> for RemoteServer {
    type Result = usize;
    fn handle(&mut self, msg: messages::Connect, _: &mut Context<Self>) -> Self::Result {
        // make a randomly generated usize as new encoder's id...
        let id = self.rng.gen::<usize>();
        // ... and register it
        self.sessions.insert(id, msg.addr);

        // send id back to encoder
        id
    }
}

impl Handler<messages::Disconnect> for RemoteServer {
    type Result = ();
    fn handle(&mut self, msg: messages::Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.id).unwrap();
    }
}

// TODO
impl Handler<messages::ClientMessage> for RemoteServer {
    type Result = Option<usize>;
    fn handle(&mut self, msg: messages::ClientMessage, _: &mut Context<Self>) -> Self::Result {
        if let Some(_) = self.sessions.get(&msg.target) {
            self.send_message(std::str::from_utf8(&msg.msg).unwrap(), msg.target);
            return Some(msg.target);
        }
        None
    }
}

impl Handler<messages::List> for RemoteServer {
    type Result = MessageResult<messages::List>;
    fn handle(&mut self, _: messages::List, _: &mut Context<Self>) -> Self::Result {
        let mut encoders = Vec::new();
        for key in self.sessions.keys() {
            encoders.push(*key);
        }
        MessageResult(encoders)
    }
}
