# Changelog

## 0.10.0 - October 7, 2026

### 📁 Tabs

- Right-click a tab for Copy Path and Reveal in File Manager. Reveal opens the file's folder with the file selected, if your file manager supports that through the desktop portal
- Both are in the command palette too (Copy File Path, Reveal in File Manager), and they work on the current tab

### 🧹 Saving

- New Trim whitespace on save setting (off by default). It strips spaces and tabs from the ends of lines and adds a final newline when the file doesn't end with one
- The cleanup happens in the editor before the write, so what you see matches the file, and one `Ctrl+Z` brings it all back
- Markdown keeps its trailing spaces, since two of them make a line break there

### 🏷️ Versioning

- Bumped to 0.10.0

## 0.9.0 - October 7, 2026

### 🎨 Color swatches

- Lines with a color value in them get a small square after the last character, filled with that color. Hex (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`), `rgb()`, `rgba()`, `hsl()`, and `hsla()` all count, in any file
- Click a square to open a color picker. Apply rewrites the value in the format it was already in (short hex stays short when it can, uppercase stays uppercase, `hsl()` stays `hsl()`), and it's one undo step
- Edit Color in the command palette opens the same picker for the color under the cursor, so you can do it from the keyboard too
- Turn them off with Color swatches in Settings, or Toggle Color Swatches in the palette

### 🏷️ Versioning

- Bumped to 0.9.0

## 0.8.0 - October 6, 2026

### 🎨 Themes

- New Slate Blue theme: a blue-gray slate editor with slightly lighter chrome and blue, cyan, white, and purple syntax colors
- New Neon theme: a dark charcoal editor with bright neon syntax colors, based on the bl1nk Kitty terminal theme
- Both show up in the theme menu and Settings, and color-coded tabs pick up their palettes too

### 🏷️ Versioning

- Bumped to 0.8.0

## 0.7.0 - October 6, 2026

### ✏️ Editing

- `Ctrl+Shift+D` duplicates the current line (or every selected line), and `Alt+Up` / `Alt+Down` move lines up and down. Each one is a single undo step
- `Ctrl+/` comments or uncomments the selected lines with the right syntax for the language. HTML, CSS, and Markdown wrap each line in a block comment instead
- `Ctrl+G` jumps to a line, or to `line:column`

### 🔎 Quick Open

- `Ctrl+P` fuzzy-finds a file in the sidebar folder, or in the current file's folder when the sidebar has none. Hidden files and folders follow the sidebar's setting, and `.git` is always skipped

### 🔠 Zoom

- `Ctrl+=` and `Ctrl+-` zoom the editor text, and `Ctrl+0` resets it. Zoom isn't saved, so Slate always starts at the font size from Settings

### 📏 Indentation and Line Endings

- Slate guesses each file's indentation (tabs or spaces, and the width) when it opens, and the Tab key follows it
- CRLF files are detected and saved back as CRLF
- Both show in the status bar. Click them to switch between spaces and tabs, pick a width, or change the line endings

### 🪟 Single Instance

- Running `slate notes.txt` while Slate is open now opens a tab in the existing window instead of starting a second copy. A plain `slate` brings the window forward. This also stops two windows from overwriting each other's saved session

### ♿ Accessibility

- Icon-only buttons now have names for screen readers, and tabs announce themselves as tabs, with their unsaved state

### 📦 Packaging

- A desktop entry and app icon, so Slate shows up in app launchers and can be picked in "Open With"
- An AUR package, `slate-editor`

### 🧭 Command Palette

- New commands for everything above, in new Edit and Go groups

### 🐛 Fixes

- When no Vulkan driver is available, Slate now prints what's wrong and how to check, instead of panicking with a backtrace

## 0.6.0 - October 5, 2026

### 🔍 Find and Replace

- A find bar (`Ctrl+F`) and a find-and-replace bar (`Ctrl+H`) that sit above the editor and match the rest of Slate
- It starts at your cursor, selects each match as you step through them, and shows "3 of 12" or "No results"
- `Enter` / `Shift+Enter` or `F3` / `Shift+F3` for next and previous, `Alt+C` for Match Case, `Ctrl+Alt+Enter` to replace everything
- If you have one line selected, it becomes the search text

### 💾 Session Restore

- Start Slate without any files and it reopens your last tabs, the active tab, cursor positions, tab colors, and the sidebar folder
- Unsaved work comes back too, including untitled tabs and files with edits you never saved
- Quitting doesn't ask about unsaved tabs anymore while this is on, since they'll be right there next time. Closing a single unsaved tab still asks
- The session is saved a second after you stop typing, so a crash loses very little
- Opening Slate with files (`slate notes.txt`) leaves your saved session alone
- On by default. You can turn it off in Settings, which brings the quit prompt back

### 📂 Changes on Disk

- Slate notices when an open file changes on disk. Tabs without edits just reload
- If you have unsaved edits, a bar asks whether to reload the disk version or keep yours
- A file that's deleted or moved away stays open, marked unsaved, so saving puts it back
- Restoring a session uses the same bar when a file changed while your edits were stashed

### 🧭 Command Palette

- `Ctrl+Shift+P` opens a searchable list of Slate's commands, each showing its shortcut
- Toggles like the sidebar, word wrap, whitespace, minimap, and Markdown preview show a check when they're on

### 📝 Markdown Preview

- `Ctrl+Shift+V` opens a live preview next to the editor in a resizable split. It updates as you type
- Each tab has its own preview, and an open preview comes back with your session
- Markdown tabs get a preview button in the status bar

### 🗺️ Minimap

- A minimap along the editor's right edge, with syntax colors from your theme
- Drag the viewport box, click to jump, or scroll over it
- Toggle it with `Ctrl+Shift+M`, the palette, or Settings. It's on by default and hides while word wrap is on

### ✏️ Editing

- Word wrap toggle (`Alt+Z`), also in the status bar and Settings. Slate remembers it

### 🏷️ Versioning

- Bumped to 0.6.0

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
