# Changelog

## 0.3.0 - October 5, 2026

### 🗂️ Tabs

- Open as many files as you want, each in its own tab
- `Ctrl+W` closes a tab, `Ctrl+Tab` / `Ctrl+Shift+Tab` (or `Ctrl+PageDown` / `Ctrl+PageUp`) switch between tabs
- Middle-click a tab to close it, drag tabs to reorder them, and double-click the empty space after the tabs for a new file
- Unsaved tabs show a dot that turns into a close button on hover
- Right-click a tab for Close, Close Others, and Tab Color
- New files are numbered (Untitled, Untitled 2, ...), and opening a file reuses an empty Untitled tab instead of stacking up blank ones
- Opening a file that's already open switches to its tab

### 🎨 Tab colors

- Give any tab a colored line along its top edge: six presets that follow the theme's palette, or a custom color from a color picker
- Colors are remembered per file, so a file keeps its color next time you open it
- Optional "Color Tabs by Language" mode colors tabs automatically; a color you pick yourself still wins

### 📂 Files

- Open Recent menu next to the Open button (the last 10 files), with Clear Recent
- The Open dialog can select several files at once
- `slate a.md b.rs` opens each file in a tab, and a path that doesn't exist yet opens empty and gets created when you save
- The title bar shows the active file's folder

### 💾 Unsaved changes

- Quitting with several unsaved tabs asks about each one in turn
- Closing the last tab leaves a fresh Untitled tab instead of closing the window

### ⚙️ Settings

- Preferences are saved to `~/.config/slate/settings.json`, and recent files and tab colors to `~/.local/state/slate/state.json`

### 🏷️ Versioning

- Bumped to 0.3.0

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
