use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::component::input::{Replace, Search};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, IndexPath, Sizable as _, WindowExt as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::color_swatches::{EditColor, ToggleColorSwatches};
use super::editing::{DuplicateLine, MoveLineDown, MoveLineUp, ResetZoom, ToggleComment, ZoomIn, ZoomOut};
use super::go_to_line::GoToLine;
use super::minimap::ToggleMinimap;
use super::preview::TogglePreview;
use super::quick_open::QuickOpen;
use super::{
    CloseTab, FindNext, FindPrevious, KEY_CONTEXT, NewFile, NextTab, Open, OpenFolder, OpenSettings, PreviousTab,
    Quit, Save, SaveAs, ToggleSidebar, ToggleWhitespace, ToggleWordWrap, Workspace,
};

actions!(slate, [ToggleCommandPalette]);

const PALETTE_WIDTH: f32 = 520.;

pub(super) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-p", ToggleCommandPalette, Some(KEY_CONTEXT))]);
}

/// One palette entry: a label and the workspace action it runs.
struct PaletteCommand {
    label: &'static str,
    keywords: &'static [&'static str],
    action: fn() -> Box<dyn Action>,
    /// For toggles: whether it's on right now.
    checked: Option<fn(&Workspace) -> bool>,
}

const fn command(label: &'static str, action: fn() -> Box<dyn Action>) -> PaletteCommand {
    PaletteCommand {
        label,
        keywords: &[],
        action,
        checked: None,
    }
}

const fn toggle(label: &'static str, action: fn() -> Box<dyn Action>, checked: fn(&Workspace) -> bool) -> PaletteCommand {
    PaletteCommand {
        label,
        keywords: &[],
        action,
        checked: Some(checked),
    }
}

impl PaletteCommand {
    const fn keywords(mut self, keywords: &'static [&'static str]) -> Self {
        self.keywords = keywords;
        self
    }
}

/// Groups in display order. `IndexPath::section` and `row` index straight into this.
static GROUPS: &[(&str, &[PaletteCommand])] = &[
    (
        "File",
        &[
            command("New File", || Box::new(NewFile)),
            command("Open...", || Box::new(Open)),
            command("Open Folder...", || Box::new(OpenFolder)).keywords(&["directory", "project"]),
            command("Save", || Box::new(Save)),
            command("Save As...", || Box::new(SaveAs)),
            command("Close Tab", || Box::new(CloseTab)),
            command("Quit", || Box::new(Quit)).keywords(&["exit"]),
        ],
    ),
    (
        "Edit",
        &[
            command("Duplicate Line", || Box::new(DuplicateLine)).keywords(&["copy"]),
            command("Move Line Up", || Box::new(MoveLineUp)),
            command("Move Line Down", || Box::new(MoveLineDown)),
            command("Toggle Comment", || Box::new(ToggleComment)).keywords(&["uncomment"]),
            command("Edit Color...", || Box::new(EditColor)).keywords(&["picker", "swatch", "hex", "rgb", "hsl"]),
        ],
    ),
    (
        "Go",
        &[
            command("Quick Open...", || Box::new(QuickOpen)).keywords(&["file", "fuzzy", "find file"]),
            command("Go to Line...", || Box::new(GoToLine)).keywords(&["jump", "line number"]),
        ],
    ),
    (
        "Find",
        &[
            command("Find", || Box::new(Search)).keywords(&["search"]),
            command("Find and Replace", || Box::new(Replace)).keywords(&["search"]),
            command("Find Next", || Box::new(FindNext)),
            command("Find Previous", || Box::new(FindPrevious)),
        ],
    ),
    (
        "View",
        &[
            toggle("Toggle Sidebar", || Box::new(ToggleSidebar), |ws| ws.sidebar_open)
                .keywords(&["files", "tree", "explorer"]),
            toggle("Toggle Word Wrap", || Box::new(ToggleWordWrap), |ws| ws.settings.word_wrap),
            toggle("Toggle Whitespace", || Box::new(ToggleWhitespace), |ws| ws.show_whitespace),
            toggle("Toggle Minimap", || Box::new(ToggleMinimap), |ws| ws.settings.show_minimap),
            toggle("Toggle Color Swatches", || Box::new(ToggleColorSwatches), |ws| ws.settings.color_swatches)
                .keywords(&["color picker"]),
            toggle("Toggle Markdown Preview", || Box::new(TogglePreview), |ws| {
                ws.active_buffer().preview.is_some()
            }),
            command("Zoom In", || Box::new(ZoomIn)).keywords(&["font", "bigger"]),
            command("Zoom Out", || Box::new(ZoomOut)).keywords(&["font", "smaller"]),
            command("Reset Zoom", || Box::new(ResetZoom)).keywords(&["font"]),
            command("Next Tab", || Box::new(NextTab)),
            command("Previous Tab", || Box::new(PreviousTab)),
        ],
    ),
    (
        "Preferences",
        &[command("Settings", || Box::new(OpenSettings)).keywords(&["preferences", "theme", "font", "options"])],
    ),
];

