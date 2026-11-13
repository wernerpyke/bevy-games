# Weapons

Design: the user (2026-11-11). Decisions: `adr-lw32`. Plan: `dev-docs/PLAN-weapons.md`. Code: `games/long-way/src/weapons/`, `enemy_hp.rs`.

## How a weapon works

1. The player loads a weapon once: a click on an energy gauge opens the drawer, and a click on a weapon there puts it in the active slot of its energy (`weapon_menu.rs`, `adr-lw31`, `adr-lw34`). Red starts with the fire ring.
2. A click on the active weapon button sends `ActiveWeaponClicked`. `weapons/mod.rs` turns it into `CastWeapon { weapon, origin, facing }`: the player's feet and the direction the player last moved in.
3. The weapon's gameplay system reads `CastWeapon`, owns the weapon's state and sends `WeaponHit { target, weapon, amount, push }` for each enemy it touches. Its visuals read the same state.
4. `weapons/damage.rs` turns a hit into `DamageEnemy` (`enemy_hp.rs`) and moves the enemy by `push` through the collision world, inside the map.
5. At 0 hit points an enemy sends `EnemyDied`. The ghost plays `Die` and is removed, and does not hurt the player.

## Values

All in `weapons/values.rs`, one row per weapon. **The numbers are placeholders until each weapon has been tuned with the user** (the spike gave looks and sizes, not damage). The values must keep the rules in `balance.md`.

| Weapon | Damage | Pushback | Tick | Glow | Status |
|---|---|---|---|---|---|
| Fire ring | 200 per second (100 per tick: the first tick kills a ghost, b1) | none | 0.5 s | on | **accepted** (W1, 2026-11-13) |
| Ice ring | 50 once when pushed, 10 once per cast when it walks in (b3) | to the edge, then a barrier | | off | **accepted** (W2, 2026-11-13) |
| Fire wall | 40 per second | none | 0.5 s | on | not built |
| Ice wall | 20 per second | none | 0.5 s | off | not built |
| Tornado | 26 per second (13 per tick of contact, b5) | out at 20 px/s, plus a 40 px bump on every second tick; the ghost still gains (b7 to b9) | 0.5 s | off | **accepted** (W3, 2026-11-13) |
| Fireball | 40 once | none | | on | not built |

