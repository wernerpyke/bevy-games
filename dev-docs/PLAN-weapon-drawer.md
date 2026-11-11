# PLAN: weapon drawer (HUD below the game, slide-up weapon menu)

This plan comes before `PLAN-weapons.md`. It gives the player a real way to pick a weapon. It builds only the UI. It does not build any weapon.

## Goal

1. Move the HUD bar from above the game area to below it.
2. A click on an energy icon (for example the red one) slides a full-width drawer up over the game area.
3. The drawer holds one button per weapon, in a horizontal row. Each button has an icon and a text label.
4. A click on a weapon button sends one message and the drawer goes away.
5. The game does not pause while the drawer is open or while it moves.

When this plan is done, a click on a weapon button logs the choice and sends `WeaponChosen`. Nothing reads the message yet. `PLAN-weapons.md` reads it.

## Out of scope

- Any weapon effect, hit test or damage (`PLAN-weapons.md`, `rm9`).
- Energy cost, cooldown, disabled buttons when the count is 0, sound.
- How a weapon is aimed (see D5). This plan only reports which weapon was chosen.
- Touch input on phones (`rm8`). Mouse only, as in `adr-lw30`.
- R-Town. Changes to the kit must not change R-Town.

## What the code does today

| Fact | Where |
|---|---|
| The HUD bar is above the game: `ScreenMode::BarTop { ratio: 0.3125 }`. The bar is 160 art pixels high (576 by 180 on screen). The window is 576 by 756. | `games/long-way/src/main.rs`, `camera.rs`, `adr-lw24` |
| The kit already knows `ScreenMode::BarBottom { ratio }` and tests it. | `crates/game-kit/src/screen.rs` |
| The `Hud` root has its own camera (`HudCamera`, render layer 1). The camera has a viewport equal to the HUD region, so anything outside that region is clipped. | `screen.rs`, `hud.rs` |
| The `GameCamera` is the default UI camera. A UI root with no `UiTargetCamera` is drawn in the game region only. | `camera.rs`, `screen-layout.md` rule 4 |
| The energy icons are plain `Node`s with a still pickup frame and a count. Nothing reacts to a click. | `games/long-way/src/hud.rs` (`spawn_hud_content`) |
| `hud.aseprite` is the bar picture, 512 by 160 art pixels. It was drawn for a bar at the top. | `adr-lw25` |
| A mouse click in the game region sends `WorldClick`, and the player walks there. A click on the HUD region sends nothing. | `crates/game-kit/src/pointer.rs`, `adr-lw30` |
| Bevy picking (`Pointer<Click>`, UI hit testing) is on: the workspace uses the `2d` feature, which includes `picking` and `ui_picking`. No game code uses it yet. | root `Cargo.toml`, `bevy-src/Cargo.toml` |
| The script runner has `click:<x>:<y>` for world clicks only. It cannot click a UI node. | `crates/game-kit/src/script.rs` |
| There is no pause key and no `Paused` state in Long Way. | `screen-layout.md` |

Two problems follow from this:

- **Click-through.** The drawer is drawn over the game region. A click on a drawer button also goes to `send_clicks` in the kit, because the cursor is inside the game region. The player would walk to the point under the button. The kit must ignore a click that lands on the drawer.
- **Clipping.** A drawer that is a child of `Hud` cannot be drawn over the game area, because the HUD camera clips it to the HUD region. The drawer must be its own UI root on the game camera.

## Decisions needed

**Decided (user):** D1 to D4 as recommended. D5: agreed, aiming is not decided here. D6: the user changes the art of `hud.aseprite`. D7: the drawer gets its own background `.aseprite` file, and the weapon icons are an empty `.aseprite` file that the user fills in (see "Art").

| # | Question | Recommendation |
|---|---|---|
| D1 | Which weapons are in the drawer of each energy colour? The design does not say. The spike has six effects: fireball, fire ring, ice ring, fire wall, ice wall, whirlpool. | Red: fireball, fire ring, fire wall. Blue: ice ring, ice wall, whirlpool (water). Violet and black: an empty drawer that says "No weapons yet". The mapping is one table in `weapon_menu.rs`, so you can change it with no other edit. |
| D2 | What happens when you click the same energy icon again, or a different one, while the drawer is open? | Same icon: the drawer closes. A different icon: the drawer stays and its buttons change at once (no slide). |
| D3 | How does the drawer leave after a weapon click? "Disappears" can mean instant or a slide. | Slide down in 0.15 s. Opening takes 0.2 s. Both use real time (`Time<Real>`), so they would still run if a pause is added later. If you want it instant, change one constant. |
| D4 | What does a click in the game area, outside the drawer, do while the drawer is open? | It closes the drawer and still sends `WorldClick`, so the player walks. This keeps the game flowing, as you asked. The alternative is that the click only closes the drawer. |
| D5 | How is a weapon aimed when it comes from a button? The cursor is on the button, so there is no aim point. Options: (a) the last cursor position seen in the game area, (b) the direction the player faces, (c) the next click in the game area sets the aim. | **Decided:** not here. The message carries only the weapon. `PLAN-weapons.md` and its activation plan decide. |
| D6 | The bar art. `hud.aseprite` was drawn for the top of the screen. | **Decided:** the user redraws it for the bottom position. Step 1 moves the bar with the current art and does not edit the file. Keep the file size 512 by 160 (`adr-lw25`). |
| D7 | Drawer look. | **Decided:** a new file `art/ui/weapon_drawer.aseprite`, 512 by 96 art pixels, used like `hud.aseprite` (`adr-lw25`). It starts as a flat palette panel with a top border, and the user redraws it. Buttons are drawn in code (a darker panel, a lighter one on hover, a pressed one) until the user wants art for them. |

