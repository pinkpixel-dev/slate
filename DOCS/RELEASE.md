# Slate 0.7.0

Released October 6, 2026.

This is the phase 6 release. It's mostly small editing things I kept missing (Go to Line, moving lines around, Ctrl+/ comments, zoom, Quick Open), plus the stuff that makes Slate feel like a real installed app: one window instead of a pile of them, a desktop entry with an icon, and an AUR package.

## Highlights

- **Line shortcuts:** `Ctrl+Shift+D` duplicates lines, `Alt+Up` / `Alt+Down` move them, and `Ctrl+/` comments or uncomments them with the right syntax for the language
- **Go to Line** (`Ctrl+G`): type a line number, or `line:column`
- **Quick Open** (`Ctrl+P`): fuzzy-find a file in the sidebar folder, or in the current file's folder when the sidebar is empty
- **Zoom** (`Ctrl+=`, `Ctrl+-`, `Ctrl+0`): makes the editor text bigger or smaller until you close Slate
- **Indentation and line endings:** Slate guesses whether a file uses tabs or spaces (and how wide) and whether it's LF or CRLF. Both show in the status bar, where you can change them. CRLF files save back as CRLF
- **Single instance:** `slate notes.txt` opens a tab in the Slate that's already running instead of starting another one
- **Desktop entry and icon**, so Slate shows up in your launcher and in "Open With" menus
- **AUR package:** `slate-editor`

## Fixes and improvements

- Two Slate windows can't overwrite each other's saved session anymore, since there's only ever one
- Icon-only buttons have names for screen readers, and tabs are announced as tabs, with their unsaved state
- The command palette has Edit and Go groups with all the new commands

## Things that work differently now

- **Launching Slate again reuses the open window.** Files you pass on the command line open as tabs there, and the second launch exits right away. A plain `slate` with no files just brings the window forward.
- **Changing line endings marks the tab unsaved**, because the file on disk will change when you save. Changing indentation doesn't, since it only affects what Tab inserts from then on. Existing indentation isn't converted.

## Known issues

- On Wayland, the existing window may not jump to the front when you launch Slate again. Your files still open as tabs. Compositors only let a window raise itself in certain cases, and I haven't added activation token support yet.
- Startup takes around half a second on my machine. Almost all of that is GPUI setting up the GPU and the window, not Slate itself.
- Carried over from 0.6.0: the minimap hides while word wrap is on, the Markdown preview doesn't scroll with the editor, and Match Case only applies to ASCII letters.

## Installing and updating

On Arch-based distros:

```bash
paru -S slate-editor
```

From source:

```bash
cd slate
git pull
cargo run --release
```

There's nothing to migrate. Zoom isn't saved, so it never shows up in `settings.json`.

## GitHub release

**Title:** Slate 0.7.0: line shortcuts, Quick Open, single instance, and an AUR package

**Body:**

Slate 0.7.0 finishes phase 6: the last batch of editing basics, plus packaging.

**New**

- Duplicate line (`Ctrl+Shift+D`), move lines (`Alt+Up` / `Alt+Down`), and toggle comment (`Ctrl+/`)
- Go to Line (`Ctrl+G`), including `line:column`
- Quick Open (`Ctrl+P`) to fuzzy-find files in the sidebar folder
- Editor zoom (`Ctrl+=` / `Ctrl+-` / `Ctrl+0`)
- Indentation and line ending detection, shown and changeable in the status bar. CRLF files stay CRLF
- Single instance: `slate file.txt` opens a tab in the running window
- Desktop entry and app icon
- On the AUR as `slate-editor`

**Heads up:** a second `slate` launch now hands its files to the open window and exits.

**Known issues:** on Wayland the window may not come to the front when you launch Slate again (files still open). The minimap hides while word wrap is on.

Made with 💖 by Pink Pixel
