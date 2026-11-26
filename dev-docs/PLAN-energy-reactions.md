# Plan: enemy reactions to weapon energy

Goal: every enemy kind has a table of how it reacts to each weapon energy (`adr-lw43`). First the docs and the ghost (no change in play), then the volcano and its shot.

The user's decisions (2026-11-25):
- The level is the weapon's energy, not the weapon. A weapon's archetype, and so its effect on an enemy, comes from its energy colour.
- Each energy gives a reaction: **pass through**, or **hit** with a damage share, a push share and an interrupt flag.
- The volcano and its shot both get `EnemyHp`, so a later violet or black weapon can hurt them. The current weapons do not.
- How often the volcano is interrupted, and how the shot holds at the tornado's rim, come from tuning values. No hard-coded rule decides them.

## Step 1. Docs (done 2026-11-25)

- `adr-lw43` in `games/long-way/docs/adr.md`.
- "Reactions to weapon energy" in `games/long-way/docs/enemies.md`, with the table per enemy and the planned volcano rows.
- `weapons.md`: a hit goes through the target's reaction.

## Step 2. The ghost (done 2026-11-25)

- `reactions.rs`: `Reaction` (`PassThrough` or `Hit { damage, push, interrupt }`), `Reactions` (one per energy, a component), the `EnemyInterrupted` message. `EnemyHp` requires `Reactions`, default `FULL`, so no enemy is hidden from weapons by mistake.
- `WeaponKind::energy()`, tested against `weapons_for`.
- `EnemyBody::hit_by(energy)`: `None` for a dead enemy or one that lets the energy pass through, else the body and its `Hit`. Every weapon uses it in place of `living_rect`, and scales its push by `push` before it forecasts the push (ice ring pin, tornado depth).
- `apply_hits` scales the damage by `damage` and sends `EnemyInterrupted` for each hit from an energy with `interrupt`.
- The fireball marks only enemies that red does not pass through as `ProjectileTarget`.
- The ghost spawns with `Reactions::FULL`. Play does not change: b1 to b14 still pass.
- Tests: the reaction lookup, `apply_hits` scaling and the interrupt message, a pass-through body is not hit by a ring, the push share reaches the push.

## Step 3. The volcano (done 2026-11-25)

Done: `EnemyHp` 300 (placeholder), `REACTIONS`, a `HIT_BOX` over the cone and the base with a 2 px margin (the ice ring stops at the base), `Hurt` on an interrupt then `Attack` from frame 0. The fireball first did not choose a target that red does not damage (Claude's default). After a manual test the user changed it (2026-11-26, `adr-lw44`): the fireball targets the nearest enemy whatever it can damage, and never an enemy shot (`EnemyShot`). Fixed in the real game: the animation crate moves to the new tag one frame late, so both shots launched when `Attack` started again; `finish_hurt` now sets frame 0. Scripted run in map 3: the tornado interrupts the volcano on every tick and it launches nothing for the tornado's 5.5 s; the fire ring touches it 6 times with no effect. Not seen: the ice ring and the fireball on the volcano (they need a click in the drawer, which the script cannot do). Still open: its death.

The plan as written:


- `EnemyHp` (placeholder value), a `HitBox` on the cone, `HurtFrames`.
- Reactions: red `Hit { damage: 0, push: 0, interrupt: false }` (touched, no effect: a fireball explodes on it), blue `Hit { damage: 0, push: 0, interrupt: true }`, violet and black `FULL`.
- On `EnemyInterrupted`: play `Hurt` once, then `Attack` from its first frame. No shots during `Hurt`.
- On `EnemyDied`: a death to be designed with the user (no `Die` tag yet).
- Check: the ice ring stops at blocked areas, and the volcano's base is one. Make sure the ring still reaches the cone.
- Open: should the fireball choose the volcano as its target, when red does no damage to it? Ask the user.
- `art-tags.md`: the `Hurt` tag.

## Step 4. The volcano shot (done 2026-11-25)

Built: `EnemyHp` 50 (placeholder), `SHOT_REACTIONS` (red passes through, blue push 8, `SHOT_BLUE_PUSH`), `Deflects` and `deflected` in `apply_hits`, b16 and `adr-lw42` changed. Scripted runs in map 3 (seed 7, player north of the volcano, a shot head on): the tornado's bump is multiplied by the push share too (40 px to 320 px) and threw the shot 404 px back. In an experiment with no bump for a body the tick does not damage, the shot held at the rim until its path ended. The user chose: only a tick that damages bumps. Then, standing: the head-on shot held at the rim until its path ended; walking towards it for 0.4 s: the tornado carried it 226 px. Not seen: the ice ring on a shot (needs a click in the drawer), and a glance at a wide angle (seed 7 gave only a slight one).

The plan as written:


- `EnemyHp` (placeholder), its `HitBox` is the ball.
- Reactions: red `PassThrough`, blue `Hit { damage: 0, push: <tuned>, interrupt: false }`, violet and black `FULL`.
- A push moves the shot through the collision world. Its path goes on from the new place (the kit path moves by the change of the path in each frame).
- **Glance off** (the user, 2026-11-25). A pushed shot also turns: its path direction becomes the direction of its own step this frame plus the push. When the push cancels the step (the sum is zero or points back), the direction stays, so a shot that meets the tornado head on holds at the rim and jitters. While it touches the rim it turns a little each frame, and it leaves along the tangent. A `Deflects` marker on the shot asks for this in `apply_hits`. The tornado's code does not change. Rejected: flow around (the shot keeps its first direction, so the tornado seems to do nothing) and bounce (a reflection that fights the head-on hold).
- **Collision in the air** (the user, 2026-11-25): the shot collides with weapons where it is drawn, also during its arc (`adr-x54`). A high shot can pass over the tornado's edge or meet it late. This is accepted.
- No physics crate (`adr-e10`): one body against one circle is a collision response, like the player's sliding. The shot's bounce is a fixed height curve of the kit path, not physics.
- Change b16 with the user: blue weapons push enemy shots.
- Tuning target: the shot holds at the tornado's rim when the player stands still and is carried when the player walks.

## Step 5. Tuning with the user

Manual test by the user (2026-11-26, map 3, `run --script`): the fire ring and the ice ring behave as intended, including a shot that hopped over the ice ring (accepted: weapons see a shot where it is drawn). The fireball did not target the volcano; changed the same day (`adr-lw44`). The user then made the volcano's `Hurt` frames 50 ms (looks better) and tried `Attack` frames of 45 ms (the volcano fired too often). Kept: `Attack` 100 ms, `Hurt` 50 ms, and the volcano does not fire while the tornado touches it, recorded as b17 (`balance.md`).


- Interrupt rate of the volcano under the tornado and the ice ring.
- The shot's push share against the tornado and the ice ring.
- Script runs with `--debug` for each case, then a play test by the user.

## Step 6. Migrate and delete this plan

- Move lasting content to `enemies.md`, `weapons.md` and `balance.md`. Add `concepts.md` or `notes.md` entries if a new Bevy pattern is used. Delete this file.
- Commit steps on the user's confirmation.
