# Roadmap

Slate is meant to stay small: a pleasant, fast editor for quick edits, not an IDE. Anything that pushes it toward plugins, language servers, or project management probably doesn't belong here.

## Phase 0: Scaffold ✅

- Window with a custom title bar, the editor, and a status bar
- Slate Dark theme
- Repo, license, and docs

## Phase 1: Core editing ✅

- Open, Save, and Save As with native file dialogs
- Language detection from the file extension, with Tree-sitter highlighting for 23 languages
- Unsaved-changes dot in the title, plus a prompt before closing with unsaved changes
- Folding and a whitespace toggle
- Opening a file from the command line (`slate notes.md`), pulled forward from phase 2

## Phase 2: Tabs and files

- Multiple tabs, `Ctrl+Tab` cycling, closing and reordering tabs
- Recent files
- Open several files from the command line, and start a new file when the path doesn't exist yet

## Phase 3: Sidebar

- Toggleable file tree (`Ctrl+B`) for an opened folder
- Refreshes when files change on disk

## Phase 4: Theming and settings

- Theme picker, with Kit's built-in theme set bundled
- A custom themes folder that hot-reloads
- Font family and size settings
- Settings saved to `~/.config/slate/`

## Phase 5: Extras

- Word wrap toggle
- Session restore
- Markdown preview split
- Minimap
- Command palette (`Ctrl+Shift+P`)

## Phase 6: Polish and packaging

- Keyboard shortcut pass, accessibility labels, and tooltips
- Startup time check
- `.desktop` file and app icon
- Packaging (AppImage and/or an AUR package)
