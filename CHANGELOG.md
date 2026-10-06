# Changelog

## 0.5.0 - October 5, 2026

### 🎨 Themes

- 37 themes to pick from: Slate Dark plus the 36 themes from GPUI Kit (Catppuccin, Gruvbox, Tokyo Night, Solarized, Ayu, Everforest, and more), all built into the app
- A palette button in the title bar for switching themes quickly
- Drop your own theme files into `~/.config/slate/themes/` and they show up in the list. Slate reloads them as soon as you save, so you can tweak a theme and watch it change
- A custom theme with the same name as a built-in one replaces it
- A theme file with a mistake in it shows an error notification instead of failing silently

### ⚙️ Settings

- A settings panel (`Ctrl+,` or the gear button in the title bar) with Appearance and General pages
- Pick the interface font and the editor font from your installed fonts, with search. The editor font list only shows monospace fonts
- Set the interface and editor font sizes
- Tab color mode and the hidden-files toggle are in the panel too
- Every setting has a reset button that puts it back to the default

### 🗂️ Tab colors

- Tabs are colored out of the box now. The new default mode cycles through the theme's syntax colors by tab position, so tab colors change along with the theme
- The tab color setting has three modes: Theme, By language, and Off. Settings files that say `"manual"` load as Off
- A color picked from a tab's right-click menu still overrides the mode. "None" in that menu is now called "Automatic", and the "Color Tabs by Language" item moved to Settings

### 🏷️ Versioning

- Bumped to 0.5.0

## 0.4.0 - October 5, 2026

### 📁 Sidebar

- A file sidebar you can show and hide with `Ctrl+B` or the sidebar button in the title bar
- Open Folder (`Ctrl+Shift+O`), or `slate ~/some/folder` from the terminal, opens a folder in the sidebar
- With no folder open, showing the sidebar opens the active file's folder. If the file has never been saved, you get an Open Folder button instead
- Folders load when you expand them, so big folders open instantly
- The tree refreshes on its own when files are added, removed, or renamed
- Dotfiles and folders like `.git`, `node_modules`, and `target` are hidden by default, with a toggle in the sidebar header. The choice is remembered
- Click a file (or select it and press Enter) to open it in a tab. Arrow keys move through the tree
- Drag the sidebar's edge to resize it

### 🏷️ Versioning

- Bumped to 0.4.0

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
