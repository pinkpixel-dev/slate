# Slate 0.6.0

Released October 5, 2026.

This is the phase 5 release, and it's a big one. Slate picks up the features I kept reaching for and not finding: a proper find and replace bar, a command palette, a Markdown preview, a minimap, and word wrap. It also gets a lot harder to lose work in. Your tabs and unsaved edits come back next time you start it, and Slate now notices when a file changes on disk instead of quietly overwriting it.

## Highlights

- **Find and replace** (`Ctrl+F`, `Ctrl+H`): a bar above the editor that starts at your cursor, counts matches, and has Match Case, Replace, and Replace All
- **Session restore:** start Slate with no files and your last tabs, cursor positions, sidebar folder, and unsaved edits come back. That includes untitled tabs you never saved
- **Changes on disk:** tabs without edits reload by themselves. Tabs with unsaved edits get a bar asking whether to reload or keep your version. Deleted files stay open so you can save them back
- **Command palette** (`Ctrl+Shift+P`): every command by name, with its shortcut next to it
- **Markdown preview** (`Ctrl+Shift+V`): a live, resizable preview next to the editor, one per tab
- **Minimap** (`Ctrl+Shift+M`): a colored overview of the file along the right edge that you can click and drag
- **Word wrap** (`Alt+Z`): in the status bar and Settings, and remembered between runs

## Things that work differently now

- **Quitting doesn't ask about unsaved tabs** while session restore is on, which is the default. The edits are saved for next time. Closing a single unsaved tab with `Ctrl+W` still asks. If you'd rather have the old prompt back, turn off **Settings → General → Editor → Restore last session**.
- **Opening Slate with files** (`slate notes.txt`) doesn't restore the session, and it doesn't touch the saved one either. Quitting that window asks about unsaved tabs like before.
- Slate keeps the session in `~/.local/state/slate/session.json`, including the text of unsaved tabs.

## Known issues

- If two Slate windows are open from separate launches without files, they'll overwrite each other's saved session, and whichever closes last wins. Single-instance mode is planned for phase 6 to fix this.
- The minimap hides while word wrap is on, and its viewport box drifts a bit below folded code. GPUI Kit doesn't expose wrapped or folded rows yet.
- The Markdown preview doesn't scroll along with the editor, and images with relative paths don't load.
- Match Case only applies to ASCII letters, and there's no regex or whole-word search yet.

## Updating

There are no settings to migrate. New settings (`word_wrap`, `restore_session`, `show_minimap`) get their defaults the first time Slate reads your existing `settings.json`.

```bash
cd slate
git pull
cargo run --release
```

## GitHub release

**Title:** Slate 0.6.0: find and replace, session restore, command palette, Markdown preview, and a minimap

**Body:**

Slate 0.6.0 finishes phase 5. It's mostly about the editor features you reach for every day, plus making it a lot harder to lose work.

**New**

- Find and replace bar (`Ctrl+F` / `Ctrl+H`) with a match counter, Match Case, and Replace All
- Session restore: your last tabs, cursors, sidebar folder, and unsaved edits (untitled tabs too) come back when you start Slate without files
- Changes on disk are noticed. Clean tabs reload, tabs with unsaved edits ask whether to reload or keep yours, and deleted files stay open so you can save them back
- Command palette (`Ctrl+Shift+P`) with shortcut hints
- Live Markdown preview (`Ctrl+Shift+V`) in a resizable split
- Minimap with syntax colors (`Ctrl+Shift+M`)
- Word wrap toggle (`Alt+Z`)

**Heads up:** with session restore on (the default), quitting no longer asks about unsaved tabs, because they're saved for next time. You can turn this off in Settings.

**Known issues:** two windows from separate launches can overwrite each other's saved session. The minimap hides while word wrap is on. The Markdown preview doesn't scroll with the editor.

Made with 💖 by Pink Pixel
