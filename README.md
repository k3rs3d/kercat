# Kercat

A tribute to Netcat in Rust

## Status: Work-in-Progress (WIP)

Kercat is currently in a very early stage of development. 

While the core objective is to mirror Netcat's main capabilities, Kercat may eventually include a few additional features or unique touches.

## Purpose

The goal of this project is to replicate the functionality of Netcat using Rust. It's more of a learning exercise rather than an attempt to create a superior tool to Netcat. 

In other words, Kercat is not intended to replace professional-grade networking tools. It is designed as an educational project and should be used with that understanding in mind. I'm just building this to explore networking concepts and learn the Rust language.

## Current Features

- Command-line parameters for launching client and server ("listen") modes
- Logging to a file
- Robust error handling
- Makes use of the `async_std` crate to perform non-blocking IO operations.


## Planned Features

- UDP support
- Time delay interval/disconnect feature
- Zero I/O mode for port scanning
- Command execution / reverse shell functionality

## Usage

Usage is meant to be similar to Netcat's. 

Run in client mode by providing a host and port:

`kercat <host> <port>`

Run in "listen" mode with the `-l` flag and a port:

`kercat -l <port>`

## Prerequisites

- Rust (latest stable version)

## Copyright

Kercat is entirely my own creation, although plenty of other code was used as examples. It is freely given away to the Internet community in the hope that it will be useful, with no restrictions except giving credit where it is due. The author assumes NO responsibility for how anyone uses it. If Kercat makes you rich somehow and you're feeling generous, mail me a check. If you are affiliated in any way with Microsoft Network, get a life. Always ski in control.
