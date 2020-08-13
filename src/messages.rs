use {
    actix::prelude::*,
    bytes::Bytes,
    serde::{Deserialize, Serialize},
};

#[derive(Message)]
#[rtype(result = "usize")]
pub struct SimpleMessage(pub String);

/// Server message types:

/// Connect: encoder connects to server, would respond with a usize as id
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<SimpleMessage>,
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

/// EncoderMessage: message sent between Encoders and Server
#[derive(Debug, Serialize, Deserialize)]
pub enum EncoderMessageType {
    /// Command dispatched by Server to Encoder
    Cmd(String),
    /// Return value of the dispatched command
    CmdRet(usize),
    /// Type for query and response for Encoder ID
    ID(Option<usize>),
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct EncoderMessage(pub EncoderMessageType);
