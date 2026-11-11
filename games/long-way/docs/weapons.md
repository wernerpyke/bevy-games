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
| Fire ring | 40 per second | at most 64 px | 0.5 s | on | not built |
| Ice ring | 20 per second | to the edge | 0.5 s | off | not built |
| Fire wall | 40 per second | none | 0.5 s | on | not built |
| Ice wall | 20 per second | none | 0.5 s | off | not built |
| Whirlpool | 20 per second | at most 96 px | 0.5 s | off | not built |
| Fireball | 40 once | none | | on | not built |

- **Damage per second** is dealt in ticks. The first frame in the effect hits at once, then one tick every `tick` seconds, each `dps * tick`, so the damage averages `dps`.
- **Pushback:** `Pixels(n)` moves a target at most `n` pixels in all per cast, away from the centre. `ToEdge` carries it to the weapon's final radius or distance. A tree, a rock, a wall or the map edge stops a push, and the target keeps taking damage there.
- **Enemy hit points:** ghost 100 (`ghost.rs`, `HIT_POINTS`).
- **Hurt look:** an enemy is tinted `#993D41` for 0.12 s after a hit, unless it has its own hurt frames (none has).

## Aim rules

| Weapon | Anchor | Aim |
|---|---|---|
| Fire ring, ice ring | The player's position at the cast. Does not follow the player. | Full circle |
| Fire wall, ice wall | In front of the player | The way the player faces sets the orientation and the travel direction |
| Whirlpool | The player, and it follows | Surrounds the player |
| Fireball | The player | The nearest visible enemy |

## Tuning

Change a number in `values.rs`, run the game or a scripted run (`dev-feedback.md`), and read the `enemy:` log lines: `enemy: <id> lost 20.0, 80.0 of 100.0 left`. For example:

`LOG_FILTER="weapons:|enemy:" ./dev-scripts/rust.sh script long-way "ui_click:66:588;wait:0.5;ui_click:136:465;wait:3;quit" --debug`

`ui_click:66:588` is the red energy icon and `ui_click:136:465` the first button of its drawer.
