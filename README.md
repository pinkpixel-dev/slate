# Slate

Slate is a small, fast text editor for Linux. It's meant for the moments when you want to open a file, fix something, and move on, without starting up a whole IDE.

I made it because I could never find an editor that hit the spot I wanted. Xed is pleasant but pretty plain, and Kate has more features and plugins than I need. Slate sits in between, closer to the simple side, with better theming and a cleaner look.

It's built in Rust with [GPUI Kit](https://gpui-kit.com), which renders the whole UI on the GPU.

> Slate is still early. See [`DOCS/ROADMAP.md`](DOCS/ROADMAP.md) for what's planned.

## What works right now

- Tabs: open as many files as you like, drag to reorder, middle-click to close
- Color-coded tabs: a thin colored line along the top of each tab makes it easy to find in a crowded row. By default, tabs cycle through the current theme's syntax colors, so they change with the theme. You can switch to coloring by language, or turn it off, in Settings. Right-click a tab to give it its own color (a preset or any custom color), which is remembered per file
- Open and save files with your desktop's native file dialogs, or from the terminal with `slate notes.md todo.txt`
- An Open Recent menu next to the Open button
- A file sidebar (`Ctrl+B`) for browsing a folder. It hides dotfiles and folders like `node_modules` until you ask for them, and it updates by itself when files change
- Syntax highlighting for 23 languages, picked automatically from the file name
- Line numbers, code folding, undo and redo, and multiple cursors (`Alt+Click`, or `Alt+Shift+Up/Down`)
- A find and replace bar (`Ctrl+F`, `Ctrl+H`) with a match counter and a Match Case toggle. It starts at your cursor and selects the match it lands on
- Session restore: start Slate without any files and it reopens your last tabs and sidebar folder, unsaved edits included. Quitting doesn't nag you about unsaved tabs, because they'll be right there next time. If a file changed on disk while you had edits stashed, Slate tells you. You can turn this off in Settings
- A dot by the file name when you have unsaved edits. Closing an unsaved tab asks Save / Don't Save / Cancel, and so does quitting when session restore is off or you opened Slate with files
- Word wrap (`Alt+Z`) and Show Whitespace toggles in the status bar, next to the cursor position and language
- 37 built-in themes, including Slate Dark (charcoal surfaces with a muted cyan-blue accent), Catppuccin, Gruvbox, Tokyo Night, and Solarized. Switch from the palette button in the title bar
- A command palette (`Ctrl+Shift+P`) for running any command by name, with its shortcut shown next to it
- A settings panel (`Ctrl+,`) for the theme, interface and editor fonts, font sizes, word wrap, session restore, tab color mode, and hidden files

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
| `Ctrl+Shift+P` | Command palette |
| `Ctrl+,` | Settings |
| `Ctrl+W` | Close tab |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab |
| `Ctrl+F` | Find |
| `Ctrl+H` | Find and replace |
| `F3` / `Shift+F3` | Next / previous match |
| `Alt+Z` | Word wrap |
| `Ctrl+Q` | Quit |

### Where Slate keeps things

Your preferences live in `~/.config/slate/settings.json`, and custom themes go in `~/.config/slate/themes/`. Recent files and tab colors live in `~/.local/state/slate/state.json`, and your last session (including the text of unsaved tabs) lives in `~/.local/state/slate/session.json`. They're all plain JSON, so you can edit or delete them. Slate falls back to defaults if one is missing.

### Custom themes

Slate uses GPUI Kit's theme format. The easiest way to start is to copy one of the files from [`themes/`](themes) into `~/.config/slate/themes/`, rename the theme inside it, and edit the colors. The Open Folder button in Settings takes you straight there.

Slate watches that folder, so your changes show up as soon as you save the file. If you give your theme the same name as a built-in one, yours replaces it. If a file has a JSON mistake, you'll get an error notification telling you which file.

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

The themes in `themes/kit/` come from [GPUI Kit](https://github.com/longbridge/gpui-kit) v0.7.1, which is also Apache 2.0.

Made with 💖 by [Pink Pixel](https://pinkpixel.dev)
