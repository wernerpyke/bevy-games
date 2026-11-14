# PLAN: each cast of a weapon costs energy (Long Way)

The user's brief (2026-11-14): a weapon costs energy to cast. Each weapon has its own cost per cast. The active weapon icon shows a 50% alpha grey overlay, a vertical gauge of the share of the cost the player has. A cast takes the cost from the inventory. Added the same day: the active button is disabled until the player has enough energy, and all gauges move smoothly instead of jumping.

## Status (2026-11-14): resume here

Not started. Decisions D1 to D12 are made, and no question is open. Next: E1.

## What is already in place

- `inventory.rs`: `Inventory` holds one count per energy, from 0 to `MAX_ENERGY` (10). Pickups add 1 (`adr-lw27`, `adr-lw34`). Energy has no use yet.
- `weapon_menu.rs`: `ActiveWeapons` (one slot per energy), `WeaponKind`, `weapons_for` (the drawer order).
- `weapons/mod.rs`: `cast_active` turns `ActiveWeaponClicked` (a click, or the keys Z X C V) into `CastWeapon`.
- `weapons/values.rs`: one `WeaponValues` row per weapon.
- `hud.rs`: `ActiveButton` per energy, the kit's `image_button` with the tags `Up` and `Down` of `ui/weapon_button.aseprite`. `update_active_buttons` puts the weapon icon (`weapon_icon`, 32 by 32) in it when `ActiveWeapons` changes. `update_gauges` sets each kit `Gauge` fraction.
- Kit `crates/game-kit/src/hud.rs` (`adr-x45`): `ImageButton { up, down }` swaps the tags on a primary press and adds Bevy's `Pressed` marker. It has **no disabled state**. `Gauge { fraction }` sets the fill width at once (`fill_gauges`).
- Bevy 0.20 has `bevy::ui::InteractionDisabled`, a marker for a widget that is greyed out and does not take input (`bevy-src/crates/bevy_ui/src/interaction_states.rs`). The kit does not use it yet.

## Decisions (the user, 2026-11-14)

| ID | Topic | Decision |
|---|---|---|
| D1 | Costs | Fire ring 1 red, fireball 3 red, tornado 1 blue, ice ring 2 blue. The cost is paid in the energy of the weapon's drawer. |
| D2 | Start energy | 0 of each. Pickups are the only source, so the first cast needs a pickup. |
| D3 | Overlay direction | The missing share of the cost is greyed from the top down. The clear part rises from the bottom as energy arrives. No overlay when the count is at or above the cost. |
| D4 | Where energy is taken | In the cast step: `cast_active` checks and takes the cost in one call (`Inventory::try_spend`). So two casts in one frame (a click and a key) cannot both use the same energy. The weapons plugin changes the inventory resource directly. |
| D5 | Overlay look | A dark layer: palette colour `#1f1932` at 50% alpha. Check it in a zoomed screenshot. |
| D6 | Overlay area | Over the 32 by 32 icon only, not the whole button. |
| D7 | Disabled button | The active button is disabled while the player has less energy than the cost of its weapon: it takes no click and does not show `Down`. The kit's image button gets a disabled state now, so a later `Disabled` tag in `weapon_button.aseprite` changes only the art. |
| D8 | A cast without enough energy | Nothing happens. The log has one line. The overlay and the disabled button show why. |
| D9 | Disabled art | This plan adds a `Disabled` frame and tag to `ui/weapon_button.aseprite`, and the active buttons use it. The kit still supports a button without a disabled tag (`disabled: None` shows `up`). |
| D10 | A cast key on a disabled button | The key does nothing, the same as a click, so an open drawer stays open. A blank slot has no cost and is not disabled: a click or key on it still closes the drawer (the user, 2026-11-12). |
| D11 | Gauge speed | A constant speed: empty to full takes 0.3 s (one pickup moves an energy gauge in 0.03 s). The fill snaps to whole art pixels. Game time, so a gauge stops while the game is paused. |
| D12 | Overlay when the weapon in a slot changes | It moves smoothly too: one rule for every change. |

## Open questions

None. Ask if a step shows something new.

