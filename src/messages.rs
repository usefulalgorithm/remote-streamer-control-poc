use {
    actix::prelude::*,
    bytes::Bytes,
    futures::channel::oneshot::Sender,
    serde::{Deserialize, Serialize},
};

/// Server message types:

/// Connect: encoder connects to server, would respond with a usize as id
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<ClientMessage>,
}

/// Disconnect: encoder disconnects from server
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// ClientMessage: client sends to server, server would dispatch, encoder would send back via tx
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    /// Peer message
    pub msg: Bytes,
    /// Sender to send back result for msg
    pub tx: Sender<i32>,
}

/// List: list available encoders
pub struct List;

impl actix::Message for List {
    type Result = Vec<usize>;
}

/// GetSession: http request sends id to server, server returns a ws endpoint
pub struct GetSession(pub usize);

impl actix::Message for GetSession {
    type Result = Option<Recipient<ClientMessage>>;
}

/// EncoderMessage: message sent between Encoders and Server
#[derive(Debug, Serialize, Deserialize)]
pub enum EncoderMessageType {
    /// Command dispatched by Server to Encoder
    Cmd(String),
    /// Return value of the dispatched command
    CmdRet(i32),
    /// Type for query and response for Encoder ID
    ID(Option<usize>),
}

#[derive(Message, Serialize, Deserialize, Debug)]
#[rtype(result = "()")]
pub struct EncoderMessage(pub EncoderMessageType);
