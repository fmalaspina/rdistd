use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    data: Vec<u8>,
    command: Command,
}

#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Copy { file_path: String },
    Run { command: String },
}

fn read_exact(stream: &mut TcpStream, buffer: &mut [u8]) {
    let mut offset = 0;
    while offset < buffer.len() {
        match stream.read(&mut buffer[offset..]) {
            Ok(0) => break, // End of stream
            Ok(n) => offset += n,
            Err(e) => panic!("Failed to read from client: {}", e),
        }
    }
}

fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0u8; 4]; // Fixed-length header size
    read_exact(&mut stream, &mut buffer);

    let message_len = u32::from_be_bytes(buffer);
    let mut message_buffer = vec![0u8; message_len as usize];
    read_exact(&mut stream, &mut message_buffer);

    let message: Message = bincode::deserialize(&message_buffer).unwrap();

    // println!("{:?}", message);

    match message.command {
        Command::Copy { file_path } => {
            std::fs::write(&file_path, &message.data).expect("Failed to save file");
            println!("File content length received: {}", &message.data.len());
        }

        Command::Run { command } => {
            // Handle the "Run" command
            // Add your logic to execute the command here
            println!("Command executed: {}", command);
        }
    }

    // Send a response back to the client if needed
    let response = b"Message received."; // Replace with your own response

    let response_len = (response.len() as u32).to_be_bytes();
    stream
        .write_all(&response_len)
        .expect("Failed to write response length");
    stream
        .write_all(response)
        .expect("Failed to write response");
    stream.flush().expect("Failed to flush stream");
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").expect("Failed to bind");
    println!("Server running on port 8080");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || handle_client(stream));
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
