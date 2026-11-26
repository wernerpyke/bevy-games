# Enemies

What each enemy does, its values and how the weapons affect it. Decisions: `adr-lw29` (ghost), `adr-lw32` (enemy hit points), `adr-lw41` (volcano), `adr-lw42` (no friendly fire). Balance rules: `balance.md`. Map markers: `maps.md`. Art tags: `art-tags.md`. Code: `ghost.rs`, `volcano.rs`, `enemy_hp.rs`.

## How an enemy works

These rules apply to every enemy.

1. **Placed by a map marker.** A legend kind `spawn_<enemy>` puts one enemy in its cell when the map loads (`adr-lw29`). There is no spawner and no respawn. The enemy belongs to the map: it goes when the map unloads and comes back when the map loads again. No state is saved.
2. **Legend character.** A special character, not a letter, digit, `@`, `.`, `|` or `"` (`adr-x37`, "Legend conventions" in `dev-docs/extensions/map-files.md`).
3. **Hit points.** An enemy that weapons can hit has `EnemyHp { current, max }` (floats, `adr-lw32`). Weapons send `DamageEnemy`. They never change `EnemyHp` directly. At 0 the enemy sends `EnemyDied` once, and its own plugin plays its death and removes it. Weapons only look for entities with `EnemyHp`, so an enemy without it cannot be hit. Every enemy has `EnemyHp`, also one that no current weapon can hurt.
4. **Reactions.** Each enemy has a `Reactions` table: how it reacts to each weapon energy (see "Reactions to weapon energy", `adr-lw43`).
5. **Hurt.** An enemy with its own hurt frames carries `HurtFrames` and plays its `Hurt` tag. Every other enemy flashes `#993D41` for 0.12 s (`HURT_COLOUR`, `HURT_FLASH_SECS` in `enemy_hp.rs`).
6. **Damage to the player.** An enemy sends `DamagePlayer { amount }` (`hp.rs`). The player has no invincibility frames: every hit counts, also two in the same frame (b15).
7. **No friendly fire.** An enemy attack hits only the player (b16, `adr-lw42`). The player's weapons affect an enemy shot only as its reaction table says (`adr-lw43`). An enemy shot uses `Projectile::without_targets()`. A reader of `ControlledTouched` filters on its own marker (`With<Ghost>`, `With<VolcanoShot>`).
8. **Enemy and enemy shot** (`adr-lw44`). An enemy is a character. An enemy shot is a projectile an enemy launches, and carries `EnemyShot`. A shot has hit points and a reaction table like an enemy, so weapons touch it as its table says, but a weapon that chooses a target (the fireball) never chooses a shot.

## Values

**The values are placeholders until each enemy is tuned with the user.** The player walks at 300 px/s and has 3 HP (`adr-lw28`).

| Enemy | Legend | HP | Moves | Attack | Damage to the player | Hurt | Status |
|---|---|---|---|---|---|---|---|
| Ghost | `!` | 100 (`HIT_POINTS`) | 90 px/s (0.3 × the player), kit strategy from the legend | Touch: plays `Attack`, then dies | 1 HP per touch, once | Own `Hurt` frames | In the game since 2026-11-09 |
| Volcano | `^` | 300 (`HIT_POINTS`, placeholder: no weapon damages it yet) | Does not move | Two rolling shots per `Attack` loop | 1 HP per shot that touches | Own `Hurt` frames, played on an interrupt | Shots since 2026-11-23, weapon reactions since 2026-11-25 |

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
- Weapons see a shot where it is drawn, also in the air (`adr-x54`), so a high shot can pass over a weapon's edge. A shot has 50 hit points (`SHOT_HIT_POINTS`, placeholder). Red passes through it. Blue pushes it 8 times as far as the weapon pushes (`SHOT_BLUE_PUSH`) and does not hurt it. The push also turns it (`Deflects`, `weapons::damage::deflected`): its new direction is its own step plus the push, so a shot that meets a weapon at an angle glances off along its edge, and one that meets it head on keeps its direction and holds, or is carried back.

