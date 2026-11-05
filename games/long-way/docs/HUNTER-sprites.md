# Hunter sprite pack: overview

Source: `games/long-way/input/hunter/` (character name in the files: "Clarice, nocturnal creature hunter"). The user has a full commercial licence and may edit the art.

Purpose: replace `Guy.aseprite` as the player sprite. This file lists what the pack has and what we must draw. It is based on the 15 JSON files, a pixel scan of the 15 sprite sheets, and a visual check of the Idle, Run, Jump Down and Normal Attack sheets.

## Status (2026-11-05) and handoff

The Hunter is the player in Long Way. Next work: draw the real up and down animations (see "Next steps"), in a new session.

**Done**
- The pack is analysed (below) and built into `characters/Hunter.aseprite`: 128x128, 46 frames, tags `Idle`, `Run`, `Hurt`, `Die` (from the pack) and `Up`, `Down` (placeholders from the jump frames, red tags). Palette is the game palette, grown to 32 colours (`adr-a5`).
- `player.rs` and `collision.rs` use the Hunter (`adr-lw15`): `Run` is the sideways walk, the art faces right (`WALK_ART_FACES_RIGHT = true`), the sprite is flipped with `transform.scale.x`, `HALF_SIZE` 64, `PLAYER_FEET_OFFSET` 56, `PLAYER_HALF_WIDTH` 14.
- Checked in the real game with `script --debug` and zoomed screenshots: the face points the way the player walks, and the cloak is centred on the feet marker in both directions.

**What the player code does now** (`animate_player` in `player.rs`)
- Standing plays `Idle` for every facing. Walking plays `Run` (sideways), `Up` or `Down`. Horizontal input wins on a diagonal.
- Walk animations play at `WALK_ANIMATION_SPEED` 2.5, idle at 1.0. Walk speed is 300.
- The flip stays at the last sideways value while walking up or down. That is fine for art that is symmetric, and needs a decision if the up and down art is not.
- No code plays `Hurt` or `Die`. No code uses `Block`.

