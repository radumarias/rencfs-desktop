GUI for [encrypted_fs](https://github.com/radumarias/encrypted_fs)

This is in very early stages, much more is to be implemented. Currently is working only on Linux, with plans to support macOS and Windows, Android and iOS in the future.

It uses:
- [egui](https://crates.io/crates/egui) with [eframe](https://crates.io/crates/eframe) for GUI
- [tokio](https://crates.io/crates/tokio) for concurrency
- [tonic](https://crates.io/crates/tonic) for gRPC communication between GUI and daemon
- [diesel](https://crates.io/crates/diesel) with Sqlite for ORM

![](https://github.com/radumarias/encrypted_fs_desktop/blob/main/demo.gif)
