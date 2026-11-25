# Enemies

What each enemy does, its values and how the weapons affect it. Decisions: `adr-lw29` (ghost), `adr-lw32` (enemy hit points), `adr-lw41` (volcano), `adr-lw42` (no friendly fire). Balance rules: `balance.md`. Map markers: `maps.md`. Art tags: `art-tags.md`. Code: `ghost.rs`, `volcano.rs`, `enemy_hp.rs`.

## How an enemy works

These rules apply to every enemy.

1. **Placed by a map marker.** A legend kind `spawn_<enemy>` puts one enemy in its cell when the map loads (`adr-lw29`). There is no spawner and no respawn. The enemy belongs to the map: it goes when the map unloads and comes back when the map loads again. No state is saved.
2. **Legend character.** A special character, not a letter, digit, `@`, `.`, `|` or `"` (`adr-x37`, "Legend conventions" in `dev-docs/extensions/map-files.md`).
3. **Hit points.** An enemy that weapons can hit has `EnemyHp { current, max }` (floats, `adr-lw32`). Weapons send `DamageEnemy`. They never change `EnemyHp` directly. At 0 the enemy sends `EnemyDied` once, and its own plugin plays its death and removes it. Weapons only look for entities with `EnemyHp`, so an enemy without it cannot be hit.
4. **Hurt.** An enemy with its own hurt frames carries `HurtFrames` and plays its `Hurt` tag. Every other enemy flashes `#993D41` for 0.12 s (`HURT_COLOUR`, `HURT_FLASH_SECS` in `enemy_hp.rs`).
5. **Damage to the player.** An enemy sends `DamagePlayer { amount }` (`hp.rs`). The player has no invincibility frames: every hit counts, also two in the same frame (b15).
6. **No friendly fire.** An enemy attack hits only the player. The player's weapons do not hit enemy shots (b16, `adr-lw42`). An enemy shot uses `Projectile::without_targets()`. A reader of `ControlledTouched` filters on its own marker (`With<Ghost>`, `With<VolcanoShot>`).

## Values

**The values are placeholders until each enemy is tuned with the user.** The player walks at 300 px/s and has 3 HP (`adr-lw28`).

| Enemy | Legend | HP | Moves | Attack | Damage to the player | Hurt | Status |
|---|---|---|---|---|---|---|---|
| Ghost | `!` | 100 (`HIT_POINTS`) | 90 px/s (0.3 × the player), kit strategy from the legend | Touch: plays `Attack`, then dies | 1 HP per touch, once | Own `Hurt` frames | In the game since 2026-11-09 |
| Volcano | `^` | None yet: cannot be hit | Does not move | Two rolling shots per `Attack` loop | 1 HP per shot that touches | None yet | Shots in the game since 2026-11-23. Hit points to come |

## Ghost

`ghost.rs`, `characters/ghost.aseprite`, `adr-lw29`.

**Design.** A slow chaser. It patrols near its home cell until it sees the player, then it chases the player and never stops. On touch it hits once and dies.

**Behaviour.**
- The marker puts its feet at the middle of its cell.
- The legend entry sets its strategy (`adr-x40`): `strategy=sentry radius=<px> sight=<px>`, `patrol radius=<px>` or `pursue`. A bad entry is logged with the map and line, and the ghost is skipped.
- It sees the player within `sight` pixels when nothing blocks the line.
- The kit walks it (`game_kit::movement`, `nav`). Its feet stay 12 px inside the map edge (`EDGE_MARGIN`).
- On touch it stops and plays `Attack` once. When `Attack` ends, the player loses 1 HP (`DAMAGE`) and the ghost plays `Die` and is removed.
- Killed by a weapon, it plays `Die` and does not hurt the player. A ghost that dies during `Attack` never lands its hit.
- A stunned ghost (ice ring) does not attack until the stun ends.

**Body.** Drawn at 0.6 × the art (60x60 canvas). Hit box 12 × 21.6 px, standing on the feet (`HIT_BOX`).

## Volcano

`volcano.rs`, `characters/volcano.aseprite`, `adr-lw41`.

**Design.** A still turret that covers an area with rolling shots. The user's design and tuning (2026-11-23).

**Behaviour.**
- The marker puts the bottom of its body 1 px above the bottom middle of its cell, on whole pixels.
- It loops `Attack` (10 frames of 100 ms).
- At frame 7 and frame 9 of the loop it launches one shot from the middle of the crater of that frame, in a random direction (`LAUNCHES`, `GameRng`). `LONG_WAY_SEED` gives repeatable runs.
- A shot follows the kit `ProjectilePath` (`adr-x54`): 5 cells in 2.5 s, a first arc to 96 px, 2 bounces at half height, then a roll for the last 30 % of the time (`SHOT_PATH`). Then it is removed.
- A shot collides where the ball is drawn (`SHOT_HIT_BOX`). A blocked area or the map edge removes it. Its own volcano's base does not (`Projectile::ignoring`). A touch on the player takes 1 HP (`SHOT_DAMAGE`) and removes the shot.
- No limit on shots in flight. No shadow.

**Body.** Drawn at 2x (128x128, `adr-a11`): about 2 cells wide, and it reaches into the row above. The base of the cone (art rows 50 to 62, x 11 to 53, grown sideways by the player's half width) is a `Blocker`. The player walks up to it and behind it. Only the layers `base`, `rim` and `lava` are drawn (`LAYERS`). `test_bg` is never drawn.

**Placement.** Give it about 5 cells of open ground. Blocked areas and the map edge band remove shots early. The bottom rows of a map can be under the HUD.

**Open.** Hit points, how it shows a hit (no `Hurt` tag yet, so the colour flash) and how it dies. When it gets `EnemyHp`, check b2 and b13 (`balance.md`), and fill in its row in "Against each weapon".

## Against each weapon

Weapon values are in `weapons.md`. The balance rules in brackets keep these results true.

| Weapon | Ghost (100 HP) | Volcano |
|---|---|---|
| Fire ring | Dies on the first tick, 100 (b1) | Not hit |
| Ice ring | Pushed to the edge for 50, 10 more if it walks in. One ring never kills (b3). A pinned ghost is stunned | Not hit |
| Tornado | 13 per tick, bumped back 40 px on every second tick. Dies after more than 60 % of the tornado's time in contact (b5). Gets through to a player who stands still (b7, b9) | Not hit |
| Fireball | A direct hit (150) kills (b10). The blast (100) kills (b11) | Not hit, and not a target |

## Add a new enemy kind

1. Ask the user for the design: movement, attack, damage, HP, how it shows a hit and how it dies.
2. Add `Kind::Spawn<Enemy>` and the kind name in `map.rs`, and a legend character from the enemy set (`adr-x37`). Write the key in quotes.
3. Spawn it from the map load hook. Do not add a `MapId` by hand (`adr-x58`).
4. Give it `EnemyHp::full(<its own HP>)` if weapons can hit it. Do not change the ghost's HP or a weapon's damage to suit it (b1, b2).
5. Hurt: add `HurtFrames` and a `Hurt` tag, or use the colour flash.
6. Its attacks hit only the player (b16): shots use `Projectile::without_targets()`, and touch readers filter on its own marker.
7. Check the "by hand" rules in `balance.md`: b2 (fire ring), b13 (fireball direct hit) and b16 (friendly fire).
8. Add a row to "Values" and a column to "Against each weapon" here, its marker to `maps.md` and its tags to `art-tags.md`.
