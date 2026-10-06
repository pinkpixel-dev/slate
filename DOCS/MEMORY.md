# Memory

Significant project decisions, the reasoning behind them, and the alternatives that were turned down.

Log a decision when a real fork in the road existed, the reason is not visible in the code, and reversing it later without knowing that reason would cost something. Skip routine implementation choices, feature announcements, and anything that belongs in `CHANGELOG.md` or `ROADMAP.md`.

Format:

```md
## YYYY-MM-DD

### Decision: <what was decided, as a statement>

What was decided: <specific, present tense, naming real files or fields>

Why: <the reason the code cannot show>

Rejected: <the alternative, and what was wrong with it>
```

Add new decisions under a date heading. Do not edit or delete old entries when a decision changes. Write a new entry that names the one it supersedes.

## 2026-10-05

### Decision: Build Slate on GPUI Kit and its Editor component

What was decided: Slate depends only on `gpui-kit = "0.7.1"` and uses `gpui_kit::component::input::{Editor, EditorState}` for text editing. Its Rope storage, find/replace, multi-cursor, folding, soft wrap, and Tree-sitter highlighting are reused instead of written from scratch.

Why: The text widget is the hardest part of an editor (cursors, selection, IME, undo, large files). Kit already ships one, along with tabs, a sidebar, a tree, a status bar, a title bar, and a JSON theme system, so the work goes into the app instead of the widget.

Rejected: GPUI-CE (the community fork of GPUI). It has nicer styling primitives (backdrop blur, smoothed corners, transitions) but no component library and no editor widget. It also can't be mixed with Kit, because Kit pins its own upstream GPUI snapshot (`gpui-pre 0.3.8`). Plain upstream GPUI was rejected for the same missing-editor reason.

### Decision: Request client-side window decorations

What was decided: The main window uses `WindowDecorations::Client` on top of `TitleBar::window_options()`, so Kit's `TitleBar` draws the min/max/close buttons.

Why: GPUI defaults to `WindowDecorations::Server`. Under server decorations, Kit's `TitleBar` hides its own controls, and the desktop draws a second title bar above ours.

Rejected: Server-side decorations, which give a doubled title bar and don't match the theme.

### Decision: Refuse non-UTF-8 and binary files instead of loading them lossily

What was decided: `document::read_text` returns an error for any file containing a NUL byte or invalid UTF-8, and the workspace shows it as an error notification. Nothing goes through `String::from_utf8_lossy`.

Why: A lossy load swaps bad bytes for U+FFFD. If the user then pressed Save, the original bytes would be permanently replaced, which is a silent data-loss path in an editor whose whole job is not breaking files.

Rejected: Lossy decoding (corrupts files on save), and encoding detection or conversion (not worth the complexity for a minimal editor right now; could come back as an explicit "Reopen with encoding" feature).

### Decision: Track unsaved edits with a revision counter, not a boolean

What was decided: `Document` keeps `revision` (bumped on each editor `InputEvent::Change`) and `saved_revision`. A save records the revision when it starts, and `mark_saved` stores that value, not the current one.

Why: Saves run on a background thread. With a plain `dirty = false` after the write, anything typed while the write was in flight would be treated as saved, and closing the window wouldn't warn about it.

Rejected: A boolean dirty flag (loses edits made mid-save), and comparing the full buffer against the saved text (O(file size) on every keystroke).

### Decision: Draw Slate's own tab strip instead of using Kit's TabBar

What was decided: `src/workspace/tab_strip.rs` renders tabs as plain GPUI divs, with GPUI's `on_drag`/`on_drop` for reordering and Kit's `ContextMenuExt`, `ColorSelect`, and `Tooltip` for the extras.

Why: Kit's `TabBar` 0.7.1 has no reordering API, and its variants draw their own borders, which leaves no clean slot for a colored top line. The user wanted color-coded tabs specifically to tell tabs apart when many are open.

