# Slate

Slate is a small, fast text editor for Linux. It's meant for the moments when you want to open a file, fix something, and move on, without starting up a whole IDE.

I made it because I could never find an editor that hit the spot I wanted. Xed is pleasant but pretty plain, and Kate has more features and plugins than I need. Slate sits in between, closer to the simple side, with better theming and a cleaner look.

It's built in Rust with [GPUI Kit](https://gpui-kit.com), which renders the whole UI on the GPU.

> Slate is still early. Theme and font options are next. See [`DOCS/ROADMAP.md`](DOCS/ROADMAP.md) for what's planned.

## What works right now

- Tabs: open as many files as you like, drag to reorder, middle-click to close
- Color-coded tabs: right-click a tab and pick a color, and a thin line along its top makes it easy to find in a crowded row. Pick from presets that follow the theme, or any custom color. Colors are remembered per file, and there's an optional mode that colors tabs by language
- Open and save files with your desktop's native file dialogs, or from the terminal with `slate notes.md todo.txt`
- An Open Recent menu next to the Open button
- A file sidebar (`Ctrl+B`) for browsing a folder. It hides dotfiles and folders like `node_modules` until you ask for them, and it updates by itself when files change
- Syntax highlighting for 23 languages, picked automatically from the file name
- Line numbers, code folding, undo and redo, find and replace, and multiple cursors (`Alt+Click`, or `Alt+Shift+Up/Down`)
- A dot by the file name when you have unsaved edits, and a Save / Don't Save / Cancel prompt before anything would throw them away
- A Show Whitespace toggle in the status bar, next to the cursor position and language
- The Slate Dark theme: charcoal surfaces with a muted cyan-blue accent

Slate only opens UTF-8 text. Binary files and other encodings get an error message instead of loading as garbled text, so saving can't quietly damage them.

### Shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+N` | New file |
| `Ctrl+O` | Open |
| `Ctrl+S` | Save |
| `Ctrl+Shift+S` | Save As |
| `Ctrl+Shift+O` | Open folder |
| `Ctrl+B` | Show or hide the sidebar |
| `Ctrl+W` | Close tab |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab |
| `Ctrl+F` | Find and replace |
| `Ctrl+Q` | Quit |

### Where Slate keeps things

Your preferences live in `~/.config/slate/settings.json`. Recent files and tab colors live in `~/.local/state/slate/state.json`. Both are plain JSON, so you can edit or delete them. Slate falls back to defaults if either one is missing.

### Languages

Bash/Shell, C, C++, CSS, Diff, Go, HTML, Java, JavaScript, JSON, Lua, Makefile, Markdown, PHP, Python, Ruby, Rust, SQL, TOML, TSX, TypeScript, YAML, and Zig. Anything else opens as plain text.

## Requirements

- Linux with a Wayland or X11 session
- A working Vulkan driver
- Rust 1.92 or newer

I've only tested it on CachyOS with the COSMIC desktop so far. The file dialogs go through the XDG desktop portal, so you need a portal backend running (most desktops ship one).

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

To open files straight away, pass them after `--`. A folder opens in the sidebar, and a path that doesn't exist yet opens as an empty file and gets created when you save:

```bash
cargo run --release -- path/to/file.md
```

The first build compiles a lot of dependencies, so it takes a few minutes. After that, rebuilds are quick.

## License

Apache 2.0. See [`LICENSE`](LICENSE).

Made with 💖 by [Pink Pixel](https://pinkpixel.dev)