## Design

### Layout

- `ScreenMode::BarBottom { ratio: HUD_BAR_RATIO }`. The window stays 576 by 756. The game region is on top (576 by 576), the bar is below (576 by 180).
- The content of the bar keeps its own coordinates inside the bar. The user redraws `hud.aseprite` (D6). If the new art moves the energy or HP rows, the constants at the top of `hud.rs` change.

### Drawer

- Height: 60% of the bar, so **96 art pixels** (`DRAWER_RATIO = 0.6` of `HUD_BAR` height 160). Width: 100% of the game region.
- The drawer is a UI root on the **game camera** (no `UiTargetCamera`), `position_type: Absolute`, `left: 0`, `width: 100%`, anchored to the bottom of the game region. Its `bottom` value goes from `-96` (hidden, outside the camera viewport, so clipped) to `0` (open). It then touches the top edge of the bar.
- It lives for the whole game, hidden. A marker `Drawer` and a resource `DrawerState { open: Option<Energy>, slide: f32 }` hold the state. `slide` goes from 0 to 1.
- Draw order: the drawer is above the world. The map transition fade should stay above the drawer. Step 3 checks this by looking at a screenshot during a fade.
- Children: a row (`flex_direction: Row`, `justify_content: Center`, `column_gap`), one button per weapon of the open energy. The row is built again when the open energy changes (like `update_hp_row`).
- Each button is a column: an icon (32 by 32 art pixels) above a label (Courier Prime, 16 art pixels, palette text colour `HUD_TEXT`). Buttons have equal width, at most 4 fit in 512 pixels. A label has at most 10 characters.

### Input

- The energy icon entries in `hud.rs` get `Button` and a picking observer. A click sends the message `EnergyIconClicked(Energy)`. The drawer system reads it and applies the rule of D2.
- The weapon buttons use the same way. A click sends `WeaponChosen(WeaponKind)` and closes the drawer.
- Hover and pressed colours come from the `Interaction` component of the button.
- **Click-through fix in the kit.** `send_clicks` in `pointer.rs` skips a click when the cursor is over a UI node that carries a new marker component `BlocksWorldClick` (kit, `pointer.rs`). It reads the picking `HoverMap`. The drawer root and its buttons carry the marker. The game area outside the drawer does not, so D4 works. This is a kit change, so it gets an ADR and a test.

### Messages and types (Long Way, new file `weapon_menu.rs`)

- `WeaponKind`: `Fireball`, `FireRing`, `IceRing`, `FireWall`, `IceWall`, `Whirlpool`. It has `label()` (the button text) and `icon_tag()` (the tag in the icon file).
- `weapons_for(Energy) -> &'static [WeaponKind]`: the table of D1.
- `WeaponChosen(WeaponKind)`: a message. In this plan a system logs it (`info!`) and nothing else reads it.
- `next_open(current: Option<Energy>, clicked: Energy) -> Option<Energy>`: the rule of D2, a plain function with tests.
- `slide_value(progress: f32) -> f32`: the easing of the slide, a plain function with tests.

### Art

Three files. The user draws all of them. Claude only creates the two new files so that the code has something to load.

| File | Size | Who | Content |
|---|---|---|---|
| `assets/art/backgrounds/hud.aseprite` (exists) | 512 by 160 | User | Redrawn for the bar at the bottom. Claude does not edit it. |
| `assets/art/ui/weapon_drawer.aseprite` (new) | 512 by 96 | Claude made a flat start (palette panel, 2 pixel top border), the user redrew it and renamed it from `weapon_drawer_placeholder` (2026-11-11) | One frame. No suffix any more. |
| `assets/art/ui/weapon_icons.aseprite` (new, **empty**) | 32 by 32 | User fills it | One tag per `WeaponKind`, one transparent frame each. The tag names are the `icon_tag()` values: `fireball`, `fire_ring`, `ice_ring`, `fire_wall`, `ice_wall`, `whirlpool`. No suffix, because Claude draws nothing in it. |

