pub struct ClientObject {
    pub id: u64,
    pub username: Option<String>,
    pub color: (u8, u8, u8),
}

impl ClientObject {
    pub fn new(id: u64, username: Option<String>, color: (u8, u8, u8)) -> Self {
        Self {
            id,
            username,
            color,
        }
    }
}