Not in this plan: a cooldown per cast (`rm9`), costs in the drawer buttons, the wall costs (set in `PLAN-walls.md` when each wall is built; until then a placeholder, and the walls cannot be cast), a held cast key that casts again (`rm100`).

## Steps

### E1. Costs in the values table

- Add `cost: u32` to `WeaponValues` (`weapons/values.rs`), with the D1 values. The walls get a placeholder (1) with a comment.
- Test: the cost of each built weapon, exact values. Every cost is at least 1 and at most `MAX_ENERGY`, so every weapon can be cast with a full gauge.

### E2. Spending in the inventory

- `Inventory::try_spend(energy, cost) -> bool`: takes `cost` and returns true when the count is at least `cost`. Otherwise it changes nothing and returns false.
- `Inventory::cost_share(energy, cost) -> f32`: the share of `cost` the player has, from 0 to 1 (1 at or above the cost).
- Plain Rust. Unit tests with different counts and costs: exactly enough, one short, 0, at the maximum, a share of 1/3 and 2/3.
- Update the module comment ("Energy has no use yet").

### E3. A cast takes its cost

- `cast_active` (`weapons/mod.rs`) gets `ResMut<Inventory>`. Check the player first, then `try_spend(energy, values(weapon).cost)`. Without enough energy it sends no `CastWeapon` and logs `weapons: <weapon> needs <cost> <energy>, has <count>`.
- This check stays even with the disabled button (E4): it is the rule, and the button only shows it.
- Tests (update the two that exist):
  - A cast takes exactly its cost from its own energy and no other.
  - With too little energy nothing is cast and nothing is taken.
  - Two casts of red in the same frame with energy for one: one `CastWeapon`, count 0.
  - A blank slot, a weapon that is not built, and no player take nothing.
  - Prove each test can fail (`dev-docs/testing.md`).

### E4. A disabled state for the kit's image button, used by the active buttons

Kit (`crates/game-kit/src/hud.rs`):
- `ImageButton` gets `disabled: Option<&'static str>`, the tag shown while the entity has `InteractionDisabled`. With `None` it shows `up`. The kit has no assets, so the tag is the game's choice.
- While disabled: a primary press does not show `down` and does not add `Pressed`. When `InteractionDisabled` is added during a press, the button drops `Pressed` and shows the disabled tag. When it is removed, the button shows `up`.
- The kit cannot stop `PointerClick` from reaching the game's observer. The kit doc says so: a game checks `InteractionDisabled` in its click observer.
- Tests in `game-kit`: a press while disabled changes nothing; disabled during a press; enabled again shows `up`; a disabled tag is shown when one is given, `up` when not. Neutral tag names, no game values. Run `./dev-scripts/rust.sh test game-kit`.

Art (`ui/weapon_button.aseprite`, D9). Read `art-pipeline.md` first.
- Add a `disabled=1` option to `dev-scripts/aseprite/draw_button.lua`: a third frame, tag `Disabled`. Recommended look: flat, no bevel (both bevel edges in the face colour), so it reads as not pressable, with an optional `disabled_face=` index (default: the face). Without the option the script makes the same two frames as today.
- The file was made by the script on 2026-11-12 and is "a basic design, not a placeholder". Before replacing it, make the new file in the scratchpad with `width=126 height=56 disabled=1` and check with an Aseprite script that its `Up` and `Down` frames are the same pixels as the current file. If they differ, the user changed the file by hand: stop and ask. Otherwise replace it (the script refuses to overwrite, so move the old file out first).
- `./dev-scripts/aseprite.sh check` on the new file (tags do not overlap). A zoomed screenshot of a disabled and an enabled button side by side for the user.

Game (`hud.rs`):
- `image_button(..., disabled: Some("Disabled"))` for the active buttons.
- A system adds or removes `InteractionDisabled` on each `ActiveButton` when `Inventory` or `ActiveWeapons` changes: disabled when the slot holds a built weapon and the count is below its cost. A blank slot is not disabled (D10).
- `click_button` sends nothing for a disabled button. `press_weapon_keys` sends no `ActiveWeaponClicked` for the cast key of a disabled button, so an open drawer stays open (D10). The drawer keys 1 to 4 are not affected.
- Tests: disabled per energy at counts below, at and above the cost; it follows a change of weapon in the slot; a blank slot is never disabled; a primary click and a cast key on a disabled button send no `ActiveWeaponClicked`, and on a blank slot they still do.

