# PLAN: the volcano launches a rolling projectile (Long Way)

The user's brief (2026-11-23): the volcano launches one projectile per loop of its `Attack` tag. The projectile is drawn in its own `.aseprite` file.

- It launches from the mouth of the volcano at frame 7.
- It launches in a random direction, then bounces and rolls in a straight line for a set distance.
- Easing on the roll speed and the bounce height makes it slow down smoothly.
- If it hits the player, the player loses 1 HP and the projectile disappears.
- When it stops, it disappears.

## Status (2026-11-23): resume here

Skeleton only. Answer the open questions first (Q1 to Q12), move the answers to the decisions table, then fill in the steps.

## What is already in place

- `games/long-way/src/volcano.rs`: `spawn_volcanoes` places a `Volcano` from a `spawn_volcano` marker (legend `^`). It loops `Attack` (10 frames, 100 ms each, so one loop per second), drawn at `SCALE` 2 with only the layers `base`, `rim` and `lava`. Its base is a `Blocker` with `SortAtBlocker` (`adr-x53`). It cannot be hit yet. Map 3 has one at cell 4,3.
- `art/characters/volcano.aseprite`: 64x64 file, drawn at 2x (`adr-a11`). Frames 9 and 10 touch the top edge of the canvas.
- Kit `crates/game-kit/src/projectile.rs` (`adr-x21`, `dev-docs/extensions/projectiles.md`): a projectile is a ground point that moves in a straight line at a fixed speed. It is removed when it hits a blocked area or the map edge (`ProjectileHit`). Its sprite is a child lifted by a fixed `height`. It hits a `ProjectileTarget` with a `HitBox`. It has no arc, bounce or easing, and it cannot hit the player.
- Long Way's fireball (`weapons/fireball.rs`) uses the kit projectile for the player's shots at enemies.
- `hp.rs`: the `DamagePlayer` message takes HP from the player. The ghost uses it (`ghost.rs`).
- The player has a `HitBox` (`player.rs`, `HIT_BOX`).
- Easing: `game_kit::area::ease_out` (quadratic). Bevy also has `EaseFunction` (not used in the project yet).
- Animation frame: `AnimationState { current_frame, .. }` (`dev-docs/bevy/notes.md`, bevy_aseprite_ultra). It gives the frame index, so "frame 7" can be found each loop.
- `./dev-scripts/shot.sh long-way <name> "<commands>" map=3 cx= cy= w= h= scale=` for zoomed screenshots of map 3.

## Open questions

| ID | Question | Notes |
|---|---|---|
| Q1 | Projectile art: file name, size, tags (for example a spin while it rolls), and drawn at what scale? Do you draw it, or do I make a `_placeholder` first? | Suggest `art/characters/volcano_projectile.aseprite`, drawn at 2x like the volcano (`adr-a11`). |
| Q2 | Where is the mouth? | Measure it in the art (the crater row and centre at frame 7) with a small Aseprite script, as was done for the base. |
| Q3 | The path: does it first fly in an arc from the mouth to the ground, and then bounce lower each time while it rolls? How many bounces, and does the last part roll flat? | Defines the height curve. |
| Q4 | "Set distance": counted from the volcano or from where it first lands? How far (in cells)? How long does the whole path take? | |
| Q5 | Random direction: any angle (360°), or only some (for example never straight up, where it is hidden behind the volcano)? | |
| Q6 | What happens at a tree, rock, marker or the map edge: it stops and disappears, it bounces off, or it passes over while in the air? | The kit projectile is removed on a hit. |
| Q7 | When does it hit the player: only while near the ground, or at any height? Can it pass over the player at the top of a bounce? | Uses the kit's hit height test (`Projectile::height`). |
| Q8 | After a hit, does the player get a short time without damage, so two projectiles in a row cannot take 2 HP at once? | The ghost has no such rule yet. |
| Q9 | Does it hurt enemies (ghosts) too? Can the player's weapons destroy it? | |
| Q10 | One launch per second can give several projectiles at once. Is there a limit? | |
| Q11 | A shadow on the ground under it, so its height can be read? | Common in top-down games; a separate sprite. |
| Q12 | Kit or game code? The bounce and roll could extend the kit projectile, or stay in Long Way until a second game needs it (`adr-x12`). | Recommendation to follow when the answers above are known. |

## Decisions

| ID | Topic | Decision |
|---|---|---|
| | | |

## Steps (draft, to fill in)

### V1. Projectile art

The `.aseprite` file (Q1). Check it with `./dev-scripts/aseprite.sh check <file> palette=games/long-way/assets/art/game.aseprite`.

### V2. The path as plain functions

Ground distance and height over time, with the easing (Q3, Q4). Plain Rust with no Bevy, unit tested: start, end, the slow-down and each bounce.

### V3. Launch at frame 7

Spawn one projectile each time the volcano's `Attack` enters frame 7, at the mouth (Q2), in a random direction (Q5). Test: one launch per loop, none on other frames.

### V4. Move, draw and stop

Move the ground point along the path, lift the sprite by the height, sort it by its ground point. Remove it when the path ends, and at blocked areas and the map edge as decided (Q6). Shadow if decided (Q11).

### V5. Hit the player

1 HP through `DamagePlayer`, then remove the projectile (Q7, Q8). Test a hit and a miss at the top of a bounce.

### V6. Scripted run

`shot.sh` on map 3: a launch, a projectile in flight, one that stops, and one that hits the player. Say which parts were not seen on screen.

### V7. Gates

`./dev-scripts/rust.sh lint`, `test long-way` (and `test game-kit` if the kit changes), `web`.

### V8. Migrate lasting content, then delete this plan

- `games/long-way/docs/maps.md`: the volcano entry.
- `games/long-way/docs/art-pipeline.md`: the projectile file.
- `games/long-way/docs/source-layout.md`: the new code.
- `games/long-way/docs/adr.md`: the decisions that hold (an `adr-lw` entry).
- `dev-docs/extensions/projectiles.md` and an `adr-x` entry if the kit changes.
- `docs/README.md` if the player needs to know something new.

### V9. Commit

Ask first.
