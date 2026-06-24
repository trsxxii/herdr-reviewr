# Diff viewer — visual polish backlog

Tuning and polish within the current stack (ratatui + syntect/two-face + Catppuccin). Not design changes — knobs and small additions. Fold into milestone 2 or a dedicated polish pass. ⭐ marks high niceness-per-effort.

## Structural palette (constants in `src/ui.rs`)

- ⭐ Retune the add/remove row tints (`DEL_BG`, `INS_BG`) — current values read muddy; align to Catppuccin diff conventions so changes pop without shouting. Tune live.
- ⭐ Emphasize the current line's number (brighter/bold), the way editors do.
- Swap/tune the change-bar colors, cursor-line bg (surface1), selection bg (surface0).
- Dim unchanged context slightly, or brighten changes, so the eye lands on the diff.

## Chrome / borders

- ⭐ Rounded borders (`BorderType::Rounded`) instead of square.
- Border + title colors in Catppuccin tones (focused vs unfocused); inner padding; a subtle header bar (mantle/crust) instead of the bright cyan.

## Nerd Font glyphs (terminal already uses JetBrainsMono Nerd Font)

- ⭐⭐ File-type icons (devicons) in the file list — a small extension→glyph map.
- Git branch glyph on the scope chip; a comment glyph in the gutter on commented lines; a nicer fold chevron.

## Gutter & extras

- ⭐ A subtle scrollbar / position rail on the diff (ratatui `Scrollbar`), optionally marking comment positions (mini-minimap).
- Thin separator between gutter and code; a small left margin so code doesn't hug the border.
- Dual old|new number columns even in unified view (currently single).

## Approach

Color values are see-it-live decisions — wire a batch, push to the pane, tune by eye ("warmer / dimmer / more contrast") rather than guessing hex in the abstract.
