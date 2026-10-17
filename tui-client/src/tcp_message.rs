use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    Kicked,
    Timeout(u64),
    ConnectionDropped,
    ClientMessage(u64),
    ServerMessage(u64),
    ClientDisconnected(u64),
    NewClient(u64, String, (u8, u8, u8)),
    AlreadyConnectedClient(u64, String, (u8, u8, u8)),
    IdColorAssign(u64, (u8, u8, u8)),
    SetUsername(String),
    UsernameTaken,
    Whisper(u64, u64),
    MessageDeserializeError,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub message_type: MessageType,
    pub body: Option<Vec<u8>>,
}

impl Message {
    pub fn new(message_type: MessageType, body: Option<Vec<u8>>) -> Self {
        Self { message_type, body }
    }
}
