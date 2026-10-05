# Changelog

## 0.2.0 - October 5, 2026

### 📂 Files

- New File (`Ctrl+N`), Open (`Ctrl+O`), Save (`Ctrl+S`), and Save As (`Ctrl+Shift+S`), using your desktop's native file dialogs
- New, Open, and Save buttons in the title bar, with tooltips that show the shortcut
- Open a file from the terminal with `slate path/to/file`
- Files that aren't UTF-8 text, like binaries or Latin-1 files, are refused with a message instead of being loaded garbled

### 🎨 Editor

- Syntax highlighting for 23 languages, picked from the file name or extension
- The status bar shows the current language
- Code folding
- Show Whitespace toggle in the status bar

### 💾 Unsaved changes

- A dot next to the file name, and in the window title, marks unsaved edits
- Closing the window, quitting, starting a new file, or opening another file with unsaved edits asks whether to save, discard, or cancel

### 🏷️ Versioning

- Bumped to 0.2.0

## 0.1.0 - October 5, 2026

### 🎉 First scaffold

- Opens a single Slate window with a client-side title bar and window controls
- Adds the editor surface with line numbers, find and replace, multiple cursors, and spaces for indentation
- Adds a status bar showing the cursor line and column, indentation, and encoding
- Adds the bundled Slate Dark theme
- Adds `Ctrl+Q` to quit, and quits when the last window closes
