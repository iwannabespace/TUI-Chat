#[derive(Clone)]
pub enum Type {
    ClientLog,
    ServerLog,
    ClientMessage(u64),
}

#[derive(Clone)]
pub struct Content {
    pub m_type: Type,
    pub content: String,
}

impl Content {
    pub fn new(m_type: Type, content: String) -> Self {
        Self { m_type, content }
    }
}

pub struct ScreenContent {
    contents: Vec<Content>,
}

impl ScreenContent {
    pub fn new() -> Self {
        Self {
            contents: Vec::new(),
        }
    }

    pub fn push(&mut self, content: Content) {
        self.contents.push(content)
    }

    pub fn retrieve(&self) -> Vec<Content> {
        self.contents.clone()
    }
}
