use async_std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use clap::{Arg, Command};
use env_logger::{Builder, Target, WriteStyle};
use log::LevelFilter;
use std::{fs::File, io::Write};

use crate::{AddressType, Config};

// Parse command-line arguments to determine the operating mode & other parameters.
// Returns either the parsed Config or an error.
pub fn parse_args() -> Result<Config, Box<dyn std::error::Error>> {
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
            Arg::new("ignore_eof")
            .short('F')
            .long("ignore-eof")
            .value_name("IGNORE_EOF")
            .help("Do not close network socket for writing after seeing EOF."),
        )
        .arg(
            Arg::new("keep_listening")
            .short('k')
            .long("keep-listening")
            .value_name("KEEP_LISTENING")
            .help("Do not close network socket for listening after client disconnection (-l mode only).")
        )
        .arg(
            Arg::new("ipv4_only")
            .short('4')
            .long("ipv4-only")
            .value_name("IPV4_ONLY")
            .conflicts_with("ipv6_only")
            .help("Only use IPv4.")
        )
        .arg(
            Arg::new("ipv6_only")
            .short('6')
            .long("ipv6-only")
            .value_name("IPV6_ONLY")
            .conflicts_with("ipv4_only")
            .help("Only use IPv6.")
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

    let keep_listening = matches.is_present("keep_listening");
    let ignore_eof = matches.is_present("ignore_eof");

    // Determine the address type
    let addr_type = if matches.is_present("ipv4_only") {
        AddressType::IPv4
    } else if matches.is_present("ipv6_only") {
        AddressType::IPv6
    } else {
        // Default
        AddressType::IP
    };

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
        .unwrap_or_else(|| "127.0.0.1".to_string()); // Default host

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

    let addresses = parse_addresses(&host, &port, addr_type);

    // Build the Config struct
    let config = Config {
        addr_type,
        addresses,
        listen,
        keep_listening,
        input_buffer_size,
        output_buffer_size,
        ignore_eof,
        // + other fields?
    };

    Ok(config)
}

// TODO: Refactor address parsing, put it in a new .rs file? 
fn parse_addresses(
    host: &str,
    port_vec: &str,
    addr_type: AddressType,
) -> Vec<async_std::net::SocketAddr> {
    let mut addresses: Vec<SocketAddr> = Vec::new();

    for port in parse_port_ranges(port_vec) {
        let address_str = format!("{}:{}", host, port);
        // TODO: Better error handling
        let address: SocketAddr = match addr_type {
            AddressType::IPv4 => {
                // Parsing it to SocketAddr first, then matching on its variant
                match address_str.parse::<SocketAddr>().unwrap() {
                    SocketAddr::V4(addr) => SocketAddr::V4(addr),
                    _ => panic!("Unexpected IPv4 address"), // Since it's supposed to be IPv4 only
                }
            }
            AddressType::IPv6 => {
                match address_str.parse::<SocketAddr>().unwrap() {
                    SocketAddr::V6(addr) => SocketAddr::V6(addr),
                    _ => panic!("Unexpected IPv6 address"), // Since it's supposed to be IPv6 only
                }
            }
            AddressType::IP => {
                // Default: both IPv4 and IPv6 are allowed
                address_str.parse::<SocketAddr>().unwrap()
            }
            // + More address types in the future?
        };

        addresses.push(address);
    }

    //debug!("Parsed addresses: {:?}", addresses);
    addresses
}

fn parse_port_ranges(port_str: &str) -> Vec<u16> {
    let mut ports = Vec::new();

    // Split input by commas
    for token in port_str.split(',') {
        if token.contains('-') {
            // It's a range
            let range: Vec<&str> = token.split('-').collect();
            if range.len() != 2 {
                println!("Invalid range format: {}", token);
                continue;
            }
            let start: u16 = match range[0].parse() {
                Ok(val) => val,
                Err(_) => {
                    println!("Invalid number: {}", range[0]);
                    continue;
                }
            };
            let end: u16 = match range[1].parse() {
                Ok(val) => val,
                Err(_) => {
                    println!("Invalid number: {}", range[1]);
                    continue;
                }
            };
            if start > end {
                println!("Invalid range: {}", token);
                continue;
            }
            for port in start..=end {
                ports.push(port);
            }
        } else {
            // It's a single port
            let port: u16 = match token.parse() {
                Ok(val) => val,
                Err(_) => {
                    println!("Invalid number: {}", token);
                    continue;
                }
            };
            ports.push(port);
        }
    }

    ports
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
