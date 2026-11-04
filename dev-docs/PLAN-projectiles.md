# PLAN: projectile weapons in R-Town

Status: in progress. Decisions D1 to D5 are accepted as recommended. D6 (no despawn on map reload) was decided by the developer. Step 1 is done: `Message`, `add_message`, `MessageWriter`, `just_pressed` and `SystemSet` are verified in `bevy-src/` and recorded in `dev-docs/bevy/notes.md`. Step 2 is done: `projectile::step` and 10 unit tests in `crates/game-kit/src/projectile.rs`. Step 3 is done: `Projectile`, `ProjectileHit` (with `HitCause`) and `ProjectilePlugin` in the same file. The plugin is not added to a game yet (step 8). Step 4 is done: `x` in the script tool, documented in `script.rs` and `dev-feedback.md`. Step 5 is done: `games/r-town/src/input.rs` (`PlayerInput`, `InputSet`, `InputPlugin`) and `player.rs` reads it. `PlayerInput::fire` is set but not read yet. Step 6 is done: `Player::facing` (public, unit vector, starts right) and `next_facing` with 3 unit tests. Step 7 is done: `assets/art/features/projectiles.aseprite` (tag `proj_8`, 13x11) and its section in `art-tags.md`. The size next to the player is still to be judged on screen (step 9). Step 8 is done: `weapon.rs` (`WeaponPlugin`, fire on `PlayerInput::fire`), `WALK_SPEED` is public, and `main.rs` adds `ProjectilePlugin` and `WeaponPlugin`. Lint and the web check pass. A short scripted run with one shot shows no error. Step 9 is done (scripted runs and screenshots): a shot is removed at a lone tree (x 399, the trunk grown by 5), a pond (395), a grove (203), and at the right, top and left map edges and on a diagonal run to the right edge. 7 shots, 7 removal lines. The `LOG_FILTER` of `rust.sh script` hides the `projectile:` line unless `LOG_FILTER="projectile:|script:"` is set. Changed after step 9: the art is resized to 7x6 (about half of 13x11) and `ProjectileSprite` turns the sprite along the flight direction (kit, `projectile.rs`, 1 more unit test). Checked on screen for a shot up and a shot left-down. Not done on purpose: despawning flying projectiles on an F1 map reload (D6). Step 10 is done: lint, all tests (kit 135, long-way 28, r-town 16) and the web compile check pass.

## 1. Goal

The player fires a projectile with the X key.

- It leaves in the direction the player faces.
- It travels at twice the player's walk speed: 160 art px/s (walk speed is 80, `player.rs`).
- It disappears when it hits a blocked place (tree, water) or leaves the map.
- Art: `input/medieval-fantasy-1/items/projectile/8.png`, a pink fireball, resized to 7x6 (about half). The art points to the right and the sprite turns along the flight direction.

The projectile logic goes in `game-kit`. Input handling is prepared for a second device (mouse) without moving it to the kit yet.

## 2. Decisions needed (recommendation first)

| # | Question | Recommendation | Trade-off |
|---|---|---|---|
| D1 | Where does the projectile code live? | **Write it in `crates/game-kit/src/projectile.rs` from the start.** It has no game types and no assets. | The workspace rule says to move code to the kit when two games use it, and to ask first. Long Way has no weapons, so there is no second user yet. Writing it in the kit now saves a move later. The cost is a kit API designed with one user. |
| D2 | Input handling | **Add `games/r-town/src/input.rs` now. Move it to the kit when the mouse is added or Long Way uses it.** The game writes a `PlayerInput` resource (`move_dir`, `fire`). The player and the weapon read only this resource and never read `KeyCode`. | A kit input module now would have one user and a guessed API. A game-local module gives the same isolation. The later move is a file move. |
| D3 | What does "facing" mean? | **Player gets a `facing: Vec2`, the last non-zero move direction, 8 directions. Start value: right.** A diagonal walk gives a diagonal shot at the same speed (normalised). | The player only has `facing_left` today (flip). A new field is needed in any case. Four directions only would be simpler but would look wrong after a diagonal walk. |
| D4 | Which collision shape does the projectile use? | **Its ground point (feet point) against the existing `CollisionWorld` edges.** | The edges are the player's: areas are grown sideways by 5 px (`adr-rt5`, `adr-rt9`). A shot stops up to 5 px before a tree's side. A lone tree blocks only a 4 px band at its base, so a shot hits it only when its ground line crosses that band. This matches how the player meets trees. Exact shapes would need a second set of areas in the kit. Revisit after play testing. |
| D5 | Fire rate and range | **One shot per key press (`just_pressed`), no cooldown, no range limit.** | Simple. Cooldown and range are weapon values and can be added in a later plan. |
| D6 | Projectiles in flight when the map reloads (F1) | **Do nothing.** They keep flying against the new collision edges and are removed by the normal rules. | Decided by the developer: F1 is a development tool and a stray shot is harmless. Despawning them would need a hook in the map reload system. |

