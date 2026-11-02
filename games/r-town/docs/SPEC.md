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

- One fixed hero: pack character 9 (`characters/9.png`, 14x16 px).
- Animation is done in code on the one sprite (adr-rt3): flip left and right and a one pixel bob while walking. The idle squash is not built yet. Up and down use the same sprite.
- The hero entity is at the feet and the sprite is a child (adr-rt6). Arrow keys and WASD move the hero.
- Walk speed: 80 art pixels per second (5 cells per second), the same pace as Guy in Long Way in body heights.
- Feet box: 10 px wide and 4 px tall at the bottom of the sprite. It fits through a one-cell gap.
- Abilities and stats: not yet decided.

## 5. World and maps

- Grid cell is 16x16 art pixels (adr-rt3). The tileset is 320x192, so 20x12 tiles.
- The first playable is one town map of 80x60 cells (1280x960 art pixels) that scrolls. The camera follows the hero and stops at the map edges.
- The camera shows 240 art pixels of height (15 cells), so the view is 26.7 cells wide at 16:9 (adr-rt4).
- Maps are markdown map files from the shared kit (adr-x13). The test map is `assets/maps/test.md`: 80x60 cells, about 90 big and dead trees scattered over open ground, with the area around the start (cell 40, 30) kept free. The ground is a flat placeholder colour. The map size is `MAP_CELLS` in `src/map.rs` (adr-rt6).
- Blocking: water, houses, trees, walls, fences, wells, barrels, crates and chests all block the hero.
- Footprints: houses and trees block at the base only. The hero walks behind the top and may overlap from below, as in Long Way (adr-lw1). Other objects use whole-tile footprints.
- Scaling: fractional scale is allowed, as in Long Way (adr-rt4). On 1080p the scale is 4.5.
- Structure beyond the town (connected screens, one large map, hub plus maps): not yet decided.

## 6. Interaction

Deferred: NPC dialogue, chests, doors, shops.

## 7. Combat

Not yet decided. Pack material that fits: weapons, projectiles, hearts HUD, nine monsters.

## 8. Progression and save

Deferred.

## 9. UI

None in the first playable. The kit's debug info box and grid stay available. Pack material for later: hearts, boxes, two bitmap fonts.

## 10. Art

Source pack: "Superpowers Medieval Fantasy Asset Pack (part 1)" by Pixel-boy, in `games/r-town/input/medieval-fantasy-1/` (gitignored).

| Folder | Contents |
|---|---|
| `characters/` | `0-characters.png` is 176x17 with 11 characters. Singles 1 to 10 are static, 9 to 16 px (hero is 9: 14x16) |
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

- First playable (built, 2026-11-02): walk around a test map of 80x60 cells, like Long Way's `rocks-test.md`. Hero, tile map, collision, scrolling pixel camera. It proves collision, scrolling and art. No enemies, no dialogue, no UI, no audio. Built on the shared crate `crates/game-kit` (adr-x12).
- The real town layout is designed after the first playable.
- Later: combat, interaction, progression, audio, UI.
- Out of scope: public release.

## 13. Open questions

- The hero's feet box is 10x4 but the kit collision works on a point: only the 5 px half width is used, and the 4 px is the depth of the blocked band at a tree base. Decide whether a real box is needed.
- Idle squash for the hero (adr-rt3) is not built.
- The pack's other categories (monsters, items, hud, animals) are not packed into `.aseprite` files yet. The terrain sheet `0-tileset.png` is not either, and the ground is a flat colour.
- The real town layout (after the first playable). The first playable uses a test map.
- Exact base footprint size of each house and tree kind (in cells).
- Overlap from below: Long Way removed it on 2026-11-02 (trees stop at the base line, like rocks). Section 5 and adr-rt5 say the hero "may overlap from below", so decide whether R-Town keeps that.
- Collision geometry: Long Way's `Trunk` and `GUY_*` values are in 60 px art units, so they need R-Town values (shared-crate plan step 4).
- Combat model, world structure beyond the town, progression and save, interaction features, audio, UI.