impl Workspace {
    pub(super) fn toggle_command_palette(&mut self, _: &ToggleCommandPalette, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let state = self.palette.clone();
        state.update(cx, |state, cx| state.set_query("", window, cx));
        let this = cx.entity().downgrade();

        window.open_dialog(cx, move |dialog, _, _| {
            let state = state.clone();
            let this = this.clone();
            dialog
                .close_button(false)
                .p_0()
                .w(px(PALETTE_WIDTH))
                .margin_top(px(72.))
                .content(move |content, _, _| content.child(render_palette(&state, this.clone())))
        });
        self.palette.update(cx, |state, cx| state.focus(window, cx));
    }

    /// Closes the palette, puts focus back on the editor, and runs the command
    /// there, where the workspace's action handlers can see it.
    fn run_palette_command(&mut self, index: IndexPath, window: &mut Window, cx: &mut Context<Self>) {
        let Some(command) = GROUPS.get(index.section).and_then(|(_, items)| items.get(index.row)) else {
            return;
        };
        window.close_dialog(cx);
        let editor = self.active_buffer().editor.clone();
        editor.update(cx, |state, cx| state.focus(window, cx));
        window.dispatch_action((command.action)(), cx);
    }
}

fn render_palette(state: &Entity<CommandState>, this: WeakEntity<Workspace>) -> Command {
    let mut palette = Command::new(state)
        .bordered(false)
        .placeholder("Type a command")
        .max_h(px(360.));

    for (group, commands) in GROUPS {
        let items = commands.iter().map(|command| {
            let this = this.clone();
            CommandItem::new()
                .label(command.label)
                .keywords(command.keywords.iter().copied())
                .child(move |window, cx| render_row(command, &this, window, cx))
        });
        palette = palette.group(CommandGroup::new().label(*group).items(items));
    }

    let confirm = this.clone();
    palette.on_confirm(move |index, window, cx| {
        _ = confirm.update(cx, |workspace, cx| workspace.run_palette_command(index, window, cx));
    })
}

/// The label, a check for toggles that are on, and the command's shortcut.
fn render_row(command: &PaletteCommand, this: &WeakEntity<Workspace>, window: &mut Window, cx: &mut App) -> AnyElement {
    let on = command
        .checked
        .zip(this.upgrade())
        .is_some_and(|(checked, workspace)| checked(workspace.read(cx)));
    // Look the binding up in the workspace's context: the palette itself lives
    // in a dialog outside it, so a plain lookup wouldn't find Slate's shortcuts.
    let binding = Kbd::binding_for_action((command.action)().as_ref(), Some(KEY_CONTEXT), window);

    h_flex()
        .w_full()
        .gap_2()
        .items_center()
        .child(div().flex_1().min_w_0().truncate().child(command.label))
        .when(on, |row| {
            row.child(Icon::new(IconName::Check).xsmall().text_color(cx.theme().muted_foreground))
        })
        .children(binding)
        .into_any_element()
}