- **Damage per second** is dealt in ticks. The first frame in the effect hits at once, then one tick every `tick` seconds, each `dps * tick`, so the damage averages `dps`.
- **Long frame (D14 in `PLAN-weapons.md`):** a frame longer than the tick deals one tick, and the rhythm starts again from it. Missed ticks are not dealt later.
- **Dead enemies (D15 in `PLAN-weapons.md`):** every weapon skips an enemy whose `EnemyHp` is 0, even while it plays `Die` and still has its `HitBox`. The fire ring does this.
- **Pushback:** `Pixels(n)` moves a target at most `n` pixels in all per cast, away from the centre. `ToEdge` carries it to the weapon's final radius or distance. A tree, a rock, a wall or the map edge stops a push, and the target keeps taking damage there.
- **Enemy hit points:** ghost 100 (`ghost.rs`, `HIT_POINTS`).
- **Hurt look:** an enemy that has its own hurt frames (`HurtFrames`, the ghost's `Hurt` tag) plays them once when it survives a hit. Any other enemy is tinted `#993D41` for 0.12 s.

## Fire ring (W1)

**Intent and purpose (the user, 2026-11-13, in the user's words):**

> The intent with the fire ring is that it is a non-blocking attack which applies some pushback to enemies and which hurts enemies while they are in contact with it. Both the player and enemies can move through the fire ring. If the fire ring hits a blocking feature it passes through the feature to retain a full circle effect.

**Implementation note (2026-11-13):** the accepted fire ring does not push (`Pushback::None` in the fire ring row of `weapons/values.rs`), which differs from "applies some pushback" in the intent. The push was removed when the ring was tuned for b1: the first tick kills a ghost in the frame it is touched, so a push would never show. The ring code still supports pushback (`Pushback::Pixels(n)`: while the front grows, it carries a hit body outward at the front's speed, up to `n` pixels in all). When an enemy that survives a tick is added (b2), give the fire ring a small push by changing that one value.

Files: `weapons/ring.rs`, shape numbers in `weapons/values.rs` (`FIRE_RING`), effects `assets/effects/fire_ring_front.particle.ron` and `fire_ring_burn.particle.ron`, art tag `fire_ring` of `art/weapons/fireball_and_rings.aseprite`.

- The ring grows from the middle of the player's body to a radius of 2 cells (128 px, so **4 cells across**, the user's choice on 2026-11-13; it was 6 cells across) in 0.767 s, eased out (fast first, slowing to a stop). It then burns for 2 s and goes. It stays where it was cast.
- A body is hit when its `HitBox` touches the damaging band. While the ring grows the band reaches from the radius 0.12 s ago to the radius now (the trail of the 40 moving front emitters). While it burns, the band is 24 px each side of the full radius (40 burners).
- A body in the band is hurt at once, then every 0.5 s while it stays (`Contact`). A body that walks into the burn band later is hurt too.
- The first tick (100) kills a ghost, so a ghost the ring touches dies at once, wherever it stands (balance rule b1, `balance.md`). The fire ring does not push. The ring code still supports pushback for other rings: while the front grows, it carries a hit body outward at the front's speed.
- **Features (D10, `adr-lw33`):** the ring passes over trees, rocks, markers and scenery, and they do not react to it. A pushed body stops at them (the collision world).
- Checked with a scripted run (`teleport:-608:-150` in map 1, the ghost 100 px above): the ghost lost 20 per tick (100, 80, 60, 40 ...) and died at 0 or walked out of the band (before b1). The ring was 6.1 cells across in the screenshot (before the change to 4 cells across).
- Frame time with the ring cast every 0.7 s (up to four rings at once, vsync off): 2.7 to 3.1 ms per frame on native, idle 2.7 to 3.9 ms. No measurable cost. The web was not measured. Measured with the 6-cell ring. The 4-cell ring has the same number of emitters, so the cost is the same or lower.
- **Tuning note (accepted by the user on 2026-11-13):** the radius went from 3 cells to 2 cells (4 cells across), after a manual test. The damage went from 40 to 200 per second (20 to 100 per tick) and the pushback from 64 px to none, so the first touch kills a ghost (balance rule b1, `balance.md`). With the old values, a ghost close to the player at the cast took one tick and walked back to attack. Grow time, burn time, band width and glow are the spike values and were not changed.

## Ice ring (W2)

**Intent and purpose (the user, 2026-11-13, in the user's words):**

> The intent with the ice ring is that it is a static shield which protects the player for the duration and does little damage to enemies. Once the ice ring is cast the player can move through it but enemies cannot. It also means that enemies are never trapped inside it, they are pushed out as the ring grows. If the ring hits a blocking feature or the map edge then the ring stops growing there but does not disappear. This means that the ring will not necessarily be a full circle. An enemy that is trapped between the ring and a blocking feature or map edge is stunned until the ring disappears.

Design: the user, 2026-11-13 (`adr-lw35`). Files: `weapons/ice_ring.rs` (reach, push, barrier, stun), `weapons/ring.rs` (growth and look, shared), shape numbers in `weapons/values.rs` (`ICE_RING`, `ICE_RAYS`), effects `assets/effects/ice_ring_front.particle.ron` and `ice_ring_burn.particle.ron` (from the spike), art tags `ice_ring1` to `ice_ring4` of `art/weapons/fireball_and_rings.aseprite`. Two emitters per position, each with one of the four shard sets.

- The ring grows from the middle of the player's body to a radius of 3 cells (192 px, 6 cells across) in 0.767 s, holds for 2 s and goes. It stays where it was cast.
- **Reach.** At the cast, 64 rays leave the centre. Each stops at the first blocking feature (the collision outlines) and at the map edge, and the ring stops with it. Near a tree the ring is not a full circle. An enemy behind a feature is not reached.
- **Push.** The growing front carries every enemy it reaches to the edge of the ring (its near side ends on the ring). The first push deals 50.
- **Barrier.** At full size an enemy that walks into the ring is moved back out in the same frame (the weapon systems run after the kit has moved the enemies) and takes 10, once per cast. The player passes freely.
- **Stun.** An enemy whose push is stopped by a feature or the map edge is `Stunned` (the kit marker): it stands still in its current frame, does not attack and takes no more damage from this ring. It walks again when the ring goes. An enemy pushed into open space is not stunned.
- **Balance:** one ring deals at most 50 + 10 = 60, so it never kills a ghost (b3). Two pushes deal 100.
- Checked with scripted runs in map 1: the ring stopped at the trees, a ghost was held at the edge and took 50 then 10, and a ghost under the top map edge was pinned, stunned and walked again after the ring. Not seen on screen: the stunned ghost's paused animation, a pin against a tree. Frame time not measured yet.
- **Tuning note (accepted by the user on 2026-11-13 after playing it):** no number moved. The start values hold: push 50, touch 10, hold 2 s, radius 3 cells, 64 rays. A stunned enemy does not attack, as built.

## Tornado (W3)

Renamed from the whirlpool on 2026-11-13 (the icon tag too). Design: `PLAN-weapons.md` W3. Balance rules b4 to b9 (`balance.md`).

**Intent and purpose (the user, 2026-11-13):**

> The intent with tornado is that its total duration is twice that of ice ring and its purpose is as a mobile shield which moves with the player.
> In terms of damage it does more damage than ice ring and can kill a ghost if it keeps in contact with a ghost for >60% of its duration.
> In terms of pushback, it continually pushes the ghost back but the ghost does incrementally make progress through it. The feel that I am trying to achieve with the pushback is that, if a player is static and just fires tornado then the ghost can reach the player. However, if the player is moving while tornado runs then the player remains safe.

Clarified by the user the same day: contact is total contact time, so a player can kill a ghost by holding the tornado against it. A ghost pushed into a tree or the map edge is not stunned: it keeps being pushed and hurt there. Walking straight at one ghost for about 2.4 s lets it through, and that is accepted. The look stays the spike's water spiral for now.

After the first play test (the user, 2026-11-13, with a marked screenshot): the push should sit where the dense droplets are. The edge starts at their inner edge and grows outward to 1.5 cells. A ghost walking in should be bumped back a couple of times (option B: a bigger bump on every second tick, rather than a small one on every tick), and the bump also works while the edge grows.

**Values (to tune):** a disc on the player's body whose edge grows evenly from 1 cell (64 px) to 1.5 cells (96 px) in 1.3 s (a droplet's life), then holds. 5.53 s (twice the ice ring). 26 per second in ticks of 13 every 0.5 s of contact (a ghost dies at 3.5 s of contact). A steady push of 20 px/s (a ghost walks at 90), and a bump of 40 px over 0.1 s on every second tick. Before the play test: a fixed radius of 2 cells and a push of 45 px/s, no bump.

Files: `weapons/tornado.rs` (gameplay and look), `weapons/values.rs` (`TORNADO`, `Pushback::Outward`), effect `assets/effects/tornado_orbit.particle.ron` (the spike's `whirl_orbit`), no sprite art.

- **Contact:** an enemy whose body overlaps the disc. Damage ticks through `Contact`, which only counts frames in contact, so the count is the total contact time.
- **Push:** each frame an enemy's depth in the disc may grow only by its own step since the last frame, less 20 px/s × the frame time, and not below 0. A deeper enemy is pushed out by the difference through the collision world. So a ghost walking in gains 70 px/s, a standing one is pushed out at 20 px/s, the growing edge carries what it meets (about 25 px/s more for 1.3 s), and the player's movement shoves an enemy ahead at the edge. An enemy that a tree or the map edge stops stays there, is not stunned and keeps taking ticks.
- **Bump:** on an enemy's 2nd, 4th, 6th... tick it is also shoved 40 px outward over 0.1 s, through the collision world. The shove finishes even when the enemy leaves the disc. Its hurt animation plays on the same tick.
- **Look:** 10 emitters on a 65 px circle around the centre send droplets along the tangent, and an attractor at the centre bends them outward, from the start of the edge to past its full radius (1.3 s life). The look did not change with the play test. The emitters follow the player. Droplets in flight do not, so the spiral trails behind a walking player.
- **Push edge** (the user, 2026-11-13): 48 dots of 1 px (`#EBF7FF`) on the edge of the disc grow with it, turn clockwise at 1 rad/s and move with the player, so a walking player sees where the push starts. They are sprites placed each frame, not particles: a particle in flight either trails behind or, moved with its emitter (Enoki's `relative_positioning`), smears along the emitter's turn. Values: `EDGE_DOTS`, `EDGE_SPIN`, `EDGE_COLOR` in `tornado.rs`.
- Checked with scripted runs in map 1, after the play test changes: with the player standing, a ghost first touched the edge 0.83 s after the cast. It made no progress across its 2nd and 4th ticks (the bumps), took 5 ticks (35 HP left), reached the player 2.3 s after its first contact, then attacked and died (the player lost 1 HP). At 0.6 s the edge dots sat on the dense droplet band. Before the changes: With the player walking, the ghost did not touch the tornado for about 4 s; it took 3 ticks when the player walked back towards it. Not seen yet: a ghost killed by holding the tornado against it, a ghost pinned against a tree. Frame time not measured.

## Aim rules

| Weapon | Anchor | Aim |
|---|---|---|
| Fire ring, ice ring | The middle of the player's body at the cast. Does not follow the player. | Full circle |
| Fire wall, ice wall | In front of the player | The way the player faces sets the orientation and the travel direction |
| Tornado | The player, and it follows | Surrounds the player |
| Fireball | The player | The nearest visible enemy |

## Reporting a problem from a manual test

Run with the log on, and copy the lines from the cast to the end:

`RUST_LOG="info,wgpu=error,naga=warn" ./dev-scripts/rust.sh run long-way --debug`

The lines (all `info`, one per event, `adr-lw32`):

| Line | Means |
|---|---|
| `weapons: cast <weapon> at <feet>, facing (x, y)` | The drawer button sent the cast |
| `ring: Fire ring started at (x, y), max radius r` | The ring exists (`Fire ring` or `Ice ring`). It is centred on the player's body |
| `enemy: <id> lost 20.0, 80.0 of 100.0 left, at (x, y)` | A damage tick, and where the enemy stood |
| `ghost: <id> hurt, plays Hurt` | The ghost survived the hit |
| `enemy: <id> died` and `ghost: <id> was killed, plays Die` | HP reached 0 and `Die` starts |
| `ghost: <id> removed after Die` | `Die` ended and the ghost is gone. **A `died` without a `removed` is a bug** |
| `ghost: <id> touched the player and attacks` and `ghost: <id> attack landed ...` | The ghost reached the player |
| `ring: Fire ring ended at age 2.77 s, touched 1: [<id> 5 ticks, pushed 62 px]` | The fire ring is over: ticks dealt and pixels pushed per enemy |
| `ice ring: <id> pushed, 50 damage` and `ice ring: <id> walked into the ring, 10 damage` | The ice ring's two kinds of damage, each once per enemy per cast |
| `ice ring: <id> pinned at (x, y), stunned until the ring goes` | A push was stopped by a feature or the map edge |
| `tornado: started at (x, y), edge 64 to 96 in 1.3 s, lasts d s` and `tornado: ended at age 5.55 s, touched 1: [<id> 6 ticks, pushed 125 px]` | The tornado: ticks dealt and pixels pushed per enemy. Each tick is also an `enemy:` line |
| `ring: Ice ring ended at age 2.77 s` and `ice ring: ended, touched 2: [<id> pushed 114 px, push damage, stunned; ...]` | The ice ring is over, and what it did to each enemy. Stunned enemies walk again |

For the kit's movement and collision lines add `long_way=debug,game_kit=debug` to `RUST_LOG`. Say where the player stood and what the ghost did on screen.

## Tuning

Change a number in `values.rs`, run the game or a scripted run (`dev-feedback.md`), and read the `enemy:` log lines (see above). For example:

`LOG_FILTER="weapons:|enemy:" ./dev-scripts/rust.sh script long-way "ui_click:65:552;wait:3;quit" --debug`

`ui_click:65:552` is the red active weapon button (it holds the fire ring at the start). The other buttons are at x 193, 321 and 449 (blue, violet, black), and the gauges below them at y 588. Coordinates are art pixels from the top left of the window (the bar starts at y 512).