- Make the two new files with a small Lua script from the scratchpad (not kept). Use palette colours only for the drawer panel. Create all icon frames and tags after all frames exist (`art-pipeline.md` rule: tags last), then run `./dev-scripts/aseprite.sh check` and read the tag ranges. They must not overlap. Do not overwrite a file that already exists (`git status` first).
- While the icons are empty, the buttons show only their label. Nothing in the code breaks: an empty frame draws nothing.
- Update `games/long-way/docs/art-pipeline.md` (the files) and `art-tags.md` (the icon tags). Do not touch `diodrama-assets`.

### Script runner

A test must be able to click the HUD and the drawer. The runner today sends a world click only. Add `ui_click:<x>:<y>`: a left click at a window position in art pixels, measured from the top left of the window. Step 6 checks how to do it:

- **A (recommended):** send Bevy `PointerInput` messages (press and release at the position). The whole path is tested: hit test, `Interaction`, click-through.
- **B (fallback):** `press:<name>`, which looks up an entity by `Name` and triggers its click event. Simple, but it skips the hit test and the click-through rule.

Record the choice in `dev-docs/extensions/dev-feedback.md`.

## Steps

### 1. Move the HUD below the game

- `main.rs`: `BarBottom { ratio: HUD_BAR_RATIO }`. Update the comments in `camera.rs` ("the lower 576 by 576", "the bar above it").
- Run `./dev-scripts/rust.sh script long-way "wait:0.5;screenshot:bar;quit" --debug` and look at the screenshot. Check: the bar is at the bottom, the energy row and the HP row are inside it, the F2 info box and the grid are in the game region, a click near the player still walks (`click:` in the game region).
- Show the picture to the user. The bar art is still the old one: the user redraws `hud.aseprite` (D6).
- Check: `./dev-scripts/rust.sh lint`. `test game-kit` (the `BarBottom` tests exist).

### 2. Kit: a click on a marked UI node is not a world click

- `crates/game-kit/src/pointer.rs`: add `BlocksWorldClick` (a marker component). `send_clicks` sends nothing when the primary mouse pointer hovers an entity with the marker (or a child of one). Keep the pure part testable: a function that takes the hovered entities and the marker query result.
- Tests in `pointer.rs`: a click on a marked node sends nothing, a click next to it sends `WorldClick`, no hover map entry still works. Use a small `App` with a hand-made `HoverMap`.
- Docs: `dev-docs/extensions/pointer.md` (specification rule and the test list). ADR in `dev-docs/adr/EXTENSIONS-INPUT.md` (next free number): why a marker and not a check of every UI node.
- Check: `./dev-scripts/rust.sh test game-kit`, `lint`, `build r-town`.

### 3. The drawer, with no weapons behind it

- New file `games/long-way/src/weapon_menu.rs` with `WeaponMenuPlugin`, added to `main.rs`. Add `pub struct`s and functions from "Messages and types". Put every number (heights, times, font size, colours) at the top of the file with a comment.
- First make the two new art files (see "Art"), after a `git status` check.
- The drawer root as above, hidden at start. Its picture is `weapon_drawer.aseprite`, on its own absolute, full-size child with `ImageNode` in stretch mode and `AseAnimation`, like the HUD bar (`adr-lw25`). A system moves `bottom` with `slide_value` while `DrawerState` changes. It uses `Time<Real>`.
- Rebuild the button row when the open energy changes. Icons come from `weapon_icons.aseprite` (empty for now). The `weapons_for` table of D1.
- Mark the root and the buttons `BlocksWorldClick`.
- Unit tests (inline, `adr-w7`): `next_open` (open, same icon closes, other icon switches, none), `slide_value` (0, 1, monotonic, clamped), `weapons_for` (no empty label, at most 4 per energy, every `WeaponKind` appears once), the 10 character label limit.

### 4. Make the energy icons clickable

- `hud.rs`: each energy entry gets `Button` and an observer that sends `EnergyIconClicked(energy)`. A marker `BlocksWorldClick` is not needed there (the HUD region sends no `WorldClick`).
- Hover feedback on the icon: a small tint with `Interaction` (a placeholder, you may change it).
- Drawer systems read `EnergyIconClicked` and apply `next_open`.

### 5. Weapon buttons

- A click on a weapon button writes `WeaponChosen`, sets `DrawerState.open = None` (the drawer slides down, D3). A system logs `weapon chosen: <name>`.
- A click in the game area while the drawer is open closes it (D4). The `WorldClick` still reaches the player.
- The drawer does not touch `Time<Virtual>`, `PlayState` or `Inventory`.

