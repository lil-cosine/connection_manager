# Connection Manager

A lightweight Linux desktop app for managing Wi-Fi and Bluetooth
connections, built with Slint and Rust.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Slint](https://img.shields.io/badge/UI-Slint-1ac09e?style=for-the-badge)
![Platform](https://img.shields.io/badge/platform-Linux-informational?style=for-the-badge&logo=linux&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)

## Overview

Connection Manager is a small, native GUI front-end for the Wi-Fi and
Bluetooth tools already on your system (`nmcli` and `bluetoothctl`).
It gives you a clean, minimal window for everyday connectivity tasks
without needing a full desktop environment's network applet.

## Features

**Wi-Fi**
- Toggle the Wi-Fi radio on/off
- View and disconnect from the currently active network
- Browse nearby available networks, sorted by signal strength
- Connect to new networks with a password prompt
- View, reconnect to, and forget saved networks

**Bluetooth**
- Toggle the Bluetooth adapter on/off
- View, connect to, disconnect from, and forget paired devices
- Scan for and connect to new nearby devices
- Automatic background scanning for new devices every 15 seconds

**Reliability**
- All system-command failures surface as an in-app error popup
  instead of silently failing or crashing the application

## Requirements

- Linux with [NetworkManager](https://networkmanager.dev/) (`nmcli`)
- [Bluetoothctl](https://man.archlinux.org/man/bluetoothctl.1)
- Rust toolchain (stable)

## Building

```bash
git clone https://github.com/lil-cosine/connection_manager.git
cd connection_manager
cargo build --release
```

## Running

```bash
cargo run --release
```

## Project Structure

```
src/
├── main.rs       # App entry point, UI callback wiring
├── wifi.rs       # Wi-Fi state via nmcli
├── bluetooth.rs  # Bluetooth state via bluetoothctl
ui/
└── app-window.slint  # Slint UI definition
```

## License

Licensed under the [MIT License](LICENSE).
