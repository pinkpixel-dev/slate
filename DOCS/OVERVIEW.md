# Slate Overview

Slate is a small, fast desktop text editor for Linux. It aims to sit between Xed and Kate: quick to open, comfortable to use, nicely themed, and without a plugin system or IDE features.

This document describes how the project works right now. It is present tense. Decisions and their reasoning live in `MEMORY.md`. Release history lives in `CHANGELOG.md`.

## Stack

- Rust, edition 2024 (needs Rust 1.92 or newer)
- [GPUI Kit](https://gpui-kit.com) `0.7.1`. It brings in GPUI (published as `gpui-pre` `0.3.8`, a pinned snapshot of Zed's GPUI), GPUI Base, GPUI Component, and the Lucide icon assets.
- 22 `tree-sitter-*` Kit features are enabled in `Cargo.toml` for syntax highlighting. JSON highlighting is built into Kit and needs no feature.
- `serde` and `serde_json` for the settings and state files.
- `Cargo.lock` pins `cc` to `1.2.67`. See `ERRORS.md` for why.
- Rendering goes through GPUI's Vulkan backend (wgpu) on Linux, under Wayland or X11. Native file dialogs go through the XDG desktop portal.
- Tests: plain unit tests plus headless UI tests using Kit's `test-support` feature, which is enabled only under `[dev-dependencies]`.

## Core Flow

1. `main` collects every command-line argument as a file path, then creates the GPUI application with `assets::AppAssets` and calls `gpui_kit::init`.
2. `theme::init` loads the bundled `themes/slate.json` into the `ThemeRegistry`, applies **Slate Dark** with `apply_config`, and switches the theme mode to dark.
3. `workspace::init` binds the shortcuts in the `Workspace` key context.
4. A window opens with client-side decorations (`WindowDecorations::Client`) and app id `dev.pinkpixel.Slate`. `Workspace::new(Storage::from_env(), ...)` loads settings and state, then starts with one Untitled tab. `open_window` wraps it in Kit's `Root`, which owns the window border, dialogs, sheets, notifications, and tooltips.
5. Each command-line path goes through `open_path(path, missing_ok: true)`.
6. When the last window closes, the app quits.

Invariants:

- Anything that would throw away unsaved edits (closing a tab, Close Others, Quit, the title bar close button, or a window-manager close) goes through `close_buffer` or `close_window`, which ask first.
- The window always has at least one tab. Closing the last one replaces it with a fresh Untitled tab.

## Structure

| Path | What it holds |
|---|---|
| `src/main.rs` | App startup, window options, command-line files, quit-on-last-window |
| `src/assets.rs` | `AppAssets`: Kit's default icons plus Slate's extra Lucide icons (`FilePlus`, `Save`, `Pilcrow`) |
| `src/theme.rs` | Loads and applies the bundled theme |
| `src/language.rs` | `Language { id, label }` and `detect(path)` |
| `src/document.rs` | `Document` (path, language, untitled number, dirty tracking) plus `read_text` and `write_text` |
| `src/tab_color.rs` | `TabColor` presets and custom colors, theme resolution, language colors |
| `src/storage.rs` | `Settings`, `AppState`, `Storage` paths, JSON loading, and atomic writes |
| `src/workspace/mod.rs` | `Workspace`: buffers, actions, tab bookkeeping, and render |
| `src/workspace/buffer.rs` | `Buffer` (one tab) and `BufferId` |
| `src/workspace/files.rs` | Open, Save, Save As, and `open_path` |
| `src/workspace/unsaved.rs` | Close flows and the Save / Don't Save / Cancel dialog |
| `src/workspace/prefs.rs` | Tab color resolution and writing settings and state |
| `src/workspace/tab_strip.rs` | The tab strip: tabs, accent lines, close buttons, drag and drop |
| `src/workspace/tab_menu.rs` | Tab right-click menu and the custom color dialog |
| `src/workspace/chrome.rs` | Title bar (with Open Recent) and status bar |
| `src/workspace/tests.rs` | Headless UI tests |
| `themes/slate.json` | The Slate Dark theme, embedded at compile time with `include_str!` |

## Workspace and buffers

`Workspace` holds `buffers: Vec<Buffer>` and an `active` index. Each `Buffer` has a stable `BufferId`, its own `Entity<EditorState>`, a `Document`, and an optional hand-picked `TabColor`. Async work (reads, writes, dialogs) carries a `BufferId`, never an index, because tabs can move or close while it runs.

Every editor gets line numbers, folding, soft wrap off, 4-space tabs, and the current whitespace setting. Each buffer subscribes to its editor: `InputEvent::Change` bumps the document revision, and any editor update redraws the workspace so the status bar stays current. Only the active buffer's `Editor` element is rendered.

New tabs go right after the active one. Untitled tabs take the lowest free number ("Untitled", "Untitled 2", ...). `Buffer::is_pristine` is true for an untitled, clean, empty tab, and opening a file reuses that tab instead of adding one.

### Actions

All in the `Workspace` key context:

| Action | Shortcut | What it does |
|---|---|---|
| `NewFile` | `Ctrl+N` | New Untitled tab |
| `Open` | `Ctrl+O` | Native multi-select open dialog |
| `Save` | `Ctrl+S` | Save the active tab (asks for a path if it has none) |
| `SaveAs` | `Ctrl+Shift+S` | Save the active tab to a new path |
| `CloseTab` | `Ctrl+W` | `close_buffer` on the active tab |
| `NextTab` | `Ctrl+Tab`, `Ctrl+PageDown` | Wraps around |
| `PreviousTab` | `Ctrl+Shift+Tab`, `Ctrl+PageUp` | Wraps around |
| `Quit` | `Ctrl+Q` | `close_window` |
| `ToggleWhitespace` | none (status bar button) | Applies to every open editor |

### Dirty tracking

`Document` keeps a `revision` that goes up on every editor change, and a `saved_revision`. The document is dirty when they differ. A save records the revision when it starts, so edits made while the write is in flight stay dirty. `EditorState::set_value` doesn't emit change events, so loading a file isn't an edit.

The window title is `"• name - Slate"` for a dirty active tab and `"name - Slate"` otherwise.

### Closing and unsaved changes

- `close_buffer(id)` removes a clean tab right away. For a dirty tab it activates the tab and opens the dialog with `PendingAction::CloseBuffer(id)`.
- `close_window` finds the first dirty tab and asks about it with `PendingAction::CloseWindow`. Once nothing is dirty, it calls `window.remove_window()`.
- Don't Save removes the tab, then continues `CloseWindow` if that was the action. Save runs `save_then(id, Some(action))`, which continues only after the write succeeds. Cancel stops everything.
- Close Others removes the clean tabs and asks about only the first dirty one, so the dialog always matches the active tab.
- Only one dialog shows at a time (`has_active_dialog`).
- Window closes are caught by `TitleBar::on_close_window` and `window.on_window_should_close`. Both go through `should_close`.

### File I/O

Reads and writes run on GPUI's background executor, and results come back through `cx.spawn_in`. Errors show as Kit error notifications.

- `open_path` makes the path absolute, switches to the tab if the file is already open, and otherwise reads it in the background. With `missing_ok`, a `NotFound` error opens an empty tab with that path. `show_file` checks for an open tab again, since two opens of the same file can race.
- `read_text` refuses files containing a NUL byte and files that aren't valid UTF-8. Nothing is decoded lossily.
- `write_text` is a plain `std::fs::write`, so it keeps the file's inode and permissions.
- After a save, the language is re-detected and the highlighter switched if it changed, and the file is added to the recent list.

## Tab strip

The tab strip is drawn by Slate (`tab_strip.rs`), not by Kit's `TabBar`. Each tab is 34px high and at most 220px wide, with a truncated name and a close button. The active tab uses `tab_active` colors; others use `tab_foreground` and highlight on hover.

- **Accent line:** a 2px bar along the top edge, colored by `Workspace::tab_color`. Tabs without a color have no line.
- **Close button:** clean tabs show the X when active or hovered. Dirty tabs show a dot that turns into the X on hover (`group_hover` on a per-tab group).
- **Mouse:** click activates, middle-click closes, and the hover tooltip shows the full path. Dragging carries a `DraggedTab`, which is also its drag preview. Dropping onto a tab calls `move_buffer(id, that_index)`. Dropping onto the empty area after the tabs moves the tab to the end, and double-clicking that area makes a new file.
- **Overflow:** the strip scrolls horizontally, and `activate` scrolls the active tab into view.

The right-click menu (`tab_menu.rs`) has Close, Close Others, and a Tab Color submenu: None, six presets with swatches, Custom... (a dialog with Kit's `ColorSelect`), and a "Color Tabs by Language" check item.

## Tab colors

`TabColor` is `Red`, `Yellow`, `Green`, `Teal`, `Blue`, `Purple`, or `Custom(hex)`. Presets resolve through the active theme's `red`, `yellow`, `green`, `cyan`, `blue`, and `magenta` colors, so they follow theme changes. Custom colors are stored as hex and stay fixed.

`Workspace::tab_color(buffer)` returns the buffer's hand-picked color first. In `TabColorMode::Language` it falls back to `TabColor::for_language(id)`, and plain text gets no color.

## Settings and state

`Storage::from_env` uses `$XDG_CONFIG_HOME/slate` (default `~/.config/slate`) and `$XDG_STATE_HOME/slate` (default `~/.local/state/slate`).

| File | Type | Contents |
|---|---|---|
| `settings.json` | `Settings` | `tab_color_mode`: `"manual"` (default) or `"language"` |
| `state.json` | `AppState` | `recent_files` (newest first, max 10, no duplicates) and `tab_colors` (path to `TabColor`) |

Both use `#[serde(default)]`, so missing keys get defaults and unknown keys are ignored. A missing file loads as defaults. An unparseable file prints a warning and loads as defaults, and it gets overwritten the next time that file is saved.

Writes go through `storage::write_atomic` (temporary file, then rename) and happen right away on the UI thread, in order. They're triggered by changing a tab color, changing the color mode, opening or saving a file, and Clear Recent.

## Theme

`themes/slate.json` uses Kit's theme format: a `ThemeSet` with one `ThemeConfig` named `Slate Dark`. UI colors live under `colors`, and editor and syntax colors under `highlight`.

| Role | Hex |
|---|---|
| Background | `#171a1c` |
| Panels (title bar, tab strip, status bar) | `#131518` |
| Border | `#2b2e32` |
| Text | `#e5e8eb` |
| Muted text | `#878d93` |
| Accent | `#42a4cf` (oklch 0.68 0.11 230) |

## Icons

Kit's default `Assets` holds 101 Lucide icons. `assets::AppAssets` adds Slate's extras through `icon_assets!` and checks them first, because the default source errors on unknown paths. Use `gpui_kit::assets::IconName` (the full catalog enum) wrapped in `Icon::new(...)` for these. `gpui_kit::component::IconName` only has the original 101.

## Tests

Run `cargo test`. There are 20 tests.

- Unit tests cover language detection, document reading and dirty tracking, untitled numbering, tab color serialization, and the storage round trip, defaults, and recent-file limits.
- `workspace/tests.rs` drives a real `Workspace` in a headless window. Each test uses `Storage::in_dir` on its own temp folder, so tests never touch your real config. They cover new tabs and cycling, the close and quit prompts across several tabs, reusing the empty tab when opening, recent files on disk, tab colors coming back for a file, and reordering.
- Test modules must import Kit types explicitly instead of `use gpui_kit::*`. See `ERRORS.md`.

## Current limits

- Session restore doesn't exist yet, so tabs aren't reopened at launch.
- Recent files are only offered from the title bar dropdown. There's no menu bar.
- The status bar's "Spaces: 4" and "UTF-8" labels are fixed.
- Kit's 36 extra theme JSON files aren't included in the crate. Only Slate Dark and Kit's Default Light/Dark are available.
- Only tested on CachyOS with COSMIC (Wayland). Drag-and-drop reordering is covered by a test of `move_buffer` but hasn't been checked by hand.