### 6. Script runner: click the UI

- Add `ui_click:<x>:<y>` (option A, or B if A fails in a short try). Parse test in `script.rs` like the `click:` test. Document it in `dev-docs/extensions/dev-feedback.md` and `dev-scripts.md` if it lists commands.
- Tests: the parse test. The behaviour is checked by the runs in step 7.

### 7. Verify

Run each with `--debug`, and look at the screenshots. Use `./dev-scripts/shot.sh` for a zoomed picture of the drawer.

| Check | How |
|---|---|
| Bar at the bottom, nothing overlaps it | `script long-way "wait:0.5;screenshot:bar;quit" --debug` |
| Click on the red icon opens the drawer, 96 art pixels high, full width, over the game area | `ui_click:` on the red icon, `wait:0.4;screenshot:open` |
| The drawer is half way during the slide | `wait:0.1;screenshot:mid` |
| Buttons are in one row, icon above label, labels fit | screenshot, zoomed |
| Click on a weapon button logs the choice and the drawer closes | `ui_click:` on a button, `wait:0.4;screenshot:closed`, read the log |
| The player does **not** walk when a button is clicked | position of the player before and after (log or two screenshots) |
| The game keeps running while the drawer is open: a ghost keeps moving | two screenshots, 1 s apart, or the ghost position in the log |
| Click in the game area while open closes the drawer and the player walks | `click:` in the game area |
| Click on the same icon closes it, another icon switches the buttons | `ui_click:` twice |
| Violet and black show the empty drawer | `ui_click:` on each |

- `./dev-scripts/rust.sh lint`, `test` (all crates), `web long-way`, `build r-town`.
- Disk routine from `CLAUDE.md`: `./dev-scripts/rust.sh size` (no `Cargo.toml` change here, so no new gate).
- State clearly what was **not** seen on screen: the slide in real time (stills only), hover and pressed colours, the web run, the real mouse (the script injects clicks).
- Delete files in `target/screenshots/`.

### 8. Migrate and delete

- ADRs (next free number in each file, why and not how):
  - `games/long-way/docs/adr.md`: the HUD is below the game, the weapon drawer is a UI root on the game camera, the drawer height is 60% of the bar, the game does not pause, D1 to D5 as decided. Mark `adr-lw24` as amended ("the bar is below the game since adr-lw3x") and fix its wording.
  - `dev-docs/adr/EXTENSIONS-INPUT.md`: `BlocksWorldClick` (step 2).
- Docs: `dev-docs/extensions/screen-layout.md` (the Long Way column: `BarBottom`; a note that a UI root on the game camera can overlap the game region and a child of `Hud` cannot), `pointer.md` (done in step 2), `games/long-way/docs/source-layout.md` (new `weapon_menu.rs`, the changed `hud.rs`), `art-pipeline.md` and `art-tags.md` (the icon file), `dev-feedback.md` (`ui_click`), `docs/README.md` if it lists screen layout or script commands. Check `CLAUDE.md` for lines to change.
- `PLAN-weapons.md` (ask first, then edit):
  - D5 and P10: the drawer is now the real trigger. The env variable `LONG_WAY_WEAPON_TEST` may stay for stress runs only, and the activation plan decides.
  - P4 to P7: scripted runs can use `ui_click:` on the drawer.
  - The aim question (D5 here) goes into the activation plan.
- `dev-docs/ROADMAP.md`: add a line to `rm9` and `rm12` (the drawer exists, the HUD is below).
- Delete this plan. Git history keeps it.
- Ask before each commit. Do not push. Suggested commits: (1) HUD below, (2) kit `BlocksWorldClick`, (3) drawer and icons, (4) script command, (5) docs.

## Risks

| Risk | Handling |
|---|---|
| The old `hud.aseprite` looks wrong at the bottom until the user redraws it | D6. Expected. Not a fault in the code. |
| The icon file is empty, so the buttons look bare in screenshots | Expected until the user draws the icons. Step 7 checks layout with labels only. |
| The drawer root on the game camera is covered by the map fade or by `state_root` screens | Step 3 looks at the order. The fade should stay above the drawer. `UiTargetCamera` and `GlobalZIndex` are the tools. Use named values, no bare numbers (`adr-x9`). |
| Picking does not see the drawer on the web | Step 7 includes `web long-way` for the compile. A run in the browser is not part of this plan. Say so in the report. |
| `HoverMap` is one frame late, so a fast click passes through | Test in step 2. The marker check runs in `PreUpdate` after picking. If it is late, read the node under the cursor from `UiStack` instead. |
| Script `PointerInput` injection does not reach `ui_picking` | Fall back to option B and say what the script cannot test. |
| Four buttons with long labels do not fit 512 pixels | The 10 character limit and its test. |