**Facts for drawing the up and down frames**
- Canvas 128x128. Feet on row y=120. Body centred on x=64. The cloak is about 20 px wide, the hat about 30, the whole body about 50 tall. Every art pixel in the pack art is 2x2, so drawing at 1x pixel detail is possible but looks finer than the rest.
- `Run` has 8 frames of 100 ms. The placeholder `Up` has 6 frames (75, 75, 75, 125, 125, 125 ms) and `Down` has 6 frames of 100 ms. The real frame counts are open.
- Only colours from the game palette (32 colours; indexes 19 to 31 are the Hunter's). Index 0 is transparent.
- `Hunter.aseprite` is committed, so git is the safety net. **Do not run the rebuild command** once drawing starts: it overwrites the file. Add or replace frames inside Aseprite. Keep the tag names, because the code uses them.
- Tooling for checks: zoomed screenshots are still manual (roadmap item 1). Recipe: run `./dev-scripts/rust.sh script long-way "teleport:<x>:<y>;wait:0.4;hold:<key>:0.5;wait:0.3;screenshot:<name>;quit" --debug`, then crop the PNG in a Lua script with `Sprite:crop` and `SpriteSize { scale = 5 }`. The window is 2560x1440 and `screen = (1280 + 1.2 * world x, 720 - 1.2 * world y)`. Delete the screenshots afterwards.

## Pack format

| Item | Value |
|---|---|
| Content | 15 PNG sheets + 15 JSON files (Aseprite "hash" export, version 1.3.7), 11 GIF previews, 3 portraits |
| Source files | No `.aseprite` files. Only exports. |
| Layout | One animation per file. One row of frames, left to right. No gaps, no trimming. |
| Frame size | 96x64 for every frame in every file |
| Tags and layers | None in the JSON. The file name is the animation name. |
| Timing | Per-frame `duration` in ms (50 to 175). 144 frames in total. |
| Palette | 13 to 14 colours per sheet. 14 colours in total, all fully opaque (checked on the built file). One (white) was already in the game palette. The other 13 were appended to it (`adr-a5`). |
| Facing | Right. Found in manual testing on 2026-11-05: with the flag set for a left-facing sprite, left and right were swapped. Now `WALK_ART_FACES_RIGHT = true` in `player.rs`. (An earlier reading of the art said left.) |

All frames have the same size, so one script can build a single `.aseprite` file from them (one tag per animation). See "Build log".

## Character size and anchor

Measured from the opaque pixels:

- The body is about 22 px wide and 25 px tall (x 24 to 45, y 22 to 47 in idle). The hat top is at y 22. The feet are at y 47.
- The body is **not centred** in the 96 px canvas. Its centre is at x of about 34, not 48. The canvas is wide to leave room for attacks and effects.
- Attack and dash effects reach x 1 to 91 and y 4 to 48.
- `Guy.aseprite` is a 60x60 canvas. Our code uses `HALF_SIZE = 30` and `PLAYER_FEET_OFFSET = 30` (feet at the bottom edge of the canvas). For the Hunter the feet are 15 px below the canvas centre (y 47 of 64, centre at 32), and the body is 14 px left of the canvas centre.
- Decision: re-centre the body and crop the canvas as needed. Effects (slash arcs, dash trail) will be separate sprites, so the canvas only has to fit the body and the weapon. This changes `HALF_SIZE`, `PLAYER_FEET_OFFSET`, `PLAYER_HALF_WIDTH` and the camera clamp.
- Decision, later changed: the character was first tried at its pack size (about 25 px tall), then doubled in Aseprite (see "Decisions").

## What the pack contains

Frame numbers start at 0. Totals are the sum of frame durations.

| Animation (file name) | Frames | Total ms | Notes |
|---|---|---|---|
| Idle | 6 | 600 | Loops. Body bounds change by 1 to 2 px only. |
| Run | 8 | 800 | Loops. Even 100 ms frames. Replaces `Walk`. |
| Jump - Up | 12 | 1250 | Crouch, rise, then a wall-cling pose (frames 7 to 11 have the same bounds as the wall-hug sheets). |
| Jump - Down | 9 | 850 | Six hover frames, then landing (frames 6 to 8). |
| Fall | 8 | 825 | Falls to the ground and lies flat. |
| Hurt | 6 | 450 | Hit reaction, then back to standing. |
| Die | 14 | 1275 | Appears to be Hurt (frames 0 to 5) followed by Fall (frames 6 to 13). |
| normal attack | 11 | 1000 | Sword slash with a swing arc effect. Reach up to x 76. |
| Charge for Special Attack | 7 | 875 | Crouch and gather. |
| enter special attack | 11 | 1100 | Large lunge. Reaches x 91 and y 4. |
| Special attack | 20 | 2175 | Appears to be Charge (frames 2 to 8) followed by enter special attack (frames 9 to 19), plus 2 lead-in frames. |
| dash(no VFX) | 16 | 1100 | Frames 1 to 5 are empty (the character is not drawn). |
| dash - VFX | 11 | 675 | Effect only. Its durations match dash frames 1 to 11. Looks like an overlay on top of the no-VFX sheet. Check by eye. |
| wall hug on contact | 1 | 75 | Single frame. |
| wall hug slide down | 4 | 450 | Loop. Same pose as the wall-hug frame. |
| Portraits | 3 PNG | | 640x416 and two others. For dialog or a UI frame. Not sprites. |

Duplicates to avoid when we build the file: Die contains Hurt and Fall; Special attack contains Charge and enter special attack; Jump - Up contains the wall-cling pose. Decide per animation whether to keep the combined sheet or the parts.

### Mapping to the tags the game uses now

| The player tag | Hunter source | Status |
|---|---|---|
| `Idle` | Idle | Side only. Usable. |
| `Walk` (side) | Run | In use, tag `Run`. Play speed (`WALK_ANIMATION_SPEED`, 2.5) is not retuned for the doubled size. |
| `Up` | none | Placeholder from the jump frames. Needs drawing. |
| `Down` | none | Placeholder from the jump frames. Needs drawing. |
| `Block` | none | Gap (not used in code today). Draw or drop. |
| `Win` | none | Dropped. We will not use it. |

## What is usable in a top-down game

- **Direct use (side view, right-facing):** Idle, Run, Hurt, Die, normal attack, special attack (Charge and enter), dash.
- **Needs a mirror:** all of the above when the player moves or faces left. The game flips the sprite (`transform.scale.x = -1`), so no extra art is needed for that.
- **Probably not usable as is:** Jump - Up, Jump - Down, Fall, wall hug and wall slide. They are side-scroller poses (gravity, walls). Fall is a possible base for a fall-down death pose. The jump frames may be used as a base for a hop or a dodge if we add one.

## Gaps we need to fill

1. **Up and down views.** The pack has no pose that faces away from or toward the camera. We need at least:
   - Idle up and idle down.
   - Walk up and walk down (8 frames each, to match Run).
   - Suggested bases: Idle and Run for body shape, hat, cloak and timing. The wall-hug pose shows the face from a different angle, so check it by eye as a reference for the front view. The user draws these.
2. **Attacks (later).** The first version has no attacks. We want to build towards them. When we add them, the pack has one sideways slash and one special. Attacks up and down need new drawing. Keep the attack, charge, special and dash sheets with the source file so they are ready.
3. **Weapon position.** The flame blade shows in idle at the lower left. For up and down views we must decide where it goes and where the flame sits (it is the main colour accent).
4. **Palette.** Done (`adr-a5`): the 13 new colours were appended to `diodrama_palette.aseprite`.
5. **Canvas and anchor.** Done: 128x128, feet at y=120, collision values set (`adr-lw15`).
6. **Block.** `Guy.aseprite` has it. The pack has no equivalent. Not used in code now. Open: draw later or drop the tag. (`Win` is dropped.)
7. **Effects as separate sprites.** The dash effect is already a separate sheet. The slash arcs and the special attack effects are baked into the character frames. When we add attacks, we must split them out by hand into effect sprites (own file and tags) and remove them from the character frames. Not needed for the first version.
8. **Source file.** Done: `characters/Hunter.aseprite` is built from the PNGs with the pack's frame durations. The user's own drawings (up and down) go into this file. The PNG sheets in `input/` stay as read-only references.

## Decisions

- File: `assets/art/characters/Hunter.aseprite`.
- Canvas: 64x64 at first (one map cell), then doubled in Aseprite on 2026-11-05 to **128x128** (two cells) with nearest-neighbour scaling, so every art pixel is 2x2. The body is centred at x=64 and the feet are at y=120 (8 px margin at the bottom). In code the feet are 56 px below the canvas centre. It fits every frame, including the lying death frames. Effects are separate sprites.
- Source: no per-animation `.aseprite` files in `input/`. `input/` stays read-only. One script builds the file from the PNGs and takes the frame durations from the JSON (`dev-scripts/aseprite/tagged_sheets.lua`).
- First version: Idle, Run, Hurt and Die, plus placeholder Up and Down (taken from the jump sheets). No attacks. We add the attack, charge, special and dash sheets later with the same script.
- Palette: done (`adr-a5`). 13 pack colours were appended to `diodrama_palette.aseprite` (indexes 19 to 31, 32 colours in total). Indexes 0 to 18 did not move. `dev-scripts/aseprite/extend_palette.lua` does this.
- Scale: the character is doubled in the art file, not in the game, so the game draws it at 1x. This gives more pixels to edit by hand. The rocks are drawn at 4x.
- `Win` tag: dropped.

## Build log

`Hunter.aseprite` is built: 128x128 (the file was built at 64x64 and then doubled), 46 frames, 6 tags. Frame durations are the same as in the pack. It has one layer, `Body`, in RGB colour mode with the pack colours. It uses the expanded game palette (every pixel colour is in it, checked).

| Tag | Frames (from 0) | Source | Status |
|---|---|---|---|
| `Idle` | 0 to 5 | Idle | Final |
| `Run` | 6 to 13 | Run | Final |
| `Hurt` | 14 to 19 | Hurt | Final |
| `Die` | 20 to 33 | Die | Final |
| `Up` | 34 to 39 | Jump - Up, frames 1 to 6 (the rise, without the wall-cling frames) | **Placeholder** |
| `Down` | 40 to 45 | Jump - Down, frames 0 to 5 (the hover, without the landing frames) | **Placeholder** |

- `Up` and `Down` are placeholders. The pack has no up or down view, so these are side-view poses that look different from `Idle` and `Run`: `Up` has the flame trailing below the body, `Down` has the blade held out in front. They face right like the rest. The user replaces them with drawn up and down walk frames. The two tags are red in Aseprite so they are easy to find. The tag names stay `Up` and `Down` because the code uses them (the `_placeholder` suffix of `art-pipeline.md` is on the file name, and this file has real art as well).
- The body is centred on the cloak (source x 38), not on the whole sprite with its flame. A first build used x 34, which put the cloak 8 px off the feet marker and made it jump sideways when the sprite flipped. With x 38 no frame is cut off at the canvas edge. Check the lying `Die` frames by eye in the game.
- The red `Hurt` frames (15, 17, 19) are one-colour hit-flash silhouettes from the pack, not an error.

Rebuild command (from the repo root):

```
H=games/long-way/input/hunter; P=Clarice-noctural-creature-hunter; S=$H/SpriteSheets/$P
aseprite -b --script-param out=games/long-way/assets/art/characters/Hunter.aseprite \
  --script-param json=$H/json \
  --script-param "sheets=Idle=$S-Idle.png,Run=$S-Run.png,Hurt=$S-Hurt.png,Die=$S-Die.png,Up=$S-Jump - Up.png@1-6,Down=$S-Jump - Down.png@0-5" \
  --script-param frame=96x64 --script-param canvas=64x64 \
  --script-param from=38,47 --script-param to=32,60 \
  --script-param placeholders=Up,Down \
  --script dev-scripts/aseprite/tagged_sheets.lua
aseprite -b --script-param palette=games/long-way/assets/art/diodrama_palette.aseprite \
  --script-param sprite=games/long-way/assets/art/characters/Hunter.aseprite \
  --script dev-scripts/aseprite/extend_palette.lua
aseprite -b games/long-way/assets/art/characters/Hunter.aseprite \
  --script-param width=128 --script-param height=128 \
  --script dev-scripts/aseprite/resize.lua
```

The build makes a 64x64 file. The last command doubles it to 128x128. Rebuilding overwrites the file, including any drawing done in it. Do this only before the user starts drawing in it.

## Open

- `Block` tag: draw or drop.
- Whether 128x128 is the right size once the up and down views are drawn.
- Names of the idle tags for up and down (for example `IdleUp` and `IdleDown`), and whether standing should keep one `Idle` for all directions.
- Frame count for the up and down walks (8 to match `Run`, or fewer).
- Where the blade and flame go in the up and down views, and whether the sprite is flipped when walking up or down.
- Whether `WALK_ANIMATION_SPEED` and the walk speed need a retune now that the sprite is twice as big.
- Not seen yet: the animations in motion, and the lying `Die` frames, in the real game.

## Next steps

1. **Up and down animations (next session).** The user draws them in `Hunter.aseprite`, with Claude's help. Start by settling the open questions above. Replace the frames of the red `Up` and `Down` tags, add the idle tags, then update `animate_player` in `player.rs` and `art-tags.md` to match. Check each step in the game with `script --debug` and zoomed screenshots.
2. Wire up `Hurt` and `Die` when the game has something that hurts the player.
3. Add attack, charge, special and dash later, with the effects as separate sprites.
4. Move the lasting content of this file into `art-pipeline.md` once the up and down views are done, and delete `Guy.aseprite` if it is no longer needed.
