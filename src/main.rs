use serde::{Deserialize, Serialize};
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command as ProcessCommand, Stdio};
#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Copy { file_path: String },
    Run { command: String },
}
#[derive(Serialize, Deserialize, Debug)]
struct Message {
    data: Vec<u8>,
    command: Command,
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
    let resp = match message.command {
        Command::Copy { file_path } => {
            std::fs::write(&file_path, &message.data).expect("Failed to save file");
            format!(
                "File with a {} bytes received and saved in path {}",
                message.data.len(),
                file_path
            )
        }

        Command::Run { command } => {
            let mut first_command = ProcessCommand::new(&command);

            first_command.stdout(Stdio::piped());
            first_command.stderr(Stdio::piped());

            let output = first_command.output().unwrap();

            if let Some(exit_code) = output.status.code() {
                if exit_code == 0 {
                    println!("Ok.");
                } else {
                    eprintln!("Failed.");
                }
            } else {
                eprintln!("Interrupted!");
            }

            format!(
                "{} executed \n Standard output: \n {} \n Standard error: {} \n",
                command,
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            )
        }
    };

    // Send a response back to the client if needed
    let response = resp.as_bytes(); // Replace with your own response

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
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let listener =
            TcpListener::bind(format!("{}:{}", "0.0.0.0", args[1])).expect("Failed to bind");
        println!("Server running on port {}", args[1]);
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    std::thread::spawn(move || handle_client(stream));
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    } else {
        println!("No port provided.")
    }
}
