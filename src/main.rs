mod fuzzy;

use core::fmt;
use std::collections::BTreeMap;

use zellij_tile::prelude::*;

/// A tracked pane, combining zellij's PaneInfo with its parent TabInfo.
///
/// Per the zellij API docs, `PaneInfo.id` combined with `PaneInfo.is_plugin`
/// uniquely identifies a pane across the entire session. Since harpoon only
/// tracks terminal panes (!is_plugin), `pane_info.id` alone is a stable,
/// globally unique identifier.
///
/// Docs: https://docs.rs/zellij-tile/latest/zellij_tile/prelude/struct.PaneInfo.html
#[derive(Clone)]
pub struct Pane {
    pub pane_info: PaneInfo,
    pub tab_info: TabInfo,
}

impl fmt::Display for Pane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} | {}", self.tab_info.name, self.pane_info.title)
    }
}

//<--------- TODO: Replace with official functions once available

/// Returns the currently active tab, if any.
///
/// `TabInfo.active` is set by zellij on the tab the user is currently viewing.
/// Docs: https://docs.rs/zellij-tile/latest/zellij_tile/prelude/struct.TabInfo.html
fn get_focused_tab(tab_infos: &Vec<TabInfo>) -> Option<TabInfo> {
    tab_infos.iter().find(|t| t.active).cloned()
}

/// Returns the focused terminal pane in the given tab.
///
/// `PaneManifest.panes` is a HashMap keyed by tab position (0-indexed), containing
/// all panes in that tab including tiled, floating, and suppressed panes.
///
/// When harpoon itself has focus (it's a plugin pane), no terminal pane will have
/// `is_focused = true`, so we fall back to the first non-plugin pane in the tab.
///
/// Docs: https://docs.rs/zellij-tile/latest/zellij_tile/prelude/struct.PaneManifest.html
fn get_focused_pane(tab_position: usize, pane_manifest: &PaneManifest) -> Option<PaneInfo> {
    let panes = pane_manifest.panes.get(&tab_position)?;
    // First, try to find a focused non-plugin pane
    if let Some(pane) = panes.iter().find(|p| p.is_focused && !p.is_plugin) {
        return Some(pane.clone());
    }
    // Fallback: if no focused non-plugin pane (e.g. harpoon itself has focus),
    // return the first non-plugin pane in the tab
    panes.iter().find(|p| !p.is_plugin).cloned()
}

//--------->

// ----------------------------------- Update ------------------------------------------------

/// Builds the full list of terminal panes across all tabs, in tab order.
///
/// Harpoon always tracks every terminal pane in the session; there is no
/// manual add/remove step. Docs:
/// https://docs.rs/zellij-tile/latest/zellij_tile/prelude/struct.PaneManifest.html
fn get_all_panes(pane_manifest: &PaneManifest, tab_infos: &Vec<TabInfo>) -> Vec<Pane> {
    let mut tab_positions: Vec<&usize> = pane_manifest.panes.keys().collect();
    tab_positions.sort();

    let mut panes = Vec::new();
    for tab_position in tab_positions {
        let Some(tab_info) = tab_infos.iter().find(|t| t.position == *tab_position) else {
            continue;
        };
        let Some(tab_panes) = pane_manifest.panes.get(tab_position) else {
            continue;
        };
        for pane_info in tab_panes {
            if !pane_info.is_plugin {
                panes.push(Pane {
                    pane_info: pane_info.clone(),
                    tab_info: tab_info.clone(),
                });
            }
        }
    }
    panes
}

#[derive(Default)]
struct State {
    selected: usize,
    panes: Vec<Pane>,
    filtered: Vec<usize>,
    query: String,
    focused_pane: Option<Pane>,
    tab_info: Option<Vec<TabInfo>>,
    pane_manifest: Option<PaneManifest>,
}

impl State {
    fn clamp_selected(&mut self) {
        if self.filtered.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }

