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
