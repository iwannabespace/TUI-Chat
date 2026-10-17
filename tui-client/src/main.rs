use client_object::ClientObject;
use command::Command;
use crossterm::event::{
    poll, read, EnableBracketedPaste, EnableMouseCapture, Event, MouseEventKind,
};
use crossterm::style::{Attribute, Color, Print, SetBackgroundColor, SetForegroundColor, Stylize};
use crossterm::terminal::{self, Clear, ClearType};
use crossterm::{cursor, ExecutableCommand, QueueableCommand};
use std::fmt::format;
use std::io::{stdout, Result, Write};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use tcp_message::{Message, MessageType};

use tcp_client::*;

mod client_object;
mod command;
mod screen_content;
mod tcp_message;
mod textbox;
use screen_content::{Content, ScreenContent, Type};
use textbox::{BufferAlterState, Textbox, TitleType};

const COMMANDS: [Command; 5] = [
    Command {
        name: "/connect",
        description: "Connect to a server",
        usage: "/connect <ip:port> <username>",
    },
    Command {
        name: "/disconnect",
        description: "Disconnect from the server",
        usage: "/disconnect",
    },
    Command {
        name: "/whisper",
        description: "Send a private message to another user",
        usage: "/whisper <username> <message>",
    },
    Command {
        name: "/exit",
        description: "Exit the program",
        usage: "/exit",
    },
    Command {
        name: "/help",
        description: "List every command",
        usage: "/help"
    }
];

const COMMAND_PREFIX: &str = "/";