Steps below follow the recommendations. If an answer differs, the plan is changed before step 1.

## 3. Design

### 3.1 Kit: `crates/game-kit/src/projectile.rs`

- `ProjectileSprite` marker for the sprite child. The kit turns that child to point along the velocity (art drawn pointing right, along +x), so only the sprite turns and its lift above the ground stays. The root does not turn.
- `Projectile { velocity: Vec2 }` component, in art px/s. The entity's `Transform` is the **ground point**. The sprite is a child, lifted by a height, as for the player (`adr-rt6`). The root carries `DepthSort`, so a shot passes behind trees (`adr-x2`) and no z number is written (`adr-x9`).
- `ProjectilePlugin`: one system in `Update` that moves each projectile by `velocity * dt`. It reads `CollisionWorld` and `CameraBounds` (the map rectangle that the player is already clamped to).
- Plain function `step(old, velocity, dt, edges, bounds) -> Step`, so it is unit testable without Bevy:
  - `Step::Flying(new)`.
  - `Step::HitBlocked(point)`: `collision::resolve(old, new, edges)` returned a different point. `resolve` stops on crossing, so a step cannot jump over a thin band.
  - `Step::LeftMap(point)`: the new point is outside `bounds`.
- On a hit the system sends a `ProjectileHit { position, cause }` message and despawns the entity. The hit message is for later work (impact effect, damage). R-Town ignores it in this plan.
- The spawn point is the player's **feet**, not the hand. Feet are never inside a blocked area, so a shot cannot start inside a tree band. The sprite child starts at hand height.
- The kit does not know which sprite to show. The game spawns the entity (`Projectile`, `DepthSort`, `Transform`, `Visibility`, sprite child). A small `Projectile::new(direction, speed)` helper builds the component.

### 3.2 Game: `games/r-town/src/`

- `input.rs`: `PlayerInput { move_dir: Vec2, fire: bool }`, filled once per frame from the keyboard (arrows, WASD, X) in a system set `InputSet` in `Update`. `player.rs` loses `input_direction` and reads `PlayerInput`. A mouse source later adds to the same resource.
  - Ordering: `ScriptPlugin` writes its fake key presses in `PreUpdate`, so reading in `Update` sees them.
- `weapon.rs`: `WeaponPlugin`. When `fire` is set, it spawns a projectile at the player's feet with `facing * PROJECTILE_SPEED`. Values: `PROJECTILE_SPEED = 2.0 * WALK_SPEED`, expressed in code as that product (make `WALK_SPEED` `pub(crate)`), and the sprite height.
- `player.rs`: add `facing: Vec2` to `Player`, set in `move_player` when the direction is not zero.
- `main.rs`: add `ProjectilePlugin` (kit) and `WeaponPlugin`.

### 3.3 Art

- Pack `8.png` into `assets/art/features/projectiles.aseprite` with `dev-scripts/aseprite/pack_pngs.lua`, prefix `proj_`, tag `proj_8` (`adr-rt1`, `adr-rt6`). The `.aseprite` file is the source of truth. The PNG is not edited.
- Code uses `Animation::tag("proj_8")`.
- Add the file and tag to `games/r-town/docs/art-tags.md`.
- Check on screen that 13x11 px is a good size next to a 14x16 player. If not, pick another number from `1.png` to `7.png` and rebuild. Do not scale the sprite.

### 3.4 Script tool

Add `"x" => KeyCode::KeyX` to `key()` in `crates/game-kit/src/script.rs` and update its header comment, so `hold:x:0.1` fires one shot. Do not add a game-specific command (`adr-x15`).

## 4. Steps

Each step ends with `./dev-scripts/rust.sh lint` and, when tests exist, `./dev-scripts/rust.sh test game-kit`.

