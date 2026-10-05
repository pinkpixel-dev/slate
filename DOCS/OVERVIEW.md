# Slate Overview

Slate is a small, fast desktop text editor for Linux. It aims to sit between Xed and Kate: quick to open, comfortable to use, nicely themed, and without a plugin system or IDE features.

This document describes how the project works right now. It is present tense. Decisions and their reasoning live in `MEMORY.md`. Release history lives in `CHANGELOG.md`.

## Stack

- Rust, edition 2024 (needs Rust 1.92 or newer)
- [GPUI Kit](https://gpui-kit.com) `0.7.1`, the only direct dependency. It brings in GPUI (published as `gpui-pre` `0.3.8`, a pinned snapshot of Zed's GPUI), GPUI Base, GPUI Component, and the Lucide icon assets.
- 22 `tree-sitter-*` Kit features are enabled in `Cargo.toml` for syntax highlighting. JSON highlighting is built into Kit and needs no feature.
- `Cargo.lock` pins `cc` to `1.2.67`. See `ERRORS.md` for why.
- Rendering goes through GPUI's Vulkan backend (wgpu) on Linux, under Wayland or X11. Native file dialogs go through the XDG desktop portal.
- Tests: plain unit tests plus headless UI tests using Kit's `test-support` feature, which is enabled only under `[dev-dependencies]`.

## Core Flow

1. `main` reads an optional file path from the first command-line argument, then creates the GPUI application with `assets::AppAssets` and calls `gpui_kit::init`.
2. `theme::init` loads the bundled `themes/slate.json` into the `ThemeRegistry`, applies **Slate Dark** with `apply_config`, and switches the theme mode to dark.
3. `workspace::init` binds the file shortcuts and `Ctrl+Q` in the `Workspace` key context.
4. A window opens with client-side decorations (`WindowDecorations::Client`) and app id `dev.pinkpixel.Slate`. `open_window` wraps the `Workspace` view in Kit's `Root`, which owns the window border, dialogs, sheets, notifications, and tooltips.
5. If a path was passed, `Workspace::load` reads it.
6. When the last window closes, the app quits.

Invariant: anything that would throw away unsaved edits (New, Open, Quit, the title bar close button, or a window-manager close) goes through `Workspace::guard_unsaved` first.

## Structure

| Path | What it holds |
|---|---|
| `src/main.rs` | App startup, window options, the command-line file argument, quit-on-last-window |
| `src/assets.rs` | `AppAssets`: Kit's default icons plus the extra Lucide icons Slate uses (`FilePlus`, `Save`, `Pilcrow`) |
| `src/theme.rs` | Loads and applies the bundled theme |
| `src/language.rs` | `Language { id, label }` and `detect(path)`, which maps file names and extensions to a highlighter name |
| `src/document.rs` | `Document` (path, language, dirty tracking) plus `read_text` and `write_text` |
| `src/workspace/mod.rs` | The `Workspace` view, its actions, close handling, and render |
| `src/workspace/files.rs` | New, Open, Save, and Save As flows, plus file loading |
| `src/workspace/unsaved.rs` | The Save / Don't Save / Cancel dialog |
| `src/workspace/chrome.rs` | Title bar and status bar rendering |
| `src/workspace/tests.rs` | Headless UI tests for the unsaved-changes flow |
| `themes/slate.json` | The Slate Dark theme, embedded at compile time with `include_str!` |
| `DOCS/` | Project docs (`OVERVIEW`, `MEMORY`, `ERRORS`, `ROADMAP`) |

## Workspace

`Workspace` owns one `Entity<EditorState>` and one `Document`. The editor has line numbers and folding on, soft wrap off, and a tab size of 4 spaces. Search (`Ctrl+F`) and multi-cursor come from Kit's `Editor` defaults.

Actions, all in the `Workspace` key context:

| Action | Shortcut | Handler |
|---|---|---|
| `NewFile` | `Ctrl+N` | `guard_unsaved(NewFile)`, then `reset_to_untitled` |
| `Open` | `Ctrl+O` | `guard_unsaved(Open)`, then `prompt_open` |
| `Save` | `Ctrl+S` | `save_then(None)` |
| `SaveAs` | `Ctrl+Shift+S` | `prompt_save_as(None)` |
| `Quit` | `Ctrl+Q` | `guard_unsaved(Close)`, then `window.remove_window()` |
| `ToggleWhitespace` | none (status bar button) | flips `show_whitespace` on the editor |

The title bar buttons and the status bar toggle dispatch these same actions.

### Dirty tracking

`Document` keeps a `revision` that goes up on every `InputEvent::Change` from the editor, and a `saved_revision`. The document is dirty when they differ. `write` records the revision at the moment the save starts, so edits made while the write is in flight keep the document dirty. `EditorState::set_value` doesn't emit change events, so loading a file or clearing the editor doesn't count as an edit.

The window title is `"• name - Slate"` while dirty and `"name - Slate"` otherwise. The title bar shows an accent dot next to the file name.

### Unsaved-changes prompt

`guard_unsaved(action)` runs the action straight away when the document is clean. Otherwise it opens a Kit dialog with three buttons (`dialog-discard`, `dialog-cancel`, `dialog-save`). It won't open a second dialog if one is already showing. Save runs `save_then(Some(action))`, which continues with the action only after the write succeeds. If Save needs a path first, the Save As dialog appears, and cancelling it cancels the action too.

Closes are caught in two places: `TitleBar::on_close_window` for our own close button, and `window.on_window_should_close` for Alt+F4 and other window-manager closes. Both call `should_close`, which returns `false` and opens the prompt when dirty.

### File I/O

Reads and writes run on GPUI's background executor, and results come back through `cx.spawn_in`. Errors show as Kit error notifications.

- `read_text` refuses files containing a NUL byte ("looks like a binary file") and files that aren't valid UTF-8. Nothing is decoded lossily.
- `write_text` is a plain `std::fs::write`, so it keeps the file's inode and permissions. It isn't an atomic temp-file-and-rename write.
- Line endings are kept exactly as they were read.

After a save to a new path, the language is re-detected and the highlighter is switched if it changed.

## Languages

`language::detect` checks special file names first (`Makefile`, `.bashrc`, `PKGBUILD`, `Cargo.lock`, `Gemfile`, and so on), then the lowercase extension. Unknown files fall back to `Language::PLAIN` (`"text"`, "Plain Text"). The `id` is the name Kit's highlighter expects, applied with `EditorState::set_highlighter`.

Supported: Bash, C, C++, CSS, Diff, Go, HTML, Java, JavaScript, JSON, Lua, Makefile, Markdown, PHP, Python, Ruby, Rust, SQL, TOML, TSX, TypeScript, YAML, Zig.

## Theme

`themes/slate.json` uses Kit's theme format: a `ThemeSet` with one `ThemeConfig` named `Slate Dark`. UI colors live under `colors`, and editor and syntax colors under `highlight`. The palette was picked in OKLCH:

| Role | Hex |
|---|---|
| Background | `#171a1c` |
| Panels (title bar, status bar, sidebar) | `#131518` |
| Border | `#2b2e32` |
| Text | `#e5e8eb` |
| Muted text | `#878d93` |
| Accent | `#42a4cf` (oklch 0.68 0.11 230) |

Fonts come from Kit's theme defaults, which fall back to an installed monospace font when the default one is missing.

## Icons

Kit's default `Assets` holds 101 Lucide icons. `assets::AppAssets` adds Slate's extras through `icon_assets!` and checks them first, because the default source errors on unknown paths. Use `gpui_kit::assets::IconName` (the full catalog enum) wrapped in `Icon::new(...)` for these. `gpui_kit::component::IconName` only has the original 101.

## Tests

Run `cargo test`.

- `language.rs` and `document.rs` have unit tests for detection, UTF-8 and binary refusal, and revision-based dirty tracking.
- `workspace/tests.rs` opens a real `Workspace` in a headless test window, types into the editor, presses shortcuts, and clicks dialog buttons. It covers Ctrl+Q asking first, Don't Save before New clearing the editor, and a clean Ctrl+Q closing right away.
- Test modules must import Kit types explicitly instead of `use gpui_kit::*`. See `ERRORS.md`.

## Current limits

- One document per window. No tabs or sidebar yet.
- A command-line path that doesn't exist shows an error instead of starting a new file at that path.
- The status bar's "Spaces: 4" and "UTF-8" labels are fixed.
- Kit's 36 extra theme JSON files aren't included in the crate. Only Slate Dark and Kit's Default Light/Dark are available.
- Only tested on CachyOS with COSMIC (Wayland). The unsaved-changes prompt is covered by the headless tests, but nobody has clicked through the native Open and Save dialogs yet.
