# Slate Overview

Slate is a small, fast desktop text editor for Linux. It aims to sit between Xed and Kate: quick to open, comfortable to use, nicely themed, and without a plugin system or IDE features.

This document describes how the project works right now. It is present tense. Decisions and their reasoning live in `MEMORY.md`. Release history lives in `CHANGELOG.md`.

## Stack

- Rust, edition 2024 (needs Rust 1.92 or newer)
- [GPUI Kit](https://gpui-kit.com) `0.7.1`, the only direct dependency. It brings in GPUI (published as `gpui-pre` `0.3.8`, a pinned snapshot of Zed's GPUI), GPUI Base, GPUI Component, and the Lucide icon assets.
- Rendering goes through GPUI's Vulkan backend (wgpu) on Linux, under Wayland or X11.
- No tests yet.

## Core Flow

1. `main` creates the GPUI application with Kit's default icon assets and calls `gpui_kit::init`.
2. `theme::init` loads the bundled `themes/slate.json` into the `ThemeRegistry`, applies **Slate Dark** with `apply_config`, and switches the theme mode to dark.
3. `workspace::init` binds `Ctrl+Q` to the global `Quit` action.
4. A window opens with client-side decorations (`WindowDecorations::Client`) and app id `dev.pinkpixel.Slate`. `open_window` wraps the `Workspace` view in Kit's `Root`, which owns the window border, dialogs, sheets, notifications, and tooltips.
5. When the last window closes, the app quits.

## Structure

| Path | What it holds |
|---|---|
| `src/main.rs` | App startup, window options, quit-on-last-window |
| `src/theme.rs` | Loads and applies the bundled theme |
| `src/workspace.rs` | The `Workspace` view: title bar, editor, status bar, and the `Quit` action |
| `themes/slate.json` | The Slate Dark theme, embedded at compile time with `include_str!` |
| `DOCS/` | Project docs (`OVERVIEW`, `MEMORY`, `ERRORS`, `ROADMAP`) |

## Workspace view

`Workspace` owns one `Entity<EditorState>`. The editor has line numbers on, soft wrap off, and a tab size of 4 spaces. Search (`Ctrl+F`) and multi-cursor come from Kit's `Editor` defaults.

`Workspace` observes the editor entity and re-renders when it does, so the status bar's `Ln/Col` stays current. The status bar's "Plain Text", "Spaces: 4", and "UTF-8" labels are fixed for now. They describe the current editor setup, since language detection and file loading don't exist yet.

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

## Current limits

- No file open or save yet. The editor starts empty as "Untitled".
- One editor, no tabs, no sidebar.
- No syntax highlighting. No Tree-sitter grammar features are enabled yet.
- Kit's 36 extra theme JSON files aren't included in the crate. Only Slate Dark and Kit's Default Light/Dark are available.
- Only tested on CachyOS with COSMIC (Wayland).
