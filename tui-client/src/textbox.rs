use std::io::{Result, Write};

use crossterm::{
    cursor,
    event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{
        Attribute, Color, ContentStyle, Print, PrintStyledContent, SetBackgroundColor,
        StyledContent, Stylize,
    },
    terminal::{Clear, ClearType},
    QueueableCommand,
};

pub enum BufferAlterState {
    Append(char),
    Delete(char),
    Submit(String),
    ControlC,
    None,
}

pub enum TitleType {
    Text(String),
    RemainingCharacterCountWithMax,
    RemainingCharacterCountWithoutMax,
    None,
}

pub struct Textbox {
    pub buffer: String,
    pub width: u16,
    pub position: (u16, u16),
    pub max_character_count: u16,
    pub title: TitleType,
    pub show_remaining_character_count: bool,
    pub focused: bool,
    pub buffer_style: ContentStyle,
    pub border_style: ContentStyle,
    pub title_style: ContentStyle,
}

impl Textbox {
    pub fn new(
        width: u16,
        position: (u16, u16),
        max_character_count: u16,
        title: TitleType,
        show_remaining_character_count: bool,
        focused: bool,
    ) -> Self {
        Self {
            buffer: String::new(),
            width,
            position,
            max_character_count,
            title,
            show_remaining_character_count,
            focused,
            border_style: ContentStyle::default(),
            buffer_style: ContentStyle::default(),
            title_style: ContentStyle::default(),
        }
    }

    // pub fn draw(&self, stdout: &mut (impl QueueableCommand + Write)) -> Result<()> {
    //     let content = format!(
    //         "{}{}",
    //         self.buffer,
    //         " ".repeat(self.width as usize - self.buffer.len())
    //     );

    //     let title = match &self.title {
    //         TitleType::None => String::new(),
    //         TitleType::Text(text) => text.to_string(),
    //         TitleType::RemainingCharacterCountWithMax => {
    //             format!("({}/{})", self.buffer.len(), self.max_character_count)
    //         }
    //         TitleType::RemainingCharacterCountWithoutMax => format!("({})", self.buffer.len()),
    //     };

    //     stdout.queue(cursor::MoveTo(self.position.0, self.position.1 - 2))?;

    //     if !title.is_empty() {
    //         stdout.queue(PrintStyledContent(StyledContent::new(self.style, title)))?;
    //     }

    //     stdout.queue(cursor::MoveTo(self.position.0, self.position.1 - 3))?;

    //     if self.show_remaining_character_count {
    //         let remaining = format!("({}/{})", self.buffer.len(), self.max_character_count);
    //         stdout.queue(cursor::MoveTo(
    //             (self.position.0 + self.width) - remaining.len() as u16 - 1,
    //             self.position.1 + 3,
    //         ))?;
    //         stdout.queue(Clear(ClearType::CurrentLine))?;
    //         stdout.queue(PrintStyledContent(StyledContent::new(
    //             self.style, remaining,
    //         )))?;
    //     }

    //     stdout.queue(cursor::MoveTo(self.position.0, self.position.1 - 1))?;
    //     stdout.queue(PrintStyledContent(StyledContent::new(self.style, &content)))?;
    //     stdout.queue(cursor::MoveTo(
    //         self.position.0 + self.buffer.len() as u16,
    //         self.position.1 - 1,
    //     ))?;

    //     Ok(())
    // }

