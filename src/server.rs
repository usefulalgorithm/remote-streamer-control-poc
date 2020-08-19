pub use crate::messages::{
    ClientMessage, Connect, Disconnect, EncoderMessage, EncoderMessageType, GetSession, List,
};
use {
    actix::prelude::*,
    rand::{self, rngs::ThreadRng, Rng},
    std::collections::HashMap,
};

/// `RemoteServer` is responsible for managing encoder websocket endpoints
/// and dispatching client messages.
pub struct RemoteServer {
    pub sessions: HashMap<usize, Recipient<ClientMessage>>,
    pub rng: ThreadRng,
}

impl Default for RemoteServer {
    fn default() -> Self {
        Self {
            sessions: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
}

impl Actor for RemoteServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for RemoteServer {
    type Result = usize;
    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        // make a randomly generated usize as new encoder's id...
        let id = self.rng.gen::<usize>();
        // ... and register it
        self.sessions.insert(id, msg.addr);

        // send id back to encoder
        id
    }
}

impl Handler<Disconnect> for RemoteServer {
    type Result = ();
    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.id).unwrap();
    }
}

impl Handler<List> for RemoteServer {
    type Result = MessageResult<List>;
    fn handle(&mut self, _: List, _: &mut Context<Self>) -> Self::Result {
        let mut encoders = Vec::new();
        for key in self.sessions.keys() {
            encoders.push(*key);
        }
        MessageResult(encoders)
    }
}

impl Handler<GetSession> for RemoteServer {
    type Result = MessageResult<GetSession>;
    fn handle(&mut self, msg: GetSession, _: &mut Context<Self>) -> Self::Result {
        let target = msg.0;
        if self.sessions.contains_key(&target) {
            let session = self.sessions.get(&target).unwrap().clone();
            return MessageResult(Some(session));
        }
        MessageResult(None)
    }
}
