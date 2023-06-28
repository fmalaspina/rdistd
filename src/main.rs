use execute::{shell, Execute};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Stdio;
#[derive(Serialize, Deserialize, Debug)]
enum Command {
    Copy { file_path: String, data: Vec<u8> },
    Run { command: String },
    Rollback { file_path: String },
}
#[derive(Serialize, Deserialize, Debug)]
struct Message {
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
        Command::Copy { file_path, data } => {
            // Generate a unique filename for the previous version
            let previous_version_filename = format!("{}.prev", &file_path);

            // Check if the file already exists
            if std::fs::metadata(&file_path).is_ok() {
                // Create a backup by renaming the existing file
                std::fs::rename(&file_path, &previous_version_filename)
                    .expect("Failed to create backup file");
            }

            std::fs::write(&file_path, &data).expect("Failed to save file");
            // Store the file path in history

            format!(
                "File with {} bytes received and saved at path {}. Previous version stored at {}",
                data.len(),
                file_path,
                previous_version_filename
            )
        }

        Command::Run { command } => {
            let mut first_command = shell(&command);

            first_command.stdout(Stdio::piped());
            first_command.stderr(Stdio::piped());

            let output = first_command.execute_output().unwrap();

            format!(
                "Command: {} (executed)\nStandard output:\n{}\nStandard error:\n{}\nExit code:{}\n",
                command,
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap(),
                output.status.code().unwrap()
            )
        }
        Command::Rollback { file_path } => {
            // Extract the original file path
            // Iterate over files in the current directory
            if let Ok(entries) = std::fs::read_dir(file_path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Some(file_name) = entry.file_name().to_str() {
                            // Check if the file has a .prev suffix
                            if file_name.ends_with(".prev") {
                                // Extract the original file path
                                let original_file_path = file_name.trim_end_matches(".prev");

                                // Check if the original file exists
                                if std::fs::metadata(&original_file_path).is_ok() {
                                    // Restore the previous version by renaming the backup file
                                    std::fs::rename(&file_name, &original_file_path)
                                        .expect("Failed to restore previous version");

                                    println!("Rollback successful: Restored previous version of file at path {}", original_file_path);
                                } else {
                                    println!(
                                        "Rollback failed: Original file not found for path {}",
                                        original_file_path
                                    );
                                }
                            }
                        }
                    }
                }

                "Rollback process completed".to_owned()
            } else {
                "Rollback failed: Error reading directory".to_owned()
            }
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