    fn select_down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.filtered.len();
    }

    fn select_up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.filtered.len() - 1;
            return;
        }
        self.selected -= 1;
    }

    fn selected_pane(&self) -> Option<&Pane> {
        let idx = *self.filtered.get(self.selected)?;
        self.panes.get(idx)
    }

    /// Recomputes the fuzzy-filtered pane list from `self.query`.
    ///
    /// When the query is empty, all panes are shown (in tab order) and the
    /// cursor is kept on the pane the user was in before harpoon opened.
    /// Otherwise, the cursor jumps to the best match.
    fn recompute_filtered(&mut self) {
        self.filtered = fuzzy::fuzzy_filter(&self.panes, &self.query, |p| p.to_string());

        if self.query.is_empty() {
            if let Some(focused) = &self.focused_pane {
                if let Some(idx) = self
                    .filtered
                    .iter()
                    .position(|&i| self.panes[i].pane_info.id == focused.pane_info.id)
                {
                    self.selected = idx;
                }
            }
        } else {
            self.selected = 0;
        }

        self.clamp_selected();
    }

    /// Reconciles the pane list against the latest manifest and updates the
    /// currently focused pane. Called on every TabUpdate and PaneUpdate event.
    fn update_panes(&mut self) -> Option<()> {
        let pane_manifest = self.pane_manifest.clone()?;
        let tab_info = self.tab_info.clone()?;

        self.panes = get_all_panes(&pane_manifest, &tab_info);

        // Track which pane the user was in before harpoon opened
        let focused_tab = get_focused_tab(&tab_info)?;
        let focused_pane_info = get_focused_pane(focused_tab.position, &pane_manifest)?;
        self.focused_pane = Some(Pane {
            pane_info: focused_pane_info,
            tab_info: focused_tab,
        });

        self.recompute_filtered();

        Some(())
    }

    fn close(&mut self) {
        self.query.clear();
        self.recompute_filtered();
        hide_self();
    }
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, _: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[
            EventType::Key,
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::PermissionRequestResult,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::TabUpdate(tab_info) => {
                self.tab_info = Some(tab_info);
                self.update_panes();
                should_render = true;
            }
            Event::PaneUpdate(pane_manifest) => {
                self.pane_manifest = Some(pane_manifest);
                self.update_panes();
                should_render = true;
            }
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                // Rename the pane after permissions are granted, since
                // rename_plugin_pane requires ChangeApplicationState permission.
                let plugin_ids = get_plugin_ids();
                rename_plugin_pane(plugin_ids.plugin_id, "harpoon");
            }
            Event::Key(key) => {
                let has_ctrl = key.key_modifiers.contains(&KeyModifier::Ctrl);
                match key.bare_key {
                    BareKey::Char('c') if has_ctrl => {
                        self.close();
                    }
                    BareKey::Char('n') if has_ctrl => {
                        self.select_down();
                        should_render = true;
                    }
                    BareKey::Char('p') if has_ctrl => {
                        self.select_up();
                        should_render = true;
                    }
                    BareKey::Esc => {
                        if self.query.is_empty() {
                            self.close();
                        } else {
                            self.query.clear();
                            self.recompute_filtered();
                        }
                        should_render = true;
                    }
                    BareKey::Backspace => {
                        self.query.pop();
                        self.recompute_filtered();
                        should_render = true;
                    }
                    BareKey::Char(c) if !has_ctrl => {
                        self.query.push(c);
                        self.recompute_filtered();
                        should_render = true;
                    }
                    BareKey::Down => {
                        self.select_down();
                        should_render = true;
                    }
                    BareKey::Up => {
                        self.select_up();
                        should_render = true;
                    }
                    BareKey::Enter => {
                        if let Some(pane) = self.selected_pane() {
                            let pane_id = pane.pane_info.id;
                            self.close();
                            // TODO: This has a bug on macOS with hidden panes
                            focus_terminal_pane(pane_id, true, false);
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        };

        should_render
    }

    fn render(&mut self, rows: usize, cols: usize) {
        // Note: y=0 overlaps with the zellij pane frame/title bar and is not visible,
        // so we start rendering from y=1.
        let header = if self.query.is_empty() {
            format!("==== {} panes ====", self.panes.len())
        } else {
            format!("==== {}/{} panes ====", self.filtered.len(), self.panes.len())
        };
        let x = cols.saturating_sub(header.len()) / 2;
        print_text_with_coordinates(Text::new(&header), x, 0, None, None);
        let mut y = 1;

        let search_line = format!("> {}", self.query);
        print_text_with_coordinates(Text::new(&search_line), 0, y, None, None);
        y += 1;

        for (display_idx, &pane_idx) in self.filtered.iter().enumerate() {
            let pane = &self.panes[pane_idx];
            let text = if display_idx == self.selected {
                Text::new(&pane.to_string()).selected()
            } else {
                Text::new(&pane.to_string())
            };
            print_text_with_coordinates(text, 0, y, None, None);
            y += 1;
        }

        let hint_y = rows.saturating_sub(1);
        let hint_line = build_hint_line(cols);
        print_text_with_coordinates(hint_line, 0, hint_y, None, None);
    }
}

fn build_hint_line(cols: usize) -> Text {
    let (line, key_ranges) = if cols > 75 {
        build_wide_hints()
    } else if cols > 50 {
        build_medium_hints()
    } else {
        build_narrow_hints()
    };

    let mut text = Text::new(&line);
    for range in key_ranges {
        text = text.color_range(3, range);
    }
    text
}

fn build_wide_hints() -> (String, Vec<std::ops::Range<usize>>) {
    let parts = [
        ("type", " to search"),
        ("<Up/Down/Ctrl n/p>", " navigate"),
        ("<Enter>", " focus"),
        ("<Esc>", " clear/close"),
    ];
    build_hint_string(&parts, ", ")
}

fn build_medium_hints() -> (String, Vec<std::ops::Range<usize>>) {
    let parts = [
        ("type", " search"),
        ("<Up/Down>", " nav"),
        ("<Enter>", " go"),
        ("<Esc>", " clear/quit"),
    ];
    build_hint_string(&parts, ", ")
}

fn build_narrow_hints() -> (String, Vec<std::ops::Range<usize>>) {
    let parts = [("type", " search"), ("<Enter>", " go"), ("<Esc>", "")];
    build_hint_string(&parts, " ")
}

fn build_hint_string(
    parts: &[(&str, &str)],
    separator: &str,
) -> (String, Vec<std::ops::Range<usize>>) {
    let mut result = String::new();
    let mut key_ranges = Vec::new();

    for (i, (key, desc)) in parts.iter().enumerate() {
        if i > 0 {
            result.push_str(separator);
        }
        let start = result.len();
        result.push_str(key);
        let end = result.len();
        key_ranges.push(start..end);
        result.push_str(desc);
    }

    (result, key_ranges)
}
