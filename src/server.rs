pub use crate::messages::{
    ClientMessage, Connect, Disconnect, EncoderMessage, EncoderMessageType, List, SimpleMessage,
};
use {
    actix::prelude::*,
    futures::executor::block_on,
    rand::{self, rngs::ThreadRng, Rng},
    std::collections::HashMap,
};

/// `RemoteServer` is responsible for managing encoder websocket endpoints
/// and dispatching client messages.
pub struct RemoteServer {
    sessions: HashMap<usize, Recipient<SimpleMessage>>,
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
    async fn send_message(&self, message: &str, addr: &Recipient<SimpleMessage>) -> usize {
        addr.send(SimpleMessage(
            serde_json::to_string(&EncoderMessage(EncoderMessageType::Cmd(message.to_owned())))
                .unwrap(),
        ))
        .await
        .unwrap()
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

impl Handler<ClientMessage> for RemoteServer {
    type Result = Option<usize>;
    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) -> Self::Result {
        if let Some(addr) = self.sessions.get(&msg.target) {
            return Some(block_on(
                self.send_message(std::str::from_utf8(&msg.msg).unwrap(), &addr),
            ));
        }
        None
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