**Body.** Drawn at 2x (128x128, `adr-a11`): about 2 cells wide, and it reaches into the row above. The base of the cone (art rows 50 to 62, x 11 to 53, grown sideways by the player's half width) is a `Blocker`. The player walks up to it and behind it. Only the layers `base`, `rim` and `lava` are drawn (`LAYERS`). `test_bg` is never drawn.

**Placement.** Give it about 5 cells of open ground. Blocked areas and the map edge band remove shots early. The bottom rows of a map can be under the HUD.

**Weapons** (`adr-lw43`). Red touches it with no effect: the fire ring touches it, and the fireball targets it like any enemy and explodes on it, but deals nothing (`adr-lw44`). Blue interrupts it on every hit: it plays `Hurt` once (4 frames of 50 ms), then `Attack` from frame 1, and it launches nothing while `Hurt` plays. An interrupt while `Hurt` plays does not start `Hurt` again. Nothing pushes it.

**Hit box** (`HIT_BOX`). The drawn cone (top at art row 20) and the blocked base, 2 px further all round. The ice ring stops at the base, so the hit box reaches past it, and the ring touches the volcano from every side.

**Tuning note** (2026-11-25, scripted run in map 3, tornado cast next to the cone). The tornado hits every 0.5 s and `Hurt` then lasted about 0.4 s, so `Attack` never got back to frame 7: the volcano launched nothing for the whole 5.5 s of the tornado. After a manual test (2026-11-26, `Hurt` now 50 ms a frame) the user kept this as rule b17 (`balance.md`).

**Not needed yet** (the user, 2026-11-26): how it dies. No current weapon can kill it, and it has no `Die` tag. Decide it with the first weapon that damages it, together with how such a hit shows: it has no `HurtFrames`, so a damaging hit flashes the colour.

## Reactions to weapon energy

Each enemy kind reacts to each weapon energy in its own way (`adr-lw43`). The energy decides, not the weapon: every red weapon is treated the same, and so is every blue weapon.

| Reaction | Effect |
|---|---|
| `PassThrough` | The weapons of that energy do not see the enemy. No contact, no damage, no push. A fireball does not target it and does not explode on it |
| `Hit { damage, push, interrupt }` | The weapon touches the enemy. `damage` and `push` are shares of the weapon's own damage and push: `1.0` is all of it, `0.0` none, more than `1.0` is stronger. `interrupt` sends `EnemyInterrupted` on every hit of that energy, and the enemy's plugin decides what that does |

A `Hit` with damage 0, push 0 and no interrupt still touches: a fireball explodes on it, with no effect on that enemy.

How often an enemy is interrupted, and how far it is pushed, follow from the weapon's ticks and pushes and from the shares. They are tuning values, not rules.

| Enemy | Red | Blue | Violet | Black | Status |
|---|---|---|---|---|---|
| Ghost | Hit: damage 1, push 1 | Hit: damage 1, push 1 | Hit: damage 1, push 1 | Hit: damage 1, push 1 | In the game |
| Volcano | Hit: damage 0, push 0 (touched, no effect) | Hit: damage 0, push 0, **interrupt** (`Hurt`, then `Attack` from frame 1) | Hit: damage 1, push 0 (placeholder) | Hit: damage 1, push 0 (placeholder) | In the game |
| Volcano shot | `PassThrough` | Hit: damage 0, push 8 (`SHOT_BLUE_PUSH`, a tuning value); a push also turns it, so it glances off (`Deflects`) | Hit: damage 1, push 1 (placeholder) | Hit: damage 1, push 1 (placeholder) | In the game |

**Tuning target, volcano shot:** under the tornado, the shot holds at the outer rim while the player stands still (some jitter is fine), and the tornado carries it while the player walks.

**Tuning note** (2026-11-25, scripted runs in map 3, seed 7, the player standing north of the volcano, a shot coming head on). With `SHOT_BLUE_PUSH` 8, the tornado's bump is also multiplied: 40 px becomes 320 px, and it threw the shot 404 px back past the volcano. In an experiment with no bump on a body the tick does not damage, the same shot held at the rim until its path ended (pushed 172 px, it stopped 8 px outside the rim and glanced a little east). Decided by the user the same day: only a tick that damages bumps (`weapons.md`, "Tornado"). With that, the head-on shot held at the rim of a standing player until its path ended, and when the player walked towards it for 0.4 s the tornado carried it 226 px. The tuning target is met at 8. The ice ring scales its push to its edge by the same 8, so it throws a shot far past its edge; the user keeps this (2026-11-26). Not checked yet: a glance at a wide angle (the scripted runs gave only a slight one).

**Code.** Each enemy's table is the constant `REACTIONS` in its own file, next to its hit points. An enemy spawned without a table reacts in full to every energy (`Reactions::FULL`).

## Stun and interrupt

Two different things. Do not use one for the other.

| | Stun | Interrupt |
|---|---|---|
| **Type** | `Stunned`, a component in the kit (`game_kit::movement`) | `EnemyInterrupted`, a message in Long Way (`reactions.rs`). The kit has no interrupt |
| **What it is** | A state that lasts | One event for each hit |
| **Length** | Until the code that added it removes it | None |
| **Effect** | Set by the kit: the entity does not walk, and its sprite animation stops on its current frame. It keeps its goal and walks on when the stun ends. The ghost also does not attack while stunned | None of its own. The enemy's plugin decides what it does. The volcano plays `Hurt`, then starts `Attack` again from frame 1 |
| **Cause** | A rule of one weapon: the ice ring stuns an enemy that its push pins against a feature or the map edge, until the ring ends (`adr-lw35`, `weapons.md`) | The enemy's reaction table: every hit from an energy with `interrupt` (`adr-lw43`) |
| **Who chooses it** | The weapon | The enemy kind |

- An interrupt cannot be built from a stun: a stun freezes the animation, and an interrupt usually plays one.
- An interrupt handler may add a stun as part of its effect, for an enemy that should stop walking when it is hit.
- An enemy whose push share is 0 is never stunned by the ice ring, because the ring never pushes it and so never pins it.

## Against each weapon

Weapon values are in `weapons.md`. The balance rules in brackets keep these results true.

| Weapon | Ghost (100 HP) | Volcano |
|---|---|---|
| Fire ring | Dies on the first tick, 100 (b1) | Touched, no effect |
| Ice ring | Pushed to the edge for 50, 10 more if it walks in. One ring never kills (b3). A pinned ghost is stunned | Not moved and never stunned. Interrupted on every hit |
| Tornado | 13 per tick, bumped back 40 px on every second tick. Dies after more than 60 % of the tornado's time in contact (b5). Gets through to a player who stands still (b7, b9) | Not moved. Interrupted on every tick |
| Fireball | A direct hit (150) kills (b10). The blast (100) kills (b11) | The target when it is the nearest enemy (`adr-lw44`). Explodes on it with no effect |

## Add a new enemy kind

1. Ask the user for the design: movement, attack, damage, HP, how it shows a hit and how it dies.
2. Add `Kind::Spawn<Enemy>` and the kind name in `map.rs`, and a legend character from the enemy set (`adr-x37`). Write the key in quotes.
3. Spawn it from the map load hook. Do not add a `MapId` by hand (`adr-x58`).
4. Give it `EnemyHp::full(<its own HP>)`, also when no current weapon can hurt it, so a later weapon can. Do not change the ghost's HP or a weapon's damage to suit it (b1, b2).
5. Write its `REACTIONS` table, one reaction per energy, and add its row to "Reactions to weapon energy". If it interrupts, handle `EnemyInterrupted` in its plugin.
6. Hurt: add `HurtFrames` and a `Hurt` tag, or use the colour flash.
7. Its attacks hit only the player (b16): shots use `Projectile::without_targets()`, and touch readers filter on its own marker. Each shot carries `EnemyShot`, its own hit points and reaction table (`adr-lw44`).
8. Check the "by hand" rules in `balance.md`: b2 (fire ring), b13 (fireball direct hit) and b16 (friendly fire).
9. Add a row to "Values" and a column to "Against each weapon" here, its marker to `maps.md` and its tags to `art-tags.md`.