1. **Check Bevy 0.20.1 APIs** with the LSP and `bevy-src/` before code: `#[derive(Message)]` and `MessageWriter` (the script tool already uses `MessageWriter<AppExit>`), `Single` and `just_pressed`. Add anything new to `dev-docs/bevy/notes.md`.
2. **Kit, pure logic and tests.** Add `projectile.rs` with `step` and unit tests. Build the edges in tests with `CollisionWorld::add_blocked`. Cases:
   - open ground: moves `velocity * dt`;
   - straight into a face: stops on the edge, returns `HitBlocked`;
   - diagonal into a face: hit;
   - a step larger than the band: still a hit (no tunnelling);
   - open path beside a band: no hit;
   - point beyond each of the four map edges: `LeftMap`;
   - start exactly on an edge, moving away: no hit.
3. **Kit, plugin.** `Projectile`, `ProjectileHit`, `ProjectilePlugin`, the move system, the `debug!` log line for each hit. Export in `lib.rs`.
4. **Script key.** Add `x` to `script.rs` (3.4).
5. **Input module.** Add `input.rs` and move the key reading out of `player.rs`. Walking must behave as before: run the existing scripted walk and compare the end position.
6. **Facing.** Add `Player::facing` (D3). Unit test the pure part (direction to facing, keep the old value when the direction is zero).
7. **Art.** Build `projectiles.aseprite` (3.3). Update `art-tags.md`.
8. **Weapon.** Add `weapon.rs`, wire `main.rs`, make `WALK_SPEED` shared.
9. **Check in the running game** (workspace rule 9). Use `./dev-scripts/rust.sh script r-town "..."` with `screenshot:<name>`. Cases to see:
   - fire right on open ground, then screenshot while it flies;
   - fire at a lone tree: it disappears at the base band;
   - fire at a grove and at a pond: it disappears at the outline (F3 shows the outline);
   - fire left or up near the map edge: it disappears at the edge, no entity stays (check the log);
   - fire after a diagonal walk: diagonal shot;
   - spawn count returns to zero (log or F2 info box).
   Report which parts were not seen on screen.
10. **Lint, test, web.** `./dev-scripts/rust.sh lint`, `test`, `web`. No new dependency is expected, so the web check is a compile check only.
11. **Commit.** Ask first. Do not push.
12. **Migrate lasting content, then delete this plan.**
    - `dev-docs/extensions/projectiles.md` (new) and a row in `dev-docs/extensions/README.md`: the kit API, the ground-point rule, the hit message.
    - `dev-docs/adr/EXTENSIONS.md`: `adr-x21` projectiles in the kit (D1, D4). Next free number, check before writing.
    - `games/r-town/docs/adr.md`: `adr-rt11` weapon values, X key, 8-direction facing, 2x walk speed (D3, D5), and D6: projectiles in flight are not despawned on an F1 map reload (a development tool, a stray shot is harmless).
    - `dev-docs/adr/EXTENSIONS.md` or `adr-rt` entry for D2 (input resource, move to the kit later). Put it where the next owner will look: `adr-x22` if the kit move is decided, otherwise in `adr-rt11`.
    - `games/r-town/docs/SPEC.md` section 4 (abilities: X fires) and `games/r-town/docs/README.md` (controls, source layout: `input.rs`, `weapon.rs`).
    - `docs/README.md`: controls table if it lists keys.
    - `dev-docs/ROADMAP.md`: add "Projectile weapons" under Completed Items. Add future items: hit effects and damage, enemies as targets, mouse aiming and firing, weapon values (cooldown, range).
    - `dev-docs/bevy/notes.md` and `concepts.md`: messages, if first use in the project is a new concept there.
    - Delete `dev-docs/PLAN-projectiles.md`.

## 5. Risks and notes

- **Sideways early stop** (D4): a shot may stop up to 5 px short of a tree's side. Watch for it in step 9.
- **Sprite size**: the 13x11 ball is nearly as large as the player. It may look too big (step 9).
- **Depth sort per shot**: many shots at once cost one `DepthSort` each. Fine at this scale.
- **Diagonal facing with sprite flip**: the player flips only left and right. The shot direction and the sprite are therefore not tied (a shot up shows the player as before). Player art for up and down is out of scope.
- **Map reload (F1)**: projectiles in flight on a reload keep flying against the new edges. Accepted (D6).
- **Unknown**: `ProjectileHit` has no reader. Keep it small until damage exists.
