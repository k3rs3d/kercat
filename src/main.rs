use clap::{Arg, Command};
use env_logger::{Builder, Target, WriteStyle};
use log::{info, error, LevelFilter};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use async_std::task;

mod session;
mod connection;
mod errors;

#[derive(Debug, Clone)]
pub struct Config {
    host: String,
    port: String,
    listen: bool, 
    input_buffer_size: usize,
    output_buffer_size: usize,
    // More in the future...
}

// Parse command-line arguments to determine the operating mode & other parameters.
// Returns either the parsed mode or an error.
fn parse_args() -> Result<Config, Box<dyn std::error::Error>> {
    let matches = Command::new("kercat")
        .arg(
            Arg::new("listen")
                .short('l')
                .long("listen")
                .help("'Server mode'; listens for data rather than sending it."),
        )
        .arg(
            Arg::new("zero-io")
            .short('z')
            .long("zero-io")
            .conflicts_with("listen")
            .help("'Zero I/O mode' used for port scanning (no data transfer). Incompatible with -l.",
        )) // TODO: -z mode
        .arg(
            Arg::new("log-file-path")
                .long("log")
                .value_name("FILE")
                .help("Records all activity into a specified log file")
                .takes_value(true),
        )
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .value_name("HOST")
                .takes_value(true)
                .help("The host address to connect to."),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .takes_value(true)
                .help("The port number to use."),
        )
        .arg(
            Arg::new("input-buffer")
                .short('I')
                .long("in-buffer")
                .value_name("SIZE")
                .takes_value(true)
                .default_value("1024")
                .help("Sets the input buffer size. Application will quit after receiving this much data."),
        )
        .arg(
            Arg::new("output-buffer")
                .short('O')
                .long("out-buffer")
                .value_name("SIZE")
                .takes_value(true)
                .default_value("1024")
                .help("Sets the output buffer size. Application will quit after sending this much data."),
        )
        .arg(
            Arg::new("extra_1")
                .index(1)
                .value_name("EXTRA_1")
                .takes_value(true)
                .hidden(true)
                .required(false),
        )
        .arg(
            Arg::new("extra_2")
                .index(2)
                .value_name("EXTRA_2")
                .takes_value(true)
                .hidden(true)
                .required(false),
        )
        .get_matches();

    // Determine the operating mode
    let listen = matches.is_present("listen");

    // DEBUG ?
    // Configure the logger based on provided log file path, if any
    let log_file_path = matches.value_of("log-file-path").map(|s| s.to_string());
    configure_logger(&log_file_path)?;

    // Parse the buffer sizes, using default values if not specified
    // Since default values are provided in clap, these unwraps are safe
    let input_buffer_size: usize = matches.value_of("input-buffer").unwrap().parse().unwrap();
    let output_buffer_size: usize = matches.value_of("output-buffer").unwrap().parse().unwrap();

    // We'll retrieve the "extra" positional arguments here
    let extra_1 = matches.value_of("extra_1").map(|s| s.to_string());
    let extra_2 = matches.value_of("extra_2").map(|s| s.to_string());

    let mut host = matches
        .value_of("host")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "localhost".to_string()); // Default host

    let port;

    // Interpret extra arguments based on the mode (e.g. hostname)
    if matches.is_present("listen") {
        port = extra_1;
    } else if extra_2.is_some() {
        host = extra_1.unwrap_or(host); // Client mode, first unmatched is host
        port = extra_2; // Client mode, second unmatched is port
    } else {
        port = extra_1;
    }

    // Provide a default port if still missing
    let port = port.unwrap_or_else(|| "8000".to_string()); // Default 8000

    // Build the Config struct
    let config = Config {
        host,
        port,
        listen,
        input_buffer_size,
        output_buffer_size,
        // + other fields?
    };

    Ok(config)
}

fn configure_logger(log_file_path: &Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut binding = Builder::from_default_env();
    let builder = binding
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}][{}] {}",
                record.target(),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .write_style(WriteStyle::Always);

    // Redirect logs to a file if a path is provided
    match log_file_path {
        Some(path) => {
            let file = File::create(path)?;
            builder.target(Target::Pipe(Box::new(file)));
        }
        None => {
            builder.filter(None, LevelFilter::Off);
        }
    }

    builder.init();
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments to build Config struct
    let config = Arc::new(parse_args()?);

    info!("Starting session with config: {:?}", &config);

    // Start the session; block_on is used to run the async function synchronously
    let result = task::block_on(session::start_session(config));
    match result {
        Ok(_) => info!("Session ended successfully"),
        Err(e) => error!("Error during session: {}", e),
    }

    Ok(())
}
