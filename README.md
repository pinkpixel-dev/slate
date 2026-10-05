# Slate

Slate is a small, fast text editor for Linux. It's meant for the moments when you want to open a file, fix something, and move on, without starting up a whole IDE.

I made it because I could never find an editor that hit the spot I wanted. Xed is pleasant but pretty plain, and Kate has more features and plugins than I need. Slate sits in between, closer to the simple side, with better theming and a cleaner look.

It's built in Rust with [GPUI Kit](https://gpui-kit.com), which renders the whole UI on the GPU.

> Slate is very early. Right now it opens a single window with a working editor and a status bar. Opening and saving files is the next thing on the list. See [`DOCS/ROADMAP.md`](DOCS/ROADMAP.md) for what's planned.

## What works right now

- An editor with line numbers, undo and redo, find and replace (`Ctrl+F`), and multiple cursors (`Alt+Click`, or `Alt+Shift+Up/Down`)
- A custom title bar with window controls, drawn by the app instead of the desktop
- A status bar showing the cursor line and column, indentation, and encoding
- The Slate Dark theme: charcoal surfaces with a muted cyan-blue accent
- `Ctrl+Q` to quit

## Requirements

- Linux with a Wayland or X11 session
- A working Vulkan driver
- Rust 1.92 or newer

I've only tested it on CachyOS with the COSMIC desktop so far.

On Arch-based systems, you probably already have the runtime libraries. If the build complains about something missing, these are the usual ones:

```bash
sudo pacman -S --needed base-devel vulkan-icd-loader libxkbcommon libxkbcommon-x11 wayland fontconfig openssl
```

## Build and run

```bash
git clone https://github.com/pinkpixel-dev/slate.git
cd slate
cargo run --release
```

The first build compiles a lot of dependencies, so it takes a few minutes. After that, rebuilds are quick.

## License

Apache 2.0. See [`LICENSE`](LICENSE).

Made with 💖 by [Pink Pixel](https://pinkpixel.dev)
