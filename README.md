# rust-nv

A cross-platform rewrite of [Notational Velocity](https://notational.net/) in Rust using [egui](https://github.com/emilk/egui), with catppuccin theme support.

## Features

- **Unified search/create** — typing in the search field simultaneously filters notes and can create new ones
- **Instant search** — substring matching on titles and content
- **Auto-save** — dirty notes saved every second
- **Keyboard-driven** — full keyboard navigation (Ctrl+L, Ctrl+J/K, Ctrl+N, Ctrl+R, etc.)
- **File watcher** — external edits to note files are reflected in real-time
- **Catppuccin themes** — Latte (light), Frappe, Macchiato, Mocha (dark)
- **Flexible layout** — toggle between vertical and horizontal note list
- **Plain text** — notes are `.txt` files, one per note, filename = title
- **Cross-platform** — Windows, macOS, Linux

## Install

```bash
cargo install --path .
```

## Build from source

```bash
git clone https://github.com/acswi/rust-nv.git
cd rust-nv
cargo build --release
```

## Usage

```bash
cargo run
# or after install:
rust-nv
```

Notes are stored in `~/Documents/rust-nv-notes/` by default.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+L | Focus search field |
| Ctrl+J | Navigate note list down |
| Ctrl+K | Navigate note list up |
| Enter | Select top match or create new note |
| Escape | Clear search |
| Ctrl+N | New note |
| Ctrl+R | Rename selected note |
| Ctrl+Shift+Delete | Delete selected note |
| Ctrl+Shift+V | Paste clipboard as new note |

## License

MIT
