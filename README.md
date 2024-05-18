GUI for [rencfs](https://github.com/radumarias/rencfs)

> ⚠️ **Warning**
> ***This is very early in development. Please do not use it with sensitive data just yet. Please wait for a
stable release.
> It's mostly ideal for experimental and learning projects.***

Currently is working only on Linux, with plans to support macOS and Windows, Android and iOS in the future.

It uses:
- [egui](https://crates.io/crates/egui) with [eframe](https://crates.io/crates/eframe) for GUI
- [tokio](https://crates.io/crates/tokio) for concurrency
- [tonic](https://crates.io/crates/tonic) for gRPC communication between GUI and daemon
- [diesel](https://crates.io/crates/diesel) with Sqlite for ORM

![](https://github.com/radumarias/rencfs_desktop/blob/main/demo.gif)
