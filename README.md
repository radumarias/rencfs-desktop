# rencfs-desktop

GUI for [rencfs](https://github.com/radumarias/rencfs).

> [!WARNING]  
> **This is still under development. Please do not use it with sensitive data for now, please wait for a
stable release.  
> It's mostly ideal for experimental and learning projects. It serves as a reference app for a GUI for `rencfs`.**

Currently is working only on Linux, with plans to support macOS and Windows, Android and iOS in the future.

It uses:
- [egui](https://crates.io/crates/egui) with [eframe](https://crates.io/crates/eframe) for GUI
- [tokio](https://crates.io/crates/tokio) for concurrency
- [tonic](https://crates.io/crates/tonic) for gRPC communication between GUI and daemon
- [diesel](https://crates.io/crates/diesel) with Sqlite for ORM

![](https://github.com/radumarias/rencfs_desktop/blob/main/demo.gif)

Video  
[![Watch the video](https://img.youtube.com/vi/MkWMS3Qmk1I/0.jpg)](https://youtu.be/MkWMS3Qmk1I)

# Contribute

Feel free to fork it, change and use it in any way that you want.
If you build something interesting and feel like sharing pull requests are always appreciated.

## How to contribute

Please see [CONTRIBUTING.md](CONTRIBUTING.md).

## Build project locally

```bash
git clone https://github.com/radumarias/rencfs-desktop
```

### Dependecies

To use the encrypted file system, you need to have FUSE installed on your system. You can install it by running the
following command (or based on your distribution).

Arch

```bash
sudo pacman -Syu && sudo pacman -S fuse3
```

Ubuntu

```bash
sudo apt-get update && sudo apt-get -y install fuse3
```


#### Protocol Buffer Compiler Installation

https://grpc.io/docs/protoc-installation/

##### Linux

Using apt or apt-get, for example

```bash
apt install -y protobuf-compiler
protoc --version  # Ensure compiler version is 3+
```

##### MacOS

Using Homebrew

```bash
brew install protobuf
protoc --version  # Ensure compiler version is 3+
```

##### Install pre-compiled binaries (any OS) 

https://grpc.io/docs/protoc-installation/#install-pre-compiled-binaries-any-os

### Build

```bash
cargo build
```

## Run

Start the daemon in one terminal

```bash
cd rencfs_desktop_daemon
cargo run --package rencfs_desktop_daemon --bin rencfs_desktop_daemon
```

Start the GUI in another terminal

```bash
cd rencfs_desktop_gui
cargo run --package rencfs_desktop_gui --bin rencfs_desktop_gui
```