    pub fn draw(&self, stdout: &mut (impl QueueableCommand + Write)) -> Result<()> {
        let ui_chars = ["━", "┃", "┏", "┓", "┗", "┛"];
        let width = self.width;
        let (x, y) = self.position;

        let title = match &self.title {
            TitleType::None => String::new(),
            TitleType::Text(text) => text.to_string(),
            TitleType::RemainingCharacterCountWithMax => {
                format!("({}/{})", self.buffer.len(), self.max_character_count)
            }
            TitleType::RemainingCharacterCountWithoutMax => format!("({})", self.buffer.len()),
        };

        let top_border = ui_chars[0].repeat(width as usize - 2 - title.len());
        let bottom_border = ui_chars[0].repeat(width as usize - 2);

        stdout.queue(cursor::MoveTo(x, y))?;
        stdout.queue(PrintStyledContent(StyledContent::new(
            self.border_style,
            ui_chars[2],
        )))?;

        if !title.is_empty() {
            stdout.queue(PrintStyledContent(StyledContent::new(
                self.title_style,
                title,
            )))?;
        }

        stdout.queue(PrintStyledContent(StyledContent::new(
            self.border_style,
            top_border,
        )))?;
        stdout.queue(PrintStyledContent(StyledContent::new(
            self.border_style,
            ui_chars[3],
        )))?;
        stdout.queue(cursor::MoveTo(x, y + 1))?;
        stdout.queue(Clear(ClearType::CurrentLine))?;
        // stdout.queue(SetBackgroundColor(
        //     self.buffer_style.foreground_colorr.unwrap(),
        // ))?;
        stdout.queue(PrintStyledContent(StyledContent::new(
            self.border_style,
            ui_chars[1],
        )))?;
        stdout.queue(cursor::MoveTo(x + width - 1, y + 1))?;
        stdout.queue(PrintStyledContent(StyledContent::new(
            self.border_style,
            ui_chars[1],
        )))?;
        stdout.queue(cursor::MoveTo(x, y + 2))?;
        stdout.queue(PrintStyledContent(StyledContent::new(
            self.border_style,
            ui_chars[4],
        )))?;
        stdout.queue(PrintStyledContent(StyledContent::new(
            self.border_style,
            bottom_border,
        )))?;
        stdout.queue(PrintStyledContent(StyledContent::new(
            self.border_style,
            ui_chars[5],
        )))?;

        if self.show_remaining_character_count {
            let remaining = format!("({}/{})", self.buffer.len(), self.max_character_count);
            stdout.queue(cursor::MoveTo(
                (x + width) - remaining.len() as u16 - 1,
                y + 3,
            ))?;
            stdout.queue(Clear(ClearType::CurrentLine))?;
            stdout.queue(PrintStyledContent(StyledContent::new(
                self.title_style,
                remaining,
            )))?;
        }

        stdout.queue(cursor::MoveTo(x + 1, y + 1))?;
        stdout.queue(PrintStyledContent(StyledContent::new(
            self.buffer_style,
            &self.buffer,
        )))?;

        Ok(())
    }

    pub fn on_paste(&mut self, text: &str) {
        self.buffer.push_str(text);
    }

    pub fn on_key_event(&mut self, event: KeyEvent) -> BufferAlterState {
        match event.code {
            KeyCode::Char(c) => {
                if c == 'x' && event.modifiers.contains(KeyModifiers::CONTROL) {
                    BufferAlterState::ControlC
                } else {
                    if self.focused {
                        if event.kind == KeyEventKind::Press {
                            if self.buffer.len() < self.max_character_count as usize {
                                self.buffer.push(c);
                            }
                        }
                    }
                    BufferAlterState::Append(c)
                }
            }
            KeyCode::Backspace => {
                if self.focused {
                    if event.kind == KeyEventKind::Press {
                        if let Some(c) = self.buffer.pop() {
                            BufferAlterState::Delete(c)
                        } else {
                            BufferAlterState::None
                        }
                    } else {
                        BufferAlterState::None
                    }
                } else {
                    BufferAlterState::None
                }
            }
            KeyCode::Enter => {
                if self.focused {
                    if event.kind == KeyEventKind::Press && !self.buffer.is_empty() {
                        let message = self.buffer.clone();
                        self.buffer.clear();
                        BufferAlterState::Submit(message)
                    } else {
                        BufferAlterState::None
                    }
                } else {
                    BufferAlterState::None
                }
            }
            _ => BufferAlterState::None,
        }
    }
}