### E5. The overlay on the active icon

- A child node over the icon in each `ActiveButton`: absolute, from the top of the icon, full width, height `round((1 - share) * 32)` art pixels, `#1f1932` at 50% alpha (D5, D6). Whole pixels, so the edge stays sharp.
- `update_active_buttons` despawns and builds the button's children again, so the overlay must be built with the icon, or kept outside the children it despawns.
- Tests: the height per energy for counts 0, 1 of 3, 2 of 3, 3 of 3 and above the cost (hidden). It follows a change of weapon (fire ring at 1 red: none; load the fireball: 21 px). A blank slot has no overlay.

### E6. Smooth gauges (the energy gauges and the overlay)

- Kit: a plain function `approach(shown, target, max_step) -> f32` that moves `shown` towards `target` by at most `max_step` and never past it. Unit tests: up, down, the last step lands exactly, no overshoot, already there.
- Kit `Gauge`: the game sets `fraction` as today. A new `shown` value moves towards it at `GaugeStyle::fill_secs` (seconds for empty to full; 0 means at once, so other games keep today's behaviour). `fill_gauges` draws `shown`, snapped to whole art pixels of the track width. Tests: with 0 it is at once; with a time it reaches the fraction after `fill_secs × change` and not before; a new fraction during a move turns it round.
- Game: Long Way sets `fill_secs` to 0.3 (D11) for the energy gauges. The overlay keeps its own shown share, moved with the same `approach` at the same speed, also when the weapon in the slot changes (D12). Test: after loading the fireball at 1 red the overlay grows from 0 to 21 px over 0.3 × 2/3 = 0.2 s.
- The disabled state (E4) and the cast check (E3) use the real count, not the shown value. So the button can be enabled a moment before the overlay is gone. Check that this looks right on screen.

### E7. Scripted run

- Map 1 has 11 red and 11 blue pickups. Find their cells in `maps/1.txt`. Then cast with 0 (nothing cast, one log line), teleport onto pickups, cast again, and read the `inventory:` and `weapons:` lines.
- Zoomed screenshots of the HUD (`dev-scripts/shot.sh`, `--debug`) at 0, 1 and 3 red with the fireball loaded, and one taken during a gauge move. Show them to the user for D5 and D9.
- Say what was not seen on screen (the motion of the gauges, the feel of the disabled click).

### E8. Gates

- `./dev-scripts/rust.sh lint`, `test game-kit`, `test long-way`. Run `web` once: no new dependency, but the kit changed. Delete `target/screenshots/` files. Run the disk use check (`CLAUDE.md`).

### E9. Migrate lasting content, then delete this plan

- `games/long-way/docs/adr.md`: a new ADR (next free ID, `adr-lw39`): costs per weapon, 0 at the start, the overlay, the disabled button, the spend in the cast step, smooth gauges, and why.
- `dev-docs/adr/EXTENSIONS-SCREEN.md`: amend `adr-x45` (or add an ADR) for the disabled state and `fill_secs`. `dev-docs/extensions/screen-layout.md`: the two widget changes.
- `dev-docs/bevy/concepts.md`: a section for `InteractionDisabled` (first use in the project). `dev-docs/bevy/notes.md` if a pattern needed checking.
- `games/long-way/docs/weapons.md`: a Cost column in the values table, "How a weapon works" step 2 (the cost check), the new log line, script notes (collect pickups before a cast). `art-tags.md` and `art-pipeline.md`: the `Disabled` frame of `weapon_button.aseprite` and how it was made. `dev-docs/extensions/dev-scripts.md`: the `disabled=` and `disabled_face=` options of `draw_button`.
- `games/long-way/docs/source-layout.md` if a file gets a new job. `docs/README.md`: casting needs energy from pickups.
- `dev-docs/ROADMAP.md`: `rm9` (energy cost done, cooldown still open), `rm100` ("A held cast key"). `dev-docs/PLAN-walls.md`: remove energy cost from "Out of scope", add the wall cost to the W5 and W6 design questions.
- Delete this file.

### E10. Commit

Only when the user says so.
