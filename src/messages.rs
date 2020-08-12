use {actix::prelude::*, bytes::Bytes};

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

/// EncoderCommand: command to dispatch to encoders
#[derive(Message)]
#[rtype(result = "usize")]
pub struct EncoderCommand {
    pub msg: String,
}

/// EncoderMessage: message sent by encoders to server
#[derive(Debug)]
pub enum EncoderMessageType {
    ID,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct EncoderMessage(pub String);

impl EncoderMessage {
    pub fn new(msg_type: EncoderMessageType) -> Self {
        EncoderMessage(format!("/{:?}", msg_type))
    }
}
