# R-Town: specification

Status: draft, interview complete for the first playable. Later topics are deferred. Process: `dev-docs/WORKFLOW.md`. Decisions: `adr.md`.

## 1. Summary

R-Town is a real-time, top-down, Zelda-like action adventure RPG for macOS and Web. Explore, fight monsters, collect items, unlock areas. Further details follow from the interview.

## 2. Targets and timing model

- Targets: macOS native and WebAssembly (workspace rule).
- Timing model: real-time. Free movement and combat use `time.delta_secs()`, as in Long Way (adr-lw7). Not turn-based, not grid-locked.
- Distribution: personal use only, inside this existing repo (adr-rt2). Revisit before any public build.

## 3. Core loop

Zelda-like adventure: explore, fight monsters, collect items, unlock areas. Only exploration is in the first playable.

## 4. Player

- One fixed player: pack character 9 (`characters/9.png`, 14x16 px).
- Animation is done in code on the one sprite (adr-rt3): flip left and right and a one pixel bob while walking. The idle squash is not built yet. Up and down use the same sprite.
- The player entity is at the feet and the sprite is a child (adr-rt6). Arrow keys and WASD move the player.
- Walk speed: 80 art pixels per second (5 cells per second), the same pace as Guy in Long Way in body heights.
- Feet box: 10 px wide and 4 px tall at the bottom of the sprite. It fits through a one-cell gap.
- Facing: the last move direction, 8 directions, right at the start (adr-rt11).
- Weapon: X fires a projectile along the facing at 160 art px per second (twice the walk speed). It disappears at a blocked area or the map edge. One shot per key press, no cooldown, no range limit, no damage yet (adr-rt11).
- Hit box (what enemies touch): 8 px wide and 12 px high, from 2 px above the feet (`adr-rt12`).
- Other abilities and stats: not yet decided.

## 5. World and maps

- Grid cell is 16x16 art pixels (adr-rt3). The tileset is 320x192, so 20x12 tiles.
- The first playable is one town map of 60x60 cells (960x960 art pixels) that scrolls. The camera follows the player and stops at the map edges.
- The camera shows 240 art pixels of height (15 cells), so the view is 26.7 cells wide at 16:9 (adr-rt4).
- Maps are text grid files from the shared kit (adr-x13, adr-x18). The test map is `assets/maps/test.txt`: 60x60 cells with `origin: top-left`. It starts blank, with the player start marked `@` (`player_start`) at column 10 on row 2 (file numbers, counted from the top, same as the F3 grid), and is filled in by hand. The ground is a flat placeholder colour. The map size is the `size` key in the file (adr-rt7).
- Blocking: water, houses, trees, walls, fences, wells, barrels, crates and chests all block the player.
- Groups (adr-rt9): items that touch, also at a corner, form a group. An item alone blocks the base band. A member of a group blocks its whole cell. See `dev-docs/extensions/groups.md`.
- Footprints: houses and trees block at the base only. The player walks behind the top and may overlap from below, as in Long Way (adr-lw1). Other objects use whole-tile footprints.
- Scaling: fractional scale is allowed, as in Long Way (adr-rt4). On 1080p the scale is 4.5.
- Structure beyond the town (connected screens, one large map, hub plus maps): not yet decided.

## 6. Interaction

Deferred: NPC dialogue, chests, doors, shops.

## 7. Combat

First slice (2026-11-04, `adr-rt11` to `adr-rt13`):

- The player fires projectiles with X (section 4).
- Two enemies, the skeleton and the goblin, come from spawners in the map file: the skeleton every 10 s walks a random patrol within 120 px at 0.5 x the player's speed, the goblin every 15 s pursues the player at 0.2 x. The first of each appears at game start. They walk around trees and water with path finding (`dev-docs/extensions/nav.md`).
- A projectile kills the enemy it hits. An enemy that touches the player kills the player: the player entity is removed. F1 resets everything and puts the player back at the start.

Not yet decided: hit points, damage, a death screen and restart flow, hearts HUD, more enemies and weapons, enemy attacks. The pack has seven more monsters (bat, cyclop, dragon, king skeleton, leonard, slim, snake) that are not packed yet.

## 8. Progression and save

Deferred.

## 9. UI

None in the first playable. The kit's debug info box and grid stay available. Pack material for later: hearts, boxes, two bitmap fonts.

## 10. Art

Source pack: "Superpowers Medieval Fantasy Asset Pack (part 1)" by Pixel-boy, in `games/r-town/input/medieval-fantasy-1/` (gitignored).

| Folder | Contents |
|---|---|
| `characters/` | `0-characters.png` is 176x17 with 11 characters. Singles 1 to 10 are static, 9 to 16 px (player is 9: 14x16) |
| `monsters/` | bat, cyclop, dragon, goblin, king skeleton, leonard, skeleton, slim, snake. Single sprites, 6 to 51 px |
| `background-elements/` | `0-tileset.png` 320x192 with grass, wood, stone and water tiles, houses, trees, gates, wells. Singles 1 to 21 |
| `items/` | weapon, element, food and potion, projectile, gold and gem, emblem, chests, barrels, crates |
| `animals/` | `0-animals.png` and singles |
| `hud/` | hearts (12x11), life boxes, 16x16 and 32x32 boxes |
| `FX/` | 11 effects, PNG and GIF |
| `sounds/`, `music/` | WAV effects, 6 OGG themes, forest ambience |
| `font-1.png`, `font-2.png` | Bitmap fonts |

Decisions:
- Pack PNGs are grouped into `.aseprite` files by pack folder (`characters`, `background-elements`, later `monsters`, `items`, `hud`), one tag per image, built by `dev-scripts/aseprite/pack_pngs.lua` (adr-rt1, adr-rt6). The catalogue of frames and tags is `art-tags.md`.
- The pack has no walk frames and no facing sprites. Characters are animated in code (adr-rt3).
- License: free for commercial use, "do not redistribute". Personal use in this repo only (adr-rt2).

## 11. Audio

Deferred. Pack has 6 music themes and 20+ effects.

## 12. Scope

- First playable (built, 2026-11-02): walk around a test map like the Long Way test map (first 80x60 cells with scattered trees, from 2026-11-02 a blank 60x60 map that is filled in by hand). Player, tile map, collision, scrolling pixel camera. It proves collision, scrolling and art. No enemies, no dialogue, no UI, no audio. Built on the shared crate `crates/game-kit` (adr-x12).
- The real town layout is designed after the first playable.
- Later: combat, interaction, progression, audio, UI.
- Out of scope: public release.

## 13. Open questions

- The player's feet box is 10x4 but the kit collision works on a point: only the 5 px half width is used, and the 4 px is the depth of the blocked band at a tree base. Decide whether a real box is needed.
- Idle squash for the player (adr-rt3) is not built.
- The pack's other categories (monsters, items, hud, animals) are not packed into `.aseprite` files yet. The terrain sheet `0-tileset.png` is not either, and the ground is a flat colour.
- The real town layout (after the first playable). The first playable uses a test map.
- Exact base footprint size of each house and tree kind (in cells).
- Overlap from below: Long Way removed it on 2026-11-02 (trees stop at the base line, like rocks). Section 5 and adr-rt5 say the player "may overlap from below", so decide whether R-Town keeps that.
- Collision geometry: Long Way's `Trunk` and `GUY_*` values are in 60 px art units, so they need R-Town values (shared-crate plan step 4).
- Combat model, world structure beyond the town, progression and save, interaction features, audio, UI.
