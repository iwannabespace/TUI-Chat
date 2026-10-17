use crossterm::style::Color;

#[derive(Clone)]
pub struct ClientObject {
    pub id: u64,
    pub color: Color,
    pub username: String,
}

impl ClientObject {
    pub fn default() -> Self {
        Self {
            id: u64::MAX,
            color: Color::White,
            username: String::new(),
        }
    }

    pub fn new(id: u64, color: Color, username: String) -> Self {
        Self {
            id,
            color,
            username,
        }
    }
}
