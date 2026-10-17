use rand::random;
use std::sync::{mpsc::channel, Arc, Mutex};
use tcp_server::*;

mod client_object;
mod tcp_message;
use client_object::*;
use tcp_message::*;

enum Event {
    ClientMessage,
    ClientIdColorAssign((u8, u8, u8)),
    ClientDisconnected,
}

struct Data {
    event: Event,
    id: u64,
    body: Option<Vec<u8>>,
}

impl Data {
    fn new(event: Event, id: u64, body: Option<Vec<u8>>) -> Self {
        Self { event, id, body }
    }
}

fn main() {
    let mut server = TcpServer::new("127.0.0.1:5858").expect("Server couldn't be created");
    let (tx, rx) = channel::<Data>();

    let tx = Arc::new(Mutex::new(tx));
    let new_client_tx = tx.clone();
    let client_disconnect_tx = tx.clone();
    let client_message_tx = tx.clone();

    server.set_nonblocking(true);

    let clients = Arc::new(Mutex::new(Vec::<ClientObject>::new()));

    // let clients_add = clients.clone();
    // let clients_remove = clients.clone();

    server.set_on_client_connected(move |id, addr| {
        println!("New connection from {addr}");
        let color = (random::<u8>(), random::<u8>(), random::<u8>());
        if let Ok(tx) = new_client_tx.lock() {
            println!("{addr}'s id is {id} and color: {color:?}");
            tx.send(Data::new(Event::ClientIdColorAssign(color), id, None))
                .unwrap();
        }
    });

    server.set_on_client_disconnected(move |id| {
        if let Ok(tx) = client_disconnect_tx.lock() {
            tx.send(Data::new(Event::ClientDisconnected, id, None))
                .unwrap();
        }
    });

    server.set_on_message_received(move |id, message| {
        if let Ok(tx) = client_message_tx.lock() {
            tx.send(Data::new(Event::ClientMessage, id, Some(message)))
                .unwrap();
        }
    });

    server.run();

    loop {
        if let Ok(data) = rx.recv() {
            if let Ok(mut clients) = clients.lock() {
                match data.event {
                    Event::ClientMessage => {
                        match bincode::deserialize::<Message>(&data.body.as_ref().unwrap()) {
                            Ok(message) => match message.message_type {
                                MessageType::SetUsername(username) => {
                                    // let username_taken: bool = clients.iter().any(|c| {
                                    //     if let Some(usr) = c.username.clone() {
                                    //         usr == username
                                    //     } else {
                                    //         false
                                    //     }
                                    // });

                                    // if username_taken {
                                    //     let message =
                                    //         Message::new(MessageType::UsernameTaken, None);

                                    //     if let Ok(msg) = bincode::serialize(&message) {
                                    //         let _ = server.send(SendOption::One(data.id), &msg);
                                    //     }
                                    // } else {
                                    let client = clients
                                        .iter_mut()
                                        .find(|client| client.id == data.id)
                                        .unwrap();

                                    client.username = Some(username.clone());

                                    println!("Client {} set username to {}", data.id, username);

                                    let new_message = Message::new(
                                        MessageType::NewClient(data.id, username, client.color),
                                        None,
                                    );

                                    if let Ok(msg) = bincode::serialize(&new_message) {
                                        if server.send(SendOption::AllExcept(data.id), &msg).is_ok()
                                        {
                                            println!("New client info is sent to others!");

                                            for c in clients.iter() {
                                                if c.id != data.id
                                                /*&& c.username.is_some()*/
                                                {
                                                    let message = Message::new(
                                                        MessageType::AlreadyConnectedClient(
                                                            c.id,
                                                            c.username.clone().unwrap(),
                                                            c.color,
                                                        ),
                                                        None,
                                                    );

                                                    if let Ok(message) =
                                                        bincode::serialize(&message)
                                                    {
                                                        let _ = server.send(
                                                            SendOption::One(data.id),
                                                            &message,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    //}
                                }
                                MessageType::Whisper(from, to) => {
                                    if server
                                        .send(SendOption::One(to), &data.body.unwrap())
                                        .is_ok()
                                    {
                                        println!("Whispered to {}", to);
                                    }
                                }
                                MessageType::ClientMessage(_id) => {
                                    if server.send(SendOption::All, &data.body.unwrap()).is_ok() {
                                        println!("Sent message to {} clients", clients.len());
                                    }
                                }
                                _ => {}
                            },
                            Err(_e) => {}
                        }
                    }
                    Event::ClientIdColorAssign(color) => {
                        let message =
                            Message::new(MessageType::IdColorAssign(data.id, color), None);

                        if let Ok(msg) = bincode::serialize(&message) {
                            if server.send(SendOption::One(data.id), &msg).is_ok() {
                                clients.push(ClientObject::new(data.id, None, color));
                            }
                        }
                    }
                    Event::ClientDisconnected => {
                        let message = Message::new(MessageType::ClientDisconnected(data.id), None);

                        if let Ok(msg) = bincode::serialize(&message) {
                            if server.send(SendOption::All, &msg).is_ok() {
                                clients.retain(|cli| cli.id != data.id);
                                println!("Client {} disconnected", data.id);
                                println!("Number of clients remaining: {}", clients.len());
                            }
                        }
                    }
                }
            }
        }
    }
}
