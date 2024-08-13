# rencfs-desktop

GUI for [rencfs](https://github.com/radumarias/rencfs).

> [!WARNING]
> **This is very early in development. Please do not use it with sensitive data just yet. Please wait for a
stable release.
> It's mostly ideal for experimental and learning projects.**

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
