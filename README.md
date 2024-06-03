GUI for [rencfs](https://github.com/radumarias/rencfs)

<a href="https://www.buymeacoffee.com/xorio42"><img src="https://img.buymeacoffee.com/button-api/?text=Buy me a coffee&emoji=☕&slug=xorio42&button_colour=FFDD00&font_colour=000000&font_family=Cookie&outline_colour=000000&coffee_colour=ffffff" /></a>

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