Rejected: Kit's `TabBar` (no drag reorder, and fighting its styling for the accent line), and Kit's `Dock` tab panels (built for IDE-style docking layouts, far more than an editor's single tab row needs).

### Decision: Keep preferences and remembered data in separate files

What was decided: `Settings` goes to `$XDG_CONFIG_HOME/slate/settings.json` and `AppState` (recent files, per-file tab colors) to `$XDG_STATE_HOME/slate/state.json`, both written atomically and right away on the UI thread.

Why: XDG keeps hand-editable preferences apart from data the app churns, and the phase 4 settings panel will only edit `settings.json`. The files are a few KB, so writing them in order on the spot avoids an older background write landing after a newer one.

Rejected: One combined file (mixes user intent with history), and writes on background tasks (can finish out of order).

### Decision: A hand-picked tab color beats "color by language"

What was decided: `Workspace::tab_color` returns the buffer's own color first and only falls back to `TabColor::for_language` in `TabColorMode::Language`.

Why: The user wants automatic coloring as a preference, but a color someone chose on purpose shouldn't vanish when the mode is on.

Rejected: Language mode overriding manual colors, which would silently hide colors the user picked.

### Decision: Keep the sidebar's own folder model and rebuild Kit tree items from it

What was decided: `src/file_tree.rs` (`FileTree`) owns loaded folders, expanded folders, and row-id-to-path mapping. `Sidebar::refresh` rebuilds the whole `Vec<TreeItem>` from it after every load, expand, filter toggle, or watcher rescan. Unloaded folders get a disabled "Loading…" child.

Why: Kit 0.7.1's `TreeItem::is_folder` is just "has children", and `TreeState` has no API for adding children to one item later. So lazy loading needs placeholders, and the real state has to live outside Kit.

Rejected: Reading the whole folder recursively up front (too slow for a home folder or a repo with `node_modules`), and writing a custom tree on a virtual list (loses Kit's keyboard navigation and accessibility roles).

### Decision: Don't reopen the last sidebar folder at launch until session restore exists

What was decided: Slate starts with the sidebar hidden and no folder. Bringing back the last folder is planned as part of the optional session restore in phase 5, alongside the last tabs.

Why: Slate is for quick edits. `slate notes.txt` shouldn't come up with last week's project in the sidebar. The user agreed on 2026-10-05.

Rejected: Always reopening the last folder (noise for quick edits), and defaulting to the home folder (a big, unhelpful tree). With no folder open, `Ctrl+B` opens the active file's folder instead.

### Decision: Keep Slate's own theme catalog instead of Kit's ThemeRegistry

What was decided: `src/theme/mod.rs` keeps a `ThemeCatalog` global with the bundled themes (embedded with `include_str!`) and the custom themes from `~/.config/slate/themes`, plus its own `ThemeWatcher`. Themes are applied with `Theme::update` and `apply_config`. Kit's `ThemeRegistry` isn't used.

Why: Kit 0.7.1's `ThemeRegistry::reload` clears its theme map and rebuilds it from Kit's two default themes plus the files in the watched folder. Any theme added with `load_themes_from_str`, which is all 37 bundled ones, would disappear on the first hot reload. `load_themes_from_str` also skips names that already exist, so an edited custom theme wouldn't update. On top of that, `watch_dir`'s callback only runs after the first load.

Rejected: `ThemeRegistry::watch_dir` (drops bundled themes on reload), and copying the bundled themes into the user's folder on first run (clutters the folder and leaves stale copies after updates).


### Decision: Draw Slate's own find bar on top of Kit's search engine

What was decided: `src/workspace/find_bar.rs` renders the find and replace bar as Slate UI between the tab strip and the editor. Editors are built with `searchable(false)`, and the bar drives Kit's headless search API on `EditorState`.

Why: The user wanted the find bar to match the rest of Slate's chrome (agreed 2026-10-05). Kit still does the matching, highlights, and replacing, so the custom part is only layout and key handling.

Rejected: Kit's built-in search panel. It works fine but floats over the editor in Kit's own styling. Writing a separate matcher was also rejected, because Kit's highlights only follow its own matcher.
