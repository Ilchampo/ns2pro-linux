# ns2pro-linux

A learning project for building a Linux userspace driver for the Nintendo
Switch 2 Pro Controller in Rust.

## Current scope

The project currently implements the first part of HID report-descriptor
parsing:

- borrowed parsing of short and long HID items;
- type-aware decoding of Main, Global, and Local tags;
- application of persistent global and one-main-item local state;
- explicit errors for malformed byte streams and invalid state operations.

The workspace contains only `crates/hid-descriptor` for now. Other crates from
the book's suggested layout should be added when the corresponding chapters are
implemented, keeping the dependency boundaries meaningful.

Run the checks with:

```sh
cargo test
cargo clippy --all-targets --all-features
```
