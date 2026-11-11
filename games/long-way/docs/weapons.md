# Weapons

Design: the user (2026-11-11). Decisions: `adr-lw32`. Plan: `dev-docs/PLAN-weapons.md`. Code: `games/long-way/src/weapons/`, `enemy_hp.rs`.

## How a weapon works

1. The player clicks an energy icon, then a weapon button (`weapon_menu.rs`, `adr-lw31`). That sends `WeaponChosen`.
2. `weapons/mod.rs` turns it into `CastWeapon { weapon, origin, facing }`: the player's feet and the direction the player last moved in.
3. The weapon's gameplay system reads `CastWeapon`, owns the weapon's state and sends `WeaponHit { target, weapon, amount, push }` for each enemy it touches. Its visuals read the same state.
4. `weapons/damage.rs` turns a hit into `DamageEnemy` (`enemy_hp.rs`) and moves the enemy by `push` through the collision world, inside the map.
5. At 0 hit points an enemy sends `EnemyDied`. The ghost plays `Die` and is removed, and does not hurt the player.

## Values

All in `weapons/values.rs`, one row per weapon. **The numbers are placeholders until each weapon has been tuned with the user** (the spike gave looks and sizes, not damage).

| Weapon | Damage | Pushback | Tick | Glow | Status |
|---|---|---|---|---|---|
| Fire ring | 40 per second | at most 64 px | 0.5 s | on | built (W1), values not tuned yet |
| Ice ring | 20 per second | to the edge | 0.5 s | off | not built |
| Fire wall | 40 per second | none | 0.5 s | on | not built |
| Ice wall | 20 per second | none | 0.5 s | off | not built |
| Whirlpool | 20 per second | at most 96 px | 0.5 s | off | not built |
| Fireball | 40 once | none | | on | not built |

- **Damage per second** is dealt in ticks. The first frame in the effect hits at once, then one tick every `tick` seconds, each `dps * tick`, so the damage averages `dps`.
- **Long frame (D14 in `PLAN-weapons.md`):** a frame longer than the tick deals one tick, and the rhythm starts again from it. Missed ticks are not dealt later.
- **Dead enemies (D15 in `PLAN-weapons.md`):** every weapon skips an enemy whose `EnemyHp` is 0, even while it plays `Die` and still has its `HitBox`. The fire ring does this.
- **Pushback:** `Pixels(n)` moves a target at most `n` pixels in all per cast, away from the centre. `ToEdge` carries it to the weapon's final radius or distance. A tree, a rock, a wall or the map edge stops a push, and the target keeps taking damage there.
- **Enemy hit points:** ghost 100 (`ghost.rs`, `HIT_POINTS`).
- **Hurt look:** an enemy that has its own hurt frames (`HurtFrames`, the ghost's `Hurt` tag) plays them once when it survives a hit. Any other enemy is tinted `#993D41` for 0.12 s.

## Fire ring (W1)

Files: `weapons/ring.rs`, shape numbers in `weapons/values.rs` (`RING`), effects `assets/effects/fire_ring_front.particle.ron` and `fire_ring_burn.particle.ron`, art tag `fire_ring` of `art/weapons/fireball_and_rings.aseprite`.

- The ring grows from the middle of the player's body to a radius of 3 cells (192 px, so **6 cells across**, the user's choice) in 0.767 s, eased out (fast first, slowing to a stop). It then burns for 2 s and goes. It stays where it was cast.
- A body is hit when its `HitBox` touches the damaging band. While the ring grows the band reaches from the radius 0.12 s ago to the radius now (the trail of the 40 moving front emitters). While it burns, the band is 24 px each side of the full radius (40 burners).
- A body in the band is hurt at once, then every 0.5 s while it stays (`Contact`). A body that walks into the burn band later is hurt too.
- While the front grows, it carries a hit body outward at the front's speed, until the pushback limit (64 px) is used up. After that the body can walk back in.
- **Features (D10, `adr-lw33`):** the ring passes over trees, rocks, markers and scenery, and they do not react to it. A pushed body stops at them (the collision world).
- Checked with a scripted run (`teleport:-608:-150` in map 1, the ghost 100 px above): the ghost loses 20 per tick (100, 80, 60, 40 ...) and dies at 0 or walks out of the band. The ring was 6.1 cells across in the screenshot.
- Frame time with the ring cast every 0.7 s (up to four rings at once, vsync off): 2.7 to 3.1 ms per frame on native, idle 2.7 to 3.9 ms. No measurable cost. The web was not measured.

## Aim rules

| Weapon | Anchor | Aim |
|---|---|---|
| Fire ring, ice ring | The middle of the player's body at the cast. Does not follow the player. | Full circle |
| Fire wall, ice wall | In front of the player | The way the player faces sets the orientation and the travel direction |
| Whirlpool | The player, and it follows | Surrounds the player |
| Fireball | The player | The nearest visible enemy |

## Reporting a problem from a manual test

Run with the log on, and copy the lines from the cast to the end:

`RUST_LOG="info,wgpu=error,naga=warn" ./dev-scripts/rust.sh run long-way --debug`

The lines (all `info`, one per event, `adr-lw32`):

| Line | Means |
|---|---|
| `weapons: cast <weapon> at <feet>, facing (x, y)` | The drawer button sent the cast |
| `ring: started at (x, y), max radius r` | The ring exists. It is centred on the player's body |
| `enemy: <id> lost 20.0, 80.0 of 100.0 left, at (x, y)` | A damage tick, and where the enemy stood |
| `ghost: <id> hurt, plays Hurt` | The ghost survived the hit |
| `enemy: <id> died` and `ghost: <id> was killed, plays Die` | HP reached 0 and `Die` starts |
| `ghost: <id> removed after Die` | `Die` ended and the ghost is gone. **A `died` without a `removed` is a bug** |
| `ghost: <id> touched the player and attacks` and `ghost: <id> attack landed ...` | The ghost reached the player |
| `ring: ended at age 2.77 s, touched 1: [<id> 5 ticks, pushed 62 px]` | The ring is over: ticks dealt and pixels pushed per enemy |

For the kit's movement and collision lines add `long_way=debug,game_kit=debug` to `RUST_LOG`. Say where the player stood and what the ghost did on screen.

## Tuning

Change a number in `values.rs`, run the game or a scripted run (`dev-feedback.md`), and read the `enemy:` log lines (see above). For example:

`LOG_FILTER="weapons:|enemy:" ./dev-scripts/rust.sh script long-way "ui_click:66:588;wait:0.5;ui_click:136:465;wait:3;quit" --debug`

`ui_click:66:588` is the red energy icon and `ui_click:136:465` the first button of its drawer.
