use actix::prelude::*;
use bytes::Bytes;
use rand::{self, rngs::ThreadRng, Rng};
use std::collections::HashMap;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

/// Server message types:

/// Connect: encoder connects to server, would respond with a usize as id
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
}

/// Disconnect: encoder disconnects from server
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// ClientMessage: client sends to server, server would dispatch or reject
#[derive(Message)]
#[rtype(result = "Option<usize>")]
pub struct ClientMessage {
    /// Peer message
    pub msg: Bytes,
    /// Id of the target encoder
    pub target: usize,
}

/// List: list available encoders
pub struct List;

impl actix::Message for List {
    type Result = Vec<usize>;
}

/// `RemoteServer` is responsible for managing encoder websocket endpoints
/// and dispatching client messages.
pub struct RemoteServer {
    sessions: HashMap<usize, Recipient<Message>>,
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
            let _ = addr.do_send(Message(message.to_owned()));
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

impl Handler<ClientMessage> for RemoteServer {
    type Result = Option<usize>;
    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) -> Self::Result {
        if let Some(_) = self.sessions.get(&msg.target) {
            self.send_message(std::str::from_utf8(&msg.msg).unwrap(), msg.target);
            return Some(msg.target);
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
