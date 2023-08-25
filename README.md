# Kercat

A tribute to Netcat in Rust

## Status: Work-in-Progress (WIP)

Kercat is currently in a very early stage of development. 

## Purpose

The goal of this project is to replicate the functionality of Netcat using Rust. It's more of a learning exercise rather than an attempt to create a superior tool to Netcat. 

Kercat is not intended to replace any networking tools, it's only an educational project and should be used with that understanding in mind. I'm building this to explore networking concepts and learn the Rust language.

## Current Features

- Makes use of the `async_std` crate to perform non-blocking IO operations.
- Command-line parameters for launching client and server ("listen") modes
- Terminates when the connection is closed
- Robust error handling
- Logging to a file


## Planned Features

- UDP support
- Connecting to multiple ports
- Time delay setting
- Disconnect on timeout 
- Zero I/O mode for port scanning
- Unix domain socket support
- Command execution / reverse shell functionality
- Possibly some additional features beyond Netcat's?

## Usage

Usage is meant to be similar to Netcat's. 

Run in client mode by providing a host and port:

`kercat <host> <port>`

Run in "listen" mode with the `-l` flag and a port:

`kercat -l <port>`

## Dependencies

- Rust (latest stable version)
- async-std = "1.12.0"
- clap = "3.2"
- env_logger = "0.10.0"
- futures = "0.3.28"
- log = "0.4.20"

## Copyright

Kercat is entirely my own creation, although plenty of other code was used as examples. It is freely given away to the Internet community in the hope that it will be useful, with no restrictions except giving credit where it is due. The author assumes NO responsibility for how anyone uses it. If Kercat makes you rich somehow and you're feeling generous, mail me a check. If you are affiliated in any way with Microsoft Network, get a life. Always ski in control.