fn main() -> Result<()> {
    let mut stdout = stdout();
    let (mut w, mut h) = terminal::size()?;
    let mut running = true;
    let mut client: Option<TcpClient> = None;
    let mut connected = false;
    let mut status_bar_pos = (1 as u16, h - 5);
    let other_clients = Arc::new(Mutex::new(Vec::<ClientObject>::new()));
    let screen_content = Arc::new(Mutex::new(ScreenContent::new()));
    let me = Arc::new(Mutex::new(ClientObject::default()));

    let (tx, rx) = channel::<u8>();
    let tx = Arc::new(Mutex::new(tx));
    let rx = Arc::new(Mutex::new(rx));

    let mut textbox = Textbox::new(
        w,
        (0, h - 4),
        120,
        TitleType::Text("Message".to_string()),
        true,
        true,
    );

    textbox.border_style.attribute(Attribute::Bold);
    textbox.buffer_style.attribute(Attribute::Bold);
    textbox.title_style.attribute(Attribute::Bold);

    terminal::enable_raw_mode()?;
    stdout.execute(SetBackgroundColor(Color::Black))?;
    stdout.execute(SetForegroundColor(Color::White))?;
    stdout.execute(Clear(ClearType::All))?;

    while running {
        if poll(Duration::ZERO)? {
            match read()? {
                Event::Paste(text) => textbox.on_paste(&text),
                Event::Key(event) => match textbox.on_key_event(event) {
                    BufferAlterState::Delete(_) => {
                        stdout.queue(Clear(ClearType::CurrentLine))?;
                    }
                    BufferAlterState::Submit(text) => {
                        if text.starts_with(COMMAND_PREFIX) {
                            let args = text.trim().split_whitespace().collect::<Vec<_>>();
                            let command_name = args[0];

                            match command_name {
                                "/help" => {
                                    let screen_content_clone = screen_content.clone();
                                    let mut help_result = String::new(); 
                                    for command in &COMMANDS {
                                        let cmd = format!("{}\n{}\n{}\n", command.name, command.description, command.usage);
                                        help_result.push_str(cmd.as_str());
                                    }
                                    screen_content_clone.lock().unwrap().push(
                                        Content::new(
                                            Type::ClientLog,
                                            help_result
                                        ),
                                    );
                                },
                                "/connect" => {
                                    if !connected {
                                        if args.len() == 3 {
                                            if args[2].len() > 10 {
                                                let screen_content_clone = screen_content.clone();
                                                screen_content_clone.lock().unwrap().push(
                                                    Content::new(
                                                        Type::ClientLog,
                                                        "Username must be less than 10 characters!"
                                                            .to_string(),
                                                    ),
                                                );
                                            } else {
                                                let me_clone = me.clone();
                                                me_clone.lock().unwrap().username =
                                                    args[2].to_string();
                                                if let Ok(conn) = TcpClient::connect(args[1]) {
                                                    client = Some(conn);
                                                    let screen_content_clone =
                                                        screen_content.clone();
                                                    let tx_clone = tx.clone();
                                                    let other_clients_clone = other_clients.clone();
                                                    client.as_mut().unwrap().set_on_message_received(move
                                                        |data| { 
                                                        if data.len() == 0 {
                                                            tx_clone.lock().unwrap().send(1).unwrap();
                                                            return;
                                                        }

                                                        match bincode::deserialize::<Message>(
                                                            &data,
                                                        ) {
                                                            Ok(message) => match message.message_type {
                                                                MessageType::IdColorAssign(id, color) => {
                                                                    if let Ok(mut me_clone) = me_clone.lock() {
                                                                        me_clone.id = id;
                                                                        me_clone.color = Color::Rgb { r: color.0, g: color.1, b: color.2 };
                                                                        screen_content_clone.lock().unwrap().push(
                                                                            Content::new(
                                                                                Type::ClientLog,
                                                                                format!("Id: {id} Color: ({}, {}, {})", color.0, color.1, color.2)));
                                                                        let _ = tx_clone.lock().unwrap().send(0);
                                                                    }
                                                                }
                                                                MessageType::NewClient(id, username, color) => {
                                                                    let color = Color::Rgb { r: color.0, g: color.1, b: color.2 };
                                                                    other_clients_clone.lock().unwrap().push(ClientObject::new(id, color, username.clone()));

                                                                    screen_content_clone.lock().unwrap().push(
                                                                        Content::new(
                                                                            Type::ClientLog,
                                                                            format!("{} joined to lobby.", username)));
                                                                }
                                                                MessageType::AlreadyConnectedClient(id, username, color) => {
                                                                    let color = Color::Rgb { r: color.0, g: color.1, b: color.2 };
                                                                    other_clients_clone.lock().unwrap().push(ClientObject::new(id, color, username.clone()));

                                                                    screen_content_clone.lock().unwrap().push(
                                                                        Content::new(
                                                                            Type::ClientLog,
                                                                            format!("{}: was already in lobby.", username)
                                                                        )
                                                                    );
                                                                }
                                                                MessageType::ClientMessage(id) => {
                                                                    if let Ok(other_clients) = other_clients_clone.lock() {
                                                                        if let Ok(utf8) = std::str::from_utf8(&message.body.unwrap()) {
                                                                            if let Some(msg_sender) = other_clients.iter().find(|c| c.id == id) {
                                                                                let username = msg_sender.username.clone();
                                                                                let color = msg_sender.color;

                                                                                screen_content_clone.lock().unwrap().push(
                                                                                    Content::new(
                                                                                        Type::ClientMessage(id),
                                                                                        format!("{}: {}", username.with(color), utf8)
                                                                                    )
                                                                                );
                                                                            } else {
                                                                                let me = me_clone.lock().unwrap();
                                                                                let username = me.username.clone();
                                                                                let color = me.color; 

                                                                                screen_content_clone.lock().unwrap().push(
                                                                                    Content::new(
                                                                                        Type::ClientMessage(id),
                                                                                        format!("{}: {}", username.with(color), utf8)
                                                                                    )
                                                                                );
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                MessageType::ClientDisconnected(id) => {
                                                                    if let Ok(mut other_clients) = other_clients_clone.lock() {
                                                                        let username = other_clients.iter().find(|c| c.id == id).expect("Bu id'de biri yok bro?").username.clone();
                                                                        other_clients.retain(|c| c.id != id);
                                                                        screen_content_clone.lock().unwrap().push(
                                                                            Content::new(
                                                                                Type::ClientLog,
                                                                                format!("{} left the lobby.", username)
                                                                            )
                                                                        );
                                                                    }
                                                                }
                                                                MessageType::Whisper(from, _to) => {
                                                                    if let Ok(other_clients) = other_clients_clone.lock() {
                                                                        if let Ok(utf8) = std::str::from_utf8(&message.body.unwrap()) {
                                                                            if let Some(msg_sender) = other_clients.iter().find(|c| c.id == from) {
                                                                                let username = msg_sender.username.clone();
                                                                                let color = msg_sender.color;

                                                                                screen_content_clone.lock().unwrap().push(
                                                                                    Content::new(
                                                                                        Type::ClientMessage(from),
                                                                                        format!("{} (whispering): {}", username.with(color), utf8)
                                                                                    )
                                                                                );
                                                                            } else {
                                                                                let me = me_clone.lock().unwrap();
                                                                                let username = me.username.clone();
                                                                                let color = me.color; 

                                                                                screen_content_clone.lock().unwrap().push(
                                                                                    Content::new(
                                                                                        Type::ClientMessage(from),
                                                                                        format!("{} (whispering): {}", username.with(color), utf8)
                                                                                    )
                                                                                );
                                                                            }
                                                                        }
                                                                    }
                                                                } 
                                                                MessageType::UsernameTaken => todo!(),
                                                                _ => {
                                                                    screen_content_clone.lock().unwrap().push(Content::new(Type::ClientLog, "Received other message type".to_string()));
                                                                }
                                                            },
                                                            Err(_e) => {
                                                                screen_content_clone.lock().unwrap().push(Content::new(Type::ClientLog, "Failed to deserialize message".to_string()));
                                                            }
                                                        }
                                                    }
                                                    );
                                                    client.as_ref().unwrap().receive();
                                                    connected = true;
                                                    stdout.queue(cursor::MoveTo(
                                                        status_bar_pos.0,
                                                        status_bar_pos.1,
                                                    ))?;
                                                    stdout.queue(Clear(ClearType::CurrentLine))?;
                                                    screen_content.lock().unwrap().push(
                                                        Content::new(
                                                            Type::ClientLog,
                                                            "Welcome to lobby!".to_string(),
                                                        ),
                                                    );
                                                } else {
                                                    screen_content.lock().unwrap().push(
                                                        Content::new(
                                                            Type::ClientLog,
                                                            format!(
                                                                "Failed to connect to {}",
                                                                args[1]
                                                            ),
                                                        ),
                                                    );
                                                }
                                            }
                                        } else {
                                            push_command_log(
                                                &mut screen_content.lock().unwrap(),
                                                command_name,
                                            );
                                        }
                                    } else {
                                        screen_content.lock().unwrap().push(Content::new(
                                            Type::ClientLog,
                                            "You are already connected!".to_string(),
                                        ));
                                    }
                                }
                                "/disconnect" => {
                                    if connected {
                                        client.as_ref().unwrap().disconnect();
                                        //client = None;
                                        connected = false;
                                        stdout.queue(cursor::MoveTo(
                                            status_bar_pos.0,
                                            status_bar_pos.1,
                                        ))?;
                                        stdout.queue(Clear(ClearType::CurrentLine))?;
                                        screen_content.lock().unwrap().push(Content::new(
                                            Type::ClientLog,
                                            "Disconnected from server".to_string(),
                                        ));
                                    } else {
                                        screen_content.lock().unwrap().push(Content::new(
                                            Type::ClientLog,
                                            "You are not connected to any server!".to_string(),
                                        ));
                                    }
                                }
                                "/whisper" => {
                                    if connected {
                                        if args.len() >= 3 {
                                            let body = args[2..].join(" ");
                                            if let Some(cli) = other_clients
                                                .lock()
                                                .unwrap()
                                                .iter()
                                                .find(|c| c.username == args[1])
                                            {
                                                let message = Message::new(
                                                    MessageType::Whisper(
                                                        me.lock().unwrap().id,
                                                        cli.id,
                                                    ),
                                                    Some(body.as_bytes().to_vec()),
                                                );

                                                if let Ok(serialized) = bincode::serialize(&message)
                                                {
                                                    if client
                                                        .as_ref()
                                                        .unwrap()
                                                        .send(&serialized)
                                                        .is_ok()
                                                    {
                                                        screen_content.lock().unwrap().push(
                                                            Content::new(
                                                                Type::ClientMessage(1),
                                                                format!(
                                                                    "You whispered to {}: {}",
                                                                    cli.username
                                                                        .clone()
                                                                        .with(cli.color),
                                                                    body
                                                                ),
                                                            ),
                                                        );
                                                    };
                                                }
                                            } else {
                                                screen_content.lock().unwrap().push(Content::new(
                                                    Type::ClientLog,
                                                    format!("{} is not in the lobby.", args[1]),
                                                ));
                                            }
                                        } else {
                                            push_command_log(
                                                &mut screen_content.lock().unwrap(),
                                                "/whisper",
                                            );
                                        }
                                    } else {
                                        screen_content.lock().unwrap().push(Content::new(
                                            Type::ClientLog,
                                            "You are not connected to any server!".to_string(),
                                        ));
                                    }
                                }
                                "/exit" => {
                                    if connected {
                                        client.as_ref().unwrap().disconnect();
                                    }

                                    running = false;
                                }
                                _ => {}
                            }
                        } else {
                            if let Ok(me) = me.lock() {
                                let message = Message::new(
                                    MessageType::ClientMessage(me.id),
                                    Some(text.as_bytes().to_vec()),
                                );

                                if let Ok(serialized) = bincode::serialize(&message) {
                                    let _ = client.as_ref().unwrap().send(&serialized);
                                }
                            }
                        }
                    }
                    BufferAlterState::ControlC => {
                        running = false;
                    }
                    _ => {}
                },
                Event::Resize(nw, nh) => {
                    w = nw;
                    h = nh;
                    textbox.width = w;
                    textbox.position = (0, h - 4);
                    status_bar_pos = (1 as u16, h - 5);
                    stdout.execute(SetBackgroundColor(Color::Black))?;
                    stdout.execute(SetForegroundColor(Color::White))?;
                    stdout.execute(Clear(ClearType::All))?;
                }
                Event::Mouse(_event) => {}
                _ => {}
            }
        }

        if let Ok(rx) = rx.lock() {
            if let Ok(res) = rx.try_recv() {
                match res {
                    0 => {
                        let me_clone = me.clone();
                        let username = me_clone.lock().unwrap().username.clone();
                        let message =
                            Message::new(MessageType::SetUsername(username.clone()), None);

                        match bincode::serialize(&message) {
                            Ok(serialized) => {
                                if client.as_ref().unwrap().send(&serialized).is_ok() {
                                    screen_content.lock().unwrap().push(Content::new(
                                        Type::ClientLog,
                                        format!("Username set as {username}!"),
                                    ));
                                } else {
                                    screen_content.lock().unwrap().push(Content::new(
                                        Type::ClientLog,
                                        "Failed to set username!".to_string(),
                                    ));
                                }
                            }
                            Err(_e) => {
                                screen_content.lock().unwrap().push(Content::new(
                                    Type::ClientLog,
                                    "Failed to serialize username!".to_string(),
                                ));
                            }
                        }
                    }
                    1 => {
                        running = false;
                    }
                    _ => {}
                }
            }
        }

        let available_row_count = (h - 5) as usize;
        let all_messages = screen_content.lock().unwrap().retrieve();
        let last_n_messages = all_messages.iter().rev().take(available_row_count).rev();

        for (i, content) in last_n_messages.enumerate() {
            stdout.queue(cursor::MoveTo(0, i as u16))?;
            stdout.queue(Print(content.content.clone()))?;
        }

        status_bar(&mut stdout, connected, status_bar_pos)?;
        textbox.draw(&mut stdout)?;
        stdout.flush()?;

        sleep(Duration::from_millis(16));
    }

    let _ = client.unwrap().disconnect();
    terminal::disable_raw_mode()?;

    Ok(())
}

fn status_bar(
    stdout: &mut (impl QueueableCommand + Write),
    connected: bool,
    position: (u16, u16),
) -> Result<()> {
    let status = if connected {
        "Status: Online"
    } else {
        "Status: Offline"
    };
    stdout.queue(cursor::MoveTo(position.0, position.1))?;
    stdout.queue(Print(status))?;
    Ok(())
}

fn push_command_log(s_content: &mut ScreenContent, command_name: &str) {
    if let Some(command) = COMMANDS.iter().find(|c| c.name == command_name) {
        s_content.push(Content::new(
            Type::ClientLog,
            format!("Command: {}", command.name.to_string()),
        ));
        s_content.push(Content::new(
            Type::ClientLog,
            format!("Description: {}", command.description.to_string()),
        ));
        s_content.push(Content::new(
            Type::ClientLog,
            format!("Usage: {}", command.usage.to_string()),
        ));
    }
}