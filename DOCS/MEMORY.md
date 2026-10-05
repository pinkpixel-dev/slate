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
