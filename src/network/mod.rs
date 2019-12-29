use std::io::Write;
use std::net::TcpListener;
use std::sync::mpsc::Receiver;

pub fn accept_clients(receiver: Receiver<Vec<u8>>) {
    let listener = TcpListener::bind("0.0.0.0:42795").expect("Could not bind to port");
    listener.set_nonblocking(true).expect("Could not make listener non-blocking");
    let mut clients = vec![];

    'accept: loop {
        match listener.accept() {
            Ok((stream, _address)) => {
                println!("New client");
                clients.push(stream);
            }
            _ => ()
        }

        match receiver.try_recv() {
            Ok(val) => {
                clients.retain(|mut client| {
                    if let Err(_e) = client.write_all(&val) {
                        println!("Could not write to client. Disconnecting client");
                        return false;
                    }
                    true
                })
            },
            _ => {
                std::thread::sleep(std::time::Duration::from_millis(5));
            },
        }
    }
}
