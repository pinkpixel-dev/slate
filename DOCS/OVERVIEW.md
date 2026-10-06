# Slate Overview

Slate is a small, fast desktop text editor for Linux. It aims to sit between Xed and Kate: quick to open, comfortable to use, nicely themed, and without a plugin system or IDE features.

This document describes how the project works right now. It is present tense. Decisions and their reasoning live in `MEMORY.md`. Release history lives in `CHANGELOG.md`.

## Stack

- Rust, edition 2024 (needs Rust 1.92 or newer)
- [GPUI Kit](https://gpui-kit.com) `0.7.1`. It brings in GPUI (published as `gpui-pre` `0.3.8`, a pinned snapshot of Zed's GPUI), GPUI Base, GPUI Component, and the Lucide icon assets.
- 22 `tree-sitter-*` Kit features are enabled in `Cargo.toml` for syntax highlighting. JSON highlighting is built into Kit and needs no feature.
- `serde` and `serde_json` for the settings and state files.
- `notify` 8.2 (the same version Kit uses for theme reloading) and `futures` for the sidebar's file watcher and the custom themes watcher.
- `fc-list` (fontconfig) at runtime, to find monospace fonts for the editor font picker. If it's missing, the picker lists every font.
- `Cargo.lock` pins `cc` to `1.2.67`. See `ERRORS.md` for why.
- Rendering goes through GPUI's Vulkan backend (wgpu) on Linux, under Wayland or X11. Native file dialogs go through the XDG desktop portal.
- Tests: plain unit tests plus headless UI tests using Kit's `test-support` feature, which is enabled only under `[dev-dependencies]`.

## Core Flow

1. `main` collects every command-line argument as an absolute file path and calls `single_instance::claim`. If another Slate is already listening on the socket, the paths go to it and this process exits. Otherwise `main` builds `Storage::from_env()`, then creates the GPUI application with `assets::AppAssets` and calls `gpui_kit::init`.
2. `theme::init(storage.themes_dir(), cx)` parses the bundled themes, creates and reads the custom themes folder, records Kit's starting fonts and radius as a baseline, and stores it all in the `ThemeCatalog` global. Nothing is applied yet.
3. `sidebar::init` binds Enter in the tree, and `workspace::init` binds the shortcuts in the `Workspace` key context.
4. A window opens with client-side decorations (`WindowDecorations::Client`) and app id `dev.pinkpixel.Slate`. `Workspace::new(storage, ...)` loads settings and state, applies the theme and fonts with `theme::apply`, starts the custom themes watcher, then opens one Untitled tab. `open_window` wraps it in Kit's `Root`, which owns the window border, dialogs, sheets, notifications, and tooltips.
5. With no command-line arguments, `restore_session` brings back the last session (if `restore_session` is on in settings). Otherwise each path goes through `open_path(path, missing_ok: true)`, and a folder opens in the sidebar.
6. If this process owns the socket, a task waits for paths from later launches, opens each with `open_path(path, missing_ok: true)`, and calls `activate_window`.
7. When the last window closes, the app quits.

Invariants:

- Anything that would throw away unsaved edits (closing a tab, Close Others, Quit, the title bar close button, or a window-manager close) goes through `close_buffer` or `close_window`, which ask first. The exception is closing the whole window while session restore is active: the edits are written to `session.json` instead of being thrown away, so it closes without asking.
- The window always has at least one tab. Closing the last one replaces it with a fresh Untitled tab.

## Structure

| Path | What it holds |
|---|---|
| `src/main.rs` | App startup, window options, command-line files, quit-on-last-window |
| `src/assets.rs` | `AppAssets`: Kit's default icons plus Slate's extra Lucide icons (`FilePlus`, `Save`, `Pilcrow`) |
| `src/theme/mod.rs` | `ThemeCatalog` (bundled and custom themes), `apply`, and `reload_custom` |
| `src/theme/fonts.rs` | `FontLists`: installed fonts, and monospace fonts via `fc-list` |
| `src/theme/watcher.rs` | `ThemeWatcher`: debounced `notify` watch on the custom themes folder |
| `src/language.rs` | `Language { id, label }`, `detect(path)`, and each language's `Comment` syntax |
| `src/document.rs` | `Document` (path, language, untitled number, dirty tracking, line ending) plus `load`, `read_text`, and `write_text` |
| `src/text_format.rs` | `LineEnding` and `Indent`: detection, labels, and CRLF conversion |
| `src/line_ops.rs` | Duplicate, move up and down, and toggle comment, as pure text edits |
| `src/file_index.rs` | Quick Open's file walk and fuzzy scoring |
| `src/single_instance.rs` | The Unix socket that hands file arguments to a running Slate |
| `src/tab_color.rs` | `TabColor` presets and custom colors, theme resolution, language colors |
| `src/file_tree.rs` | `FileTree`: the sidebar's folder model, `list_dir`, hidden-file filtering, and tree item building |
| `src/sidebar/mod.rs` | `Sidebar` view: lazy loading, refresh, `SidebarEvent` |
| `src/sidebar/view.rs` | Sidebar header, empty state, and tree rows |
| `src/sidebar/watcher.rs` | `DirWatcher`: per-folder `notify` watches with debounced rescans |
| `src/minimap.rs` | The minimap model: text to colored runs, and the scroll math |
| `src/session.rs` | `Session` and `SessionTab`, the shape of `session.json` |
| `src/storage.rs` | `Settings`, `AppState`, `Storage` paths, JSON loading, and atomic writes |
| `src/workspace/mod.rs` | `Workspace`: buffers, actions, tab bookkeeping, and render |
| `src/workspace/buffer.rs` | `Buffer` (one tab) and `BufferId` |
| `src/workspace/disk_watch.rs` | `FileWatcher`, checking open files against disk, and the "changed on disk" bar |
| `src/workspace/files.rs` | Open, Save, Save As, and `open_path` |
| `src/workspace/find_bar.rs` | The find and replace bar, and the search actions |
| `src/workspace/minimap.rs` | Minimap state, background refresh, painting, and mouse handling |
| `src/workspace/palette.rs` | The command palette and its command list |
| `src/workspace/folders.rs` | Open Folder, the sidebar toggle, sidebar events, and the resizable body layout |
| `src/workspace/unsaved.rs` | Close flows and the Save / Don't Save / Cancel dialog |
| `src/workspace/prefs.rs` | Tab color resolution, appearance and hidden-file setters, theme reloads, and writing settings and state |
| `src/workspace/session.rs` | Restoring, collecting, and writing the session, and the hot exit check |
| `src/workspace/preview.rs` | The Markdown preview split |
| `src/workspace/settings_panel.rs` | The settings sheet and its searchable font pickers |
| `src/workspace/tab_strip.rs` | The tab strip: tabs, accent lines, close buttons, drag and drop |
| `src/workspace/tab_menu.rs` | Tab right-click menu and the custom color dialog |
| `src/workspace/chrome.rs` | Title bar (Open Recent, theme menu, Settings button) and status bar |
| `src/workspace/format_menus.rs` | The status bar's indentation and line ending menus |
| `src/workspace/editing.rs` | Line editing and zoom actions |
| `src/workspace/go_to_line.rs` | The Go to Line dialog and its parser |
| `src/workspace/quick_open.rs` | The Quick Open dialog |
| `src/workspace/tests.rs` | Headless UI tests and the shared test helpers |
| `src/workspace/find_tests.rs` | Headless tests for the find bar |
| `src/workspace/wrap_tests.rs` | Headless tests for word wrap |
| `src/workspace/session_tests.rs` | Headless tests for session restore |
| `src/workspace/palette_tests.rs` | Headless tests for the command palette |
| `src/workspace/disk_tests.rs` | Headless tests for disk changes |
| `src/workspace/preview_tests.rs` | Headless tests for the Markdown preview |
| `src/workspace/minimap_tests.rs` | Headless tests for the minimap |
| `src/workspace/editing_tests.rs` | Headless tests for line editing, Go to Line, zoom, Quick Open, line endings, and indentation |
| `packaging/` | The desktop entry, the app icon, and the AUR `PKGBUILD` |
| `themes/slate.json` | The Slate Dark theme, embedded at compile time with `include_str!` |
| `themes/kit/*.json` | Kit's 21 theme files (36 themes) from the `v0.7.1` tag, embedded the same way |

## Workspace and buffers

`Workspace` holds `buffers: Vec<Buffer>` and an `active` index. Each `Buffer` has a stable `BufferId`, its own `Entity<EditorState>`, a `Document`, an optional hand-picked `TabColor`, a `disk_conflict` flag, a `saves_in_flight` count, an optional Markdown `preview`, its `MinimapState`, and its `Indent`. Async work (reads, writes, dialogs) carries a `BufferId`, never an index, because tabs can move or close while it runs.

Every editor gets line numbers, folding, the indentation guessed from its text (4 spaces when there's nothing to go on), the current whitespace setting, and soft wrap from the `word_wrap` setting. Editors are built with `searchable(false)`, which turns off Kit's own search panel so `Ctrl+F` and `Ctrl+H` reach the workspace. Each buffer subscribes to its editor: `InputEvent::Change` bumps the document revision, and any editor update redraws the workspace so the status bar stays current. Only the active buffer's `Editor` element is rendered.

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
| `ToggleWordWrap` | `Alt+Z` (also a status bar button) | Flips `word_wrap`, saves it, and applies it to every open editor |
| `ToggleSidebar` | `Ctrl+B` | Shows or hides the sidebar. Showing it with no folder open opens the active file's folder |
| `OpenFolder` | `Ctrl+Shift+O` | Native folder picker, then `show_folder` |
| `Search` (Kit's) | `Ctrl+F` | Opens the find bar |
| `Replace` (Kit's) | `Ctrl+H` | Opens the find bar with the replace row |
| `FindNext` | `F3` | Next match. Opens the find bar if it's closed |
| `FindPrevious` | `Shift+F3` | Previous match. Opens the find bar if it's closed |
| `ToggleCommandPalette` | `Ctrl+Shift+P` | Opens the command palette |
| `ToggleMinimap` | `Ctrl+Shift+M` | Flips `show_minimap` and saves it |
| `TogglePreview` | `Ctrl+Shift+V` (also a status bar button on Markdown tabs) | Shows or hides the active tab's Markdown preview |
| `DuplicateLine` | `Ctrl+Shift+D` | Copies the selected lines below themselves |
| `MoveLineUp` / `MoveLineDown` | `Alt+Up` / `Alt+Down` | Swaps the selected lines with the line above or below |
| `ToggleComment` | `Ctrl+/` | Comments or uncomments the selected lines. Does nothing for languages without comments |
| `GoToLine` | `Ctrl+G` | Opens the Go to Line dialog |
| `QuickOpen` | `Ctrl+P` | Opens Quick Open |
| `ZoomIn` / `ZoomOut` / `ResetZoom` | `Ctrl+=` (and `Ctrl++`) / `Ctrl+-` / `Ctrl+0` | Changes the editor font size for this run |
| `OpenSettings` | `Ctrl+,` | Opens the settings sheet. Escape closes it (Kit's sheet handles that, since focus moves out of the `Workspace` context) |

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
- `load` calls `read_text`, notes the line ending (`LineEnding::detect`: CRLF when at least half the breaks are `\r\n`), and turns every `\r\n` into `\n`. The editor only ever holds `\n`. Every read goes through `load`: opening, session restore, disk checks, and Reload.
- `read_text` refuses files containing a NUL byte and files that aren't valid UTF-8. Nothing is decoded lossily.
- `write_text` converts back to the document's line ending, then does a plain `std::fs::write`, so it keeps the file's inode and permissions.
- Opening a file runs `Indent::detect` on the first 2000 lines. More tab-indented lines than space-indented ones means tabs. Otherwise the width is the most common step up in space indentation, ignoring one-space steps (block comment stars) unless nothing else shows up.
- `Document` keeps the file's modified time from the last read or save (`disk_modified`), so session restore can tell whether a file changed under unsaved edits.
- After a save, the language is re-detected and the highlighter switched if it changed, and the file is added to the recent list.

## Line editing

`line_ops.rs` works on plain text and a selection, and returns one `LineEdit` (a range, its replacement, and the new selection). `editing.rs` reads the active editor's text and selection, selects the range, calls Kit's `replace` (which is undoable and emits a change event), then sets the new selection. That makes each command one undo step. The actions only run while the active editor has focus, so `Alt+Up` in the find bar doesn't move lines behind it.

All three commands work on whole lines. A selection that ends at the very start of a line leaves that line out, which matches how selecting lines with `Shift+Down` feels. Toggle Comment uncomments only when every non-blank line is already commented, inserts markers at the shallowest indent, and leaves blank lines alone. `Language::comment` gives the syntax: a line prefix for most languages, and a block pair (`<!-- -->`, `/* */`) for HTML, Markdown, and CSS.

## Go to Line and Quick Open

Both are Kit dialogs opened with `open_dialog`, like the command palette.

Go to Line holds a single-line `InputState` that lives on the workspace. Kit's dialog turns Enter into its `Confirm` action, so the jump happens in the dialog's `on_ok`, which returns false (keeping the dialog open) when the text isn't a line number. `parse_target` accepts `12`, `12:5`, and `12,5`. Lines past the end land on the last line, and the target is unfolded before the cursor moves there.

Quick Open uses a second `CommandState` with `filterable(false)`, so Kit shows exactly the rows Slate gives it. Opening it walks the root folder on the background executor with `file_index::list_files` (hidden files follow `show_hidden_files`, `.git` is always skipped, symlinked folders aren't followed, and it stops at 50,000 files). Each query change goes through `on_query` to `file_index::rank`, which keeps the best 100. A match needs every query character in order. Consecutive characters, characters at the start of a word, and characters in the file name score extra, and shorter paths win ties. The root is the sidebar folder, or the active file's folder when there isn't one.

## Zoom

`Settings::editor_zoom` holds the zoom steps and is `#[serde(skip)]`, so it's never written to `settings.json`. `theme::apply` adds it to the editor font size and clamps the result to 6 to 48. Because zoom lives in `Settings`, theme and font changes keep it.

## Single instance

`single_instance::claim` binds `$XDG_RUNTIME_DIR/slate.sock` (or `slate-$USER.sock` in the temp folder). If the bind fails because the socket exists, it tries to connect and send. A failed connect means a stale socket from a crash, so it removes the file and binds again. Paths travel as raw bytes separated by NUL. The listener runs on its own thread and hands paths to the app over a `futures` channel. Because only one process owns the window, only one writes `session.json`.

## Packaging

`packaging/dev.pinkpixel.Slate.desktop` and `packaging/dev.pinkpixel.Slate.png` share the window's app id, which is how Wayland desktops match the window to its icon. `packaging/aur/PKGBUILD` builds the `slate-editor` package from the GitHub release tarball for a version tag. `DOCS/AUR.md` covers publishing it.

## Tab strip

The tab strip is drawn by Slate (`tab_strip.rs`), not by Kit's `TabBar`. Each tab is 34px high and at most 220px wide, with a truncated name and a close button. The active tab uses `tab_active` colors; others use `tab_foreground` and highlight on hover. For screen readers, the strip has the `TabList` role, each tab has the `Tab` role with its name (plus ", unsaved" when dirty) and selected state, and the close button is a named `Button`. Icon-only Kit buttons elsewhere get an `accessibility_label`, since Kit only names a button from its visible label.

- **Accent line:** a 2px bar along the top edge, colored by `Workspace::tab_color`. Tabs without a color have no line.
- **Close button:** clean tabs show the X when active or hovered. Dirty tabs show a dot that turns into the X on hover (`group_hover` on a per-tab group).
- **Mouse:** click activates, middle-click closes, and the hover tooltip shows the full path. Dragging carries a `DraggedTab`, which is also its drag preview. Dropping onto a tab calls `move_buffer(id, that_index)`. Dropping onto the empty area after the tabs moves the tab to the end, and double-clicking that area makes a new file.
- **Overflow:** the strip scrolls horizontally, and `activate` scrolls the active tab into view.

The right-click menu (`tab_menu.rs`) has Close, Close Others, and a Tab Color submenu: Automatic (clears the hand-picked color), six presets with swatches, and Custom... (a dialog with Kit's `ColorSelect`). The color mode itself lives in Settings.

## Minimap

The minimap is a 96px strip along the editor's right edge (`render_with_minimap`), drawn with a GPUI `canvas`. Kit's editor doesn't have one. It shows when `show_minimap` is on and word wrap is off. With wrap on, Kit doesn't expose how lines map to wrapped rows, so the viewport box would drift. `Ctrl+Shift+M`, the palette, and Settings toggle it.

**Model (`src/minimap.rs`).** `build_lines` turns the text into a list of `Run`s per line: stretches of non-space characters in one color, 1px per column and 2px per line, with tabs rounded to 4-column stops and nothing past column 160. `Viewport { total, top, visible }` is the editor's position in rows, and `view` turns it into the minimap's own scroll plus the viewport box. A document taller than the pane scrolls the minimap in step with the editor, reaching the bottom together. `top_for_box` (dragging) and `top_centering` (clicking) go the other way.

**Colors.** Each tab's `MinimapState` keeps its own Kit `SyntaxHighlighter`, because the editor's isn't public. `refresh_minimap` takes the text and the highlighter, parses on the background executor, builds the runs with the theme's `highlight_theme` colors (at 80% opacity, plain text at 55% of `foreground`), and puts the highlighter back. Files over 2 MB skip syntax colors.

**When it rebuilds.** `text_changed` marks the tab stale and rebuilds 300ms after typing stops. `ensure_minimap` rebuilds right away if the active tab is stale, or was built for another language or theme. It runs on tab switches, theme changes, and turning the minimap or wrap back on. The theme is told apart by the address of `highlight_theme`, since Kit swaps in a new one on every theme change.

**Mouse.** The canvas's prepaint stores its bounds in `minimap_bounds`. Pressing on the viewport box drags it. Pressing elsewhere centers that line, then drags from the middle of the box. Scrolling the wheel over the minimap scrolls the editor. All of it goes through `set_scroll_offset` with the editor's line height.

Folded code still shows in full in the minimap, so the box sits a little lower than it should below a fold.

## Markdown preview

Each `Buffer` can hold a `preview: Option<Entity<TextViewState>>`. `Ctrl+Shift+V` (or the status bar button, or the palette) creates it from the tab's current text, or drops it. It's per tab, and it works on any tab, so a `README` with no extension can be previewed too. The status bar button only shows on Markdown tabs, or on a tab whose preview is already open.

While it's open, `render_with_preview` puts the editor and Kit's `TextView` side by side in an `h_resizable` split. The preview scrolls on its own, and Kit opens links with the system browser.

`sync_preview` sends the editor's text to the preview with `set_text`, and Kit parses it off the UI thread. It runs on every editor change, and also everywhere Slate swaps in text with `set_value`, since that doesn't emit a change event: opening a file into the empty tab, reloading a tab from disk, and session restore loading a clean file.

The editor and preview don't scroll together, and relative image paths aren't resolved against the file's folder.

## Changes on disk

`FileWatcher` (`disk_watch.rs`) keeps a non-recursive `notify` watch on the folder of every open file, not on the files themselves, because many tools save by writing a temp file and renaming it over the original. `sync_file_watches` recomputes the folder set whenever tabs change or a save finishes. Events settle for 150 ms, then `on_files_changed` re-reads every open file among the changed paths in the background.

| What's on disk | Clean tab | Tab with unsaved edits |
|---|---|---|
| Same modified time, or same text | Records the new time, nothing else | Same |
| Different text | Reloads quietly with `set_value` and keeps the cursor offset | Sets `disk_conflict`, which shows a bar above the editor: Keep Mine (records the disk time, keeps the edits) or Reload (loads the disk version and marks the tab clean) |
| Gone (deleted or moved) | Keeps the tab, marks it unsaved, and shows a warning. `Ctrl+S` writes it back | Same |
| Unreadable now (binary or not UTF-8) | Error notification, tab left alone | Same |

Slate's own saves don't count. `write` bumps the buffer's `saves_in_flight`, checks skip buffers with a save running, and a read that finishes after a save started is dropped. Saving also clears `disk_conflict`.

In tests, `Workspace::new` doesn't start `FileWatcher` (see `ERRORS.md`). The tests call `on_files_changed` directly.

## Command palette

`Ctrl+Shift+P` opens Kit's `Command` component inside a Kit dialog (520px wide, 72px from the top). `Workspace` keeps one `CommandState` and clears its query each time the palette opens.

The commands live in the static `GROUPS` list in `palette.rs`: File, Find, View, and Preferences. Each entry has a label, optional search keywords, the action it runs, and, for toggles, a function that says whether it's on. Kit filters by a case-insensitive substring match on the label and keywords.

Items don't use Kit's `CommandItem::action`. Dialogs are drawn by Kit's `Root`, next to the workspace instead of inside it, so an action dispatched from the palette would never reach the workspace's handlers. Instead, `on_confirm` maps the `IndexPath` back to `GROUPS`, closes the dialog, focuses the active editor, and dispatches the action from there. For the same reason, each row draws its own shortcut hint with `Kbd::binding_for_action(action, Some("Workspace"), window)`, and toggles that are on show a check.

To add a command, add an entry to `GROUPS`. Its action has to be handled on the `Workspace` element or on the editor.

## Session restore

A window restores and saves the session only when Slate starts without arguments and `restore_session` is on. `Workspace::session_active` tracks that. A window opened with files (`slate notes.txt`) never reads or writes `session.json`, so whatever the last no-argument run stashed is kept for later, and quitting asks about unsaved tabs as usual.

**What's saved.** `collect_session` keeps every file tab and every untitled tab with text. Untitled tabs always store their text. File tabs store it only when they have unsaved edits, along with the file's `disk_modified` from when those edits started. Each tab also keeps its hand-picked color, its untitled number, its cursor offset, and whether its preview was open. The active tab and the sidebar folder are saved too.

**When it's written.** `schedule_session_save` replaces a one-second timer on every edit, tab switch, tab open or close, save, and sidebar change, so typing doesn't write on each key. Closing the window writes it straight away. Writes use `write_atomic` on the UI thread, like the other state files.

**Hot exit.** While the session is active, `close_window` and `should_close` write the session and close without the unsaved-changes dialog. `Ctrl+W` on a single unsaved tab still asks.

**Restoring.** `restore_session` reopens the folder (if it still exists), then adds tabs in order after the starting Untitled tab and removes that tab once anything is restored.

- Tabs with saved text come back with that text, still marked unsaved. If the file's modified time no longer matches, the tab gets `disk_conflict`, so it shows the same Reload / Keep Mine bar as a live change.
- Clean file tabs are reloaded from disk in the background. A file that's gone, or no longer reads as text, gets an error notification and its tab is dropped.
- Cursors are put back with `set_selected_range`.

**Turning it off** in Settings deletes `session.json` (everything in it is open in the window at that point) and stops saving. Turning it on starts saving the current window.

Only one Slate process should run with restore active at a time. Two would overwrite each other's `session.json`, and the last one to close wins.

## Word wrap

`Settings::word_wrap` drives soft wrap for every editor. `Alt+Z` and the status bar button run `ToggleWordWrap`, which has a window and applies the change right away. The settings panel's switch calls `set_word_wrap` instead. Its callbacks only get `App`, and Kit's `set_soft_wrap` takes a `Window`, so it saves the setting and then uses `cx.defer` to apply it through the workspace's stored `AnyWindowHandle` once the current window update is done.

## Find bar

Slate draws its own find and replace bar (`find_bar.rs`) instead of using Kit's built-in panel, so it matches the tab strip and status bar. It sits between the tab strip and the editor. All the matching, highlighting, and replacing still happens in Kit's search engine on `EditorState` (`set_search_query`, `next_search_match`, `replace_current_search_match`, `replace_all_search_matches`, `close_search`).

`Workspace` holds a `FindBar` with the open flag, replace mode, the match case toggle, the two Kit `InputState` fields, and `searched`: the `BufferId` whose editor currently has the highlights.

- **Opening:** `Ctrl+F` or `Ctrl+H` shows the bar, fills the query with the editor's selection when it's a single line, and selects the query text.
- **Searching:** every change to the query searches the active editor, then moves to the first match that ends at or after the selection start. Kit's matcher has no public way to set its current match, so `jump_to_anchor` steps through matches one at a time, going whichever way round is shorter. Past 512 steps it gives up and stays on the first match.
- **Current match:** it's also selected in the editor, so closing the bar leaves the cursor on it.
- **Keys in the bar:** Enter and Shift+Enter step through matches. Enter in the replace field replaces the current match. `Ctrl+Alt+Enter` replaces all. `Alt+C` toggles match case. Tab moves between the two fields. Escape closes the bar from the bar or from the editor; the workspace only takes Escape while the bar is open.
- **Tabs:** switching tabs moves the search to the new tab and clears the old tab's highlights.
- **Count:** the bar shows "3 of 12", or "No results" in the danger color.

Match case only affects ASCII letters, because Kit builds its matcher with `ascii_case_insensitive`. There's no regex or whole-word mode.

## Sidebar

`Workspace` holds an `Entity<Sidebar>` and a `sidebar_open` flag. When the sidebar is open, `render_body` puts it in a Kit `h_resizable` panel (240px to start, 160 to 480px) to the left of the editor column. The tab strip sits above the editor only. When it's closed, the editor column takes the full width. The title bar's panel button and `Ctrl+B` toggle it.

`Sidebar` keeps a `FileTree` model and rebuilds Kit `TreeItem`s from it on every change (`refresh`), restoring the selection by row id. That's because Kit's `Tree` only treats an item as a folder when it already has children, and it has no API for loading children later.

- **Lazy loading:** only the root is read when a folder opens. Expanding a folder (`TreeEvent::Expanded`) reads it on the background executor. Until it loads, it shows a disabled "Loading…" child. A loaded empty folder shows "Empty".
- **Rows:** row ids are the path strings. `FileTree::row(id)` maps an id back to `RowKind::File`, `Folder`, or `Placeholder`.
- **Sorting and hiding:** folders come first, then files, both case-insensitive. Names starting with `.` and the folders `node_modules`, `target`, and `__pycache__` are hidden unless `show_hidden_files` is on. Filtering happens when items are built, so toggling is instant.
- **Watching:** `DirWatcher` adds a non-recursive `notify` watch for each loaded folder. Events are batched for 150ms, then every loaded folder that was touched (the changed path or its parent) gets re-read. A folder that fails to read is forgotten along with its subfolders. If `notify` can't start, the sidebar works without auto-refresh.
- **Opening files:** clicking a file row, or pressing Enter on it (`OpenSelected`, bound to `enter` in the `Tree` context), emits `SidebarEvent::OpenFile`, and the workspace calls `open_path`. Enter on a folder toggles it. The arrow keys come from Kit's tree.

## Tab colors

`TabColor` is `Red`, `Yellow`, `Green`, `Teal`, `Blue`, `Purple`, or `Custom(hex)`. Presets resolve through the active theme's `red`, `yellow`, `green`, `cyan`, `blue`, and `magenta` colors, so they follow theme changes. Custom colors are stored as hex and stay fixed.

`Workspace::tab_hsla(index, buffer, cx)` returns the buffer's hand-picked color first. Otherwise it depends on `TabColorMode`:

| Mode | Color |
|---|---|
| `Theme` (default) | `tab_color::theme_cycle(theme)[index % len]`: the theme's `keyword`, `string`, `function`, `type`, `constant`, `attribute`, `tag`, `number`, `title`, and `constructor` syntax colors, skipping ones that look the same as an earlier pick or as the text color. With fewer than three left, it uses the six presets instead |
| `Language` | `TabColor::for_language(id)`, resolved through the theme. Plain text gets no color |
| `Off` | None |

Theme mode colors by position, so dragging a tab to a new spot changes its color.

## Settings and state

`Storage::from_env` uses `$XDG_CONFIG_HOME/slate` (default `~/.config/slate`) and `$XDG_STATE_HOME/slate` (default `~/.local/state/slate`).

| File | Type | Contents |
|---|---|---|
| `settings.json` | `Settings` | `tab_color_mode`: `"theme"` (default), `"language"`, or `"off"`. The old `"manual"` value loads as `"off"`. `show_hidden_files`: `false` by default. `word_wrap`: `false` by default. `show_minimap`: `true` by default. `restore_session`: `true` by default (`Settings` has a hand-written `Default` for this). `theme`, `ui_font`, `ui_font_size`, `editor_font`, `editor_font_size`: all optional, `null` means the default |
| `state.json` | `AppState` | `recent_files` (newest first, max 10, no duplicates) and `tab_colors` (path to `TabColor`) |
| `session.json` | `Session` | Open tabs, the active tab, the sidebar folder, and whether the sidebar was open. See Session restore |

Both use `#[serde(default)]`, so missing keys get defaults and unknown keys are ignored. A missing file loads as defaults. An unparseable file prints a warning and loads as defaults, and it gets overwritten the next time that file is saved.

Writes go through `storage::write_atomic` (temporary file, then rename) and happen right away on the UI thread, in order. They're triggered by changing a tab color, the color mode, the hidden-files toggle, or anything in the settings panel, by opening or saving a file, and by Clear Recent.

## Theme

`ThemeCatalog` holds two lists: the bundled themes (Slate Dark plus Kit's 36) and the custom themes from `$XDG_CONFIG_HOME/slate/themes`. `find(name)` checks custom first, so a custom theme with a bundled theme's name replaces it. `names()` returns every name once, sorted. Kit's own `ThemeRegistry` isn't used (see `MEMORY.md`).

`theme::apply(&settings, cx)` runs inside one `Theme::update`: reset fonts, sizes, radius, and shadow to the baseline, `apply_config` the chosen theme (falling back to Slate Dark for an unknown name), then apply any font overrides from settings. `apply_config` also sets the light or dark mode. The editor uses `mono_font_family` and `mono_font_size`.

`ThemeWatcher` watches the custom folder (not recursively) and waits 150 ms for a save to settle. Then `Workspace::on_themes_changed` reloads the custom list, shows an error notification for each file that didn't parse, and reapplies the current settings, so edits to the active theme show up right away.

The settings sheet is Kit's `Settings` component inside a right-side `Sheet` (680px). Its fields read and write the workspace's `Settings` through a `WeakEntity`. The font pickers are searchable Kit `Select`s, built the first time the sheet opens because listing fonts runs `fc-list`. The editor picker only lists families fontconfig reports with spacing 90 or higher, filtered to names GPUI can load.

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

Run `cargo test`. There are 105 tests.

- Unit tests cover the file tree (sorting, hidden filtering, placeholders, forgetting folders), language detection, document reading and dirty tracking, untitled numbering, tab color serialization and near-duplicate color matching, the storage round trip, defaults, and recent-file limits, every bundled theme parsing, custom theme folder loading (including a broken file), and `fc-list` output parsing.
- `workspace/tests.rs` drives a real `Workspace` in a headless window. Each test uses `Storage::in_dir` on its own temp folder, so tests never touch your real config. They cover new tabs and cycling, the close and quit prompts across several tabs, reusing the empty tab when opening, recent files on disk, tab colors coming back for a file, reordering, opening a folder into the sidebar, `Ctrl+B` opening the active file's folder, theme and font settings applying and persisting, theme switches changing syntax and palette colors, the tab color modes, `Ctrl+,` opening the settings sheet, and the sheet's X needing two clicks while a font list is open. Sidebar tests read the tree's entries from `TreeState` rather than querying rows by id, because tree rows and their `ListItem`s share integer ids.
- `workspace/find_tests.rs` covers stepping through matches, starting from the cursor and seeding from the selection, match case, replace and replace all, and the search following tab switches.
- `workspace/wrap_tests.rs` checks wrapping by behavior, since Kit has no soft wrap getter: on a long wrapped line, Down stays on buffer line 0. It covers `Alt+Z`, new tabs picking up the setting, the saved setting, and the settings panel path.
- `session.rs` unit tests cover the `session.json` round trip and the disk-change check. `workspace/session_tests.rs` covers a full quit and relaunch (untitled text, an edited file, a clean file, the active tab, and the sidebar folder), the one-second debounce, a launch with files leaving the stored session alone, and turning restore off.
- `workspace/palette_tests.rs` covers running commands by typing and pressing Enter, opening the find bar from the palette, Escape closing it without running anything, and shortcut hints resolving while the palette has focus.
- `workspace/disk_tests.rs` covers a clean tab reloading, Keep Mine and Reload on a tab with unsaved edits, Slate's own save not counting as an outside change, a deleted file staying open as unsaved, and session restore showing the bar for a file that changed.
- `workspace/preview_tests.rs` covers the preview rendering and following typing, previews being per tab, and an open preview coming back with the session.
- `minimap.rs` unit tests cover building runs (words, tabs, syntax colors) and the scroll math for short and long documents, dragging, and clicking. `workspace/minimap_tests.rs` covers every line getting drawn with syntax colors, a click scrolling the editor, and `Ctrl+Shift+M` and word wrap hiding it. Kit's test `click` only finds Kit components, so the click test calls `on_minimap_down` directly.
- `text_format.rs`, `line_ops.rs`, `file_index.rs`, `single_instance.rs`, and `go_to_line.rs` have unit tests for line ending and indentation detection, each line edit (including selections and toggling twice), fuzzy ranking and the folder walk, forwarding paths over a real socket and replacing a stale one, and line number parsing. `workspace/editing_tests.rs` drives the shortcuts in a headless window: duplicate, move, and undo; `Ctrl+/` in Rust and plain text; line shortcuts staying out of the find bar; Go to Line, including bad input keeping the dialog open; zoom surviving a theme change without being saved; Quick Open finding and opening a file; a CRLF file saving as CRLF and then as LF; and detected tabs driving the Tab key.
- Test modules must import Kit types explicitly instead of `use gpui_kit::*`. See `ERRORS.md`.

## Current limits

- On Wayland, a second launch may not raise the existing window. Its files still open. Slate doesn't pass an XDG activation token along.
- The minimap is hidden while word wrap is on, and doesn't account for folded code.
- The sidebar's width resets each time it opens, and it can't create, rename, or delete files.
- Recent files are only offered from the title bar dropdown. There's no menu bar.
- The status bar's "UTF-8" label is fixed. Changing indentation in the status bar doesn't convert existing indentation, and it's forgotten when the tab closes.
- Typing a font size goes through Kit's number field, which applies each keystroke clamped to 8 to 32. Typing `14` briefly applies 8 first. The + and - buttons don't have that problem.
- The theme list doesn't mark which themes are custom.
- With a font list open in Settings, the first click on the panel's X only closes the list. Kit's `Select` handles the click outside its popup itself and stops it there.
- Only tested on CachyOS with COSMIC (Wayland). The sidebar hasn't been checked on screen yet.
