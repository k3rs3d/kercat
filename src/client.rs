use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use log::{error, info, debug};

pub fn start_client(host: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Formulate the address, initiate connection to the server
    let address = format!("{}:{}", host, port);
    info!("Connecting to {}:{}", host, port);

    let stream = TcpStream::connect(address)?;
    let stream = Arc::new(Mutex::new(stream));
    info!("Connected successfully!");

    let is_server_closed = Arc::new(AtomicBool::new(false));

    // Create a channel for sending and receiving messages
    let (tx, rx) = mpsc::channel();

    // Thread for receiving data
    // It continuously reads from the server and prints any received data
    let receive_stream = Arc::clone(&stream);
    let is_server_closed_rcv = Arc::clone(&is_server_closed);
    let receive_handle = thread::spawn(move || {
        let mut buffer = [0u8; 1024];
        loop {
            let mut locked_stream = receive_stream.lock().unwrap();
            match locked_stream.read(&mut buffer) {
                Ok(0) => {
                    // Server has closed the connection
                    info!("Connection closed by the server.");
                    is_server_closed_rcv.store(true, Ordering::SeqCst);
                    break;
                }
                Ok(n) => {
                    // Successfully received data from the server
                    let received = std::str::from_utf8(&buffer[0..n]).unwrap();
                    println!("\nServer: {}", received);
                    debug!("Received {} bytes from server: {}", n, received);
                }
                Err(e) => {
                    // Handle read errors
                    error!("An error occurred while reading from the server: {}", e);
                    break;
                }
            }
        }
    });

    // Thread to read user input
    let is_server_closed_input = Arc::clone(&is_server_closed);
    let tx_input = tx.clone();
    let input_handle = thread::spawn(move || {
        loop {
            if is_server_closed_input.load(Ordering::SeqCst) {
                // Exit if the server has closed the connection
                info!("Exiting input loop as the server has closed the connection.");
                break;
            }
            let mut input = String::new(); // Declare input inside the loop
            print!("> ");
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut input).unwrap();
            tx_input.send(input).unwrap(); // Send the input to the writing thread
        }
    });

    // Thread for sending data
    // It continuously receives user input from the channel and sends it to the server
    let send_stream = Arc::clone(&stream);
    let send_handle = thread::spawn(move || {
        let mut locked_stream = send_stream.lock().unwrap();
        loop {
            let input = rx.recv().unwrap(); // Receive input from the other thread
            if let Err(e) = locked_stream.write_all(input.as_bytes()) {
                // Handle write errors
                error!("An error occurred while writing to the server: {}", e);
                break;
            }
            debug!("Sent data to server: {}", input.trim());
        }
    });

    // Wait for all threads to complete
    receive_handle.join().unwrap();
    input_handle.join().unwrap();
    send_handle.join().unwrap();

    Ok(())
}
