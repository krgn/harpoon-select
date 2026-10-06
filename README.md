# harpoon-select

A [Zellij](https://zellij.dev) plugin for quickly fuzzy-finding and
switching between panes.

This is a fork of [Nacho114/harpoon](https://github.com/Nacho114/harpoon)
(a Zellij port of [ThePrimeagen's harpoon](https://github.com/ThePrimeagen/harpoon)
for nvim), reworked around fuzzy search instead of manual bookmarking. See
[Fork rationale](#fork-rationale-vs-upstream) below for what changed and why.

## Usage

- Start typing to fuzzy search all panes across all tabs
- `Up`/`Down` or `Ctrl + n`/`Ctrl + p` to cycle through the (filtered) pane list
- `Enter` to switch to the selected pane
- `Esc` clears the current search, or closes harpoon-select if the search is already empty
- `Ctrl + c` to exit

## Why?

In a sentence: quickly jump to any pane by typing a few letters of its name.

- Every terminal pane in every tab is tracked automatically, no manual bookmarking
- Fuzzy search narrows the list as you type
- Panes are automatically removed from the list when they are closed
- When tabs or panes change name, these changes propagate to the list

## Fork rationale (vs. upstream)

Upstream harpoon is a curated bookmark list: you `a`/`A`-add specific panes,
`d`-remove them, and the list persists to disk (keyed by tab name + pane
title) so it survives a session restart. That's the right model if you only
ever care about a handful of favorite panes.

This fork instead treats harpoon-select as a pane switcher over *everything*: the
list is always every terminal pane in every tab, and you narrow it by typing
instead of by curating it up front. That flips a few design decisions:

- **`a` / `A` / `d` removed.** There's nothing to add or remove — the pane
  list is rebuilt from the live `PaneManifest`/`TabInfo` on every
  `PaneUpdate`/`TabUpdate` event (`get_all_panes` in `src/main.rs`).
- **Fuzzy search on keypress.** Any typed character now appends to a search
  query and re-filters/re-ranks the pane list (`src/fuzzy.rs`), rather than
  being a reserved shortcut.
- **`j`/`k` navigation removed.** Those letters are needed as ordinary search
  text (e.g. searching for a pane titled "jest" or "kubectl"), so navigation
  is `Up`/`Down` only, mirroring how most fuzzy finders (fzf, telescope,
  etc.) separate "type to filter" from "arrow keys to move".
- **`persistence.rs` removed entirely.** Upstream's disk-persisted bookmark
  list existed to survive pane-ID churn across a session restore. Since the
  list here is always rebuilt live from whatever panes currently exist,
  there's nothing left to persist — the previous curated-list identity
  problem (matching bookmarks back to new pane IDs by tab name + title) no
  longer applies.
- **`zellij-tile` bumped to `0.45.1`** to track a current Zellij release
  (`focus_terminal_pane` gained a third argument upstream in that version,
  handled in `src/main.rs`).
- **Nix devshell provisions the Rust toolchain.** `flake.nix` now pulls in
  `rustup` and, via `shellHook`, installs a `stable` toolchain plus the
  `wasm32-wasip1` target scoped to the project directory (`.rustup`/
  `.cargo`), so `nix develop` is enough to build without a manual `rustup`
  setup step.

If you want the original curated-bookmark behavior, use upstream
[Nacho114/harpoon](https://github.com/Nacho114/harpoon) instead — this fork
is a deliberate behavior change, not a superset.

## Installation

**Requires Zellij `0.45.0` or newer.**

_Note_: you will need to have `wasm32-wasip1` added to rust as a target to build the plugin. This can be done with `rustup target add wasm32-wasip1`. If you use Nix, running `nix develop` picks up a matching toolchain via `flake.nix` automatically.

```bash
git clone git@github.com:krgn/harpoon-select.git
cd harpoon-select
cargo build --release
mkdir -p ~/.config/zellij/plugins/
mv target/wasm32-wasip1/release/harpoon-select.wasm ~/.config/zellij/plugins/
```

## Keybinding

Add the following to your [zellij config](https://zellij.dev/documentation/configuration.html)
somewhere inside the [keybinds](https://zellij.dev/documentation/keybindings.html) section:

```kdl
shared_except "locked" {
    bind "Ctrl y" {
        LaunchOrFocusPlugin "file:~/.config/zellij/plugins/harpoon-select.wasm" {
            floating true; move_to_focused_tab true;
        }
    }
}
```

> You likely already have a `shared_except "locked"` section in your configs. Feel free to add `bind` there.

## Contributing

If you find any issues or want to suggest ideas please [open an issue](https://github.com/krgn/harpoon-select/issues/new).

### Development

Make sure you have [rust](https://rustup.rs/) installed (or run `nix develop`,
which provisions it for you) then run:

```sh
zellij action new-tab --layout ./plugin-dev-workspace.kdl
```
