use clap::{Arg, Command};
use env_logger::{Builder, Target, WriteStyle};
use log::LevelFilter;
use std::fs::File;
use std::io::Write;

mod client;
mod server;

// Define the operating mode
enum Mode {
    Server { port: String },
    Client { host: String, port: String },
}

// Parse command-line arguments to determine the operating mode & other parameters.
// Returns either the parsed mode or an error.
fn parse_args() -> Result<Mode, Box<dyn std::error::Error>> {
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
            .help(
            "'Zero I/O mode' used for port scanning (no data transfer). Incompatible with -l.",
        )) // TODO: -z mode
        .arg(
            Arg::with_name("log")
                .long("log")
                .value_name("FILE")
                .help("Sets a custom log file")
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

    // DEBUG ?
    // Configure the logger based on provided log file path, if any
    let log_file_path = matches.value_of("log");
    configure_logger(log_file_path)?;

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

    // Determine the operating mode
    if matches.is_present("listen") {
        Ok(Mode::Server { port })
    } else {
        Ok(Mode::Client { host, port })
    }    

    //result
}

fn configure_logger(log_file_path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
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
        },
        None => {
            // If no filepath, disable logging by setting the level filter to Off
            builder.filter(None, LevelFilter::Off);
        },
    }

    builder.init();
    Ok(())
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;

    match args {
        Mode::Server { port } => {
            println!("Server mode on port {}", port);
            server::start_server(&port)?;
        }
        Mode::Client { host, port } => {
            println!("Client mode, connecting to {} on port {}", host, port);
            client::start_client(&host, &port)?;
        }
    }

    Ok(())
}
