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

## Step 3. The volcano

- `EnemyHp` (placeholder value), a `HitBox` on the cone, `HurtFrames`.
- Reactions: red `Hit { damage: 0, push: 0, interrupt: false }` (touched, no effect: a fireball explodes on it), blue `Hit { damage: 0, push: 0, interrupt: true }`, violet and black `FULL`.
- On `EnemyInterrupted`: play `Hurt` once, then `Attack` from its first frame. No shots during `Hurt`.
- On `EnemyDied`: a death to be designed with the user (no `Die` tag yet).
- Check: the ice ring stops at blocked areas, and the volcano's base is one. Make sure the ring still reaches the cone.
- Open: should the fireball choose the volcano as its target, when red does no damage to it? Ask the user.
- `art-tags.md`: the `Hurt` tag.

## Step 4. The volcano shot

- `EnemyHp` (placeholder), its `HitBox` is the ball.
- Reactions: red `PassThrough`, blue `Hit { damage: 0, push: <tuned>, interrupt: false }`, violet and black `FULL`.
- A push moves the shot through the collision world. Its path goes on from the new place (the kit path moves by the change of the path in each frame).
- Change b16 with the user: blue weapons push enemy shots.
- Tuning target: the shot holds at the tornado's rim when the player stands still and is carried when the player walks.

## Step 5. Tuning with the user

- Interrupt rate of the volcano under the tornado and the ice ring.
- The shot's push share against the tornado and the ice ring.
- Script runs with `--debug` for each case, then a play test by the user.

## Step 6. Migrate and delete this plan

- Move lasting content to `enemies.md`, `weapons.md` and `balance.md`. Add `concepts.md` or `notes.md` entries if a new Bevy pattern is used. Delete this file.
- Commit steps on the user's confirmation.
