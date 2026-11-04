# R-Town documentation

Documentation for this game only. General rules that other games may reuse are extensions, in `dev-docs/extensions/`. This folder holds what R-Town adds on top of them: the specification, values, art and decisions.

| Document | Contents |
|---|---|
| [SPEC.md](SPEC.md) | The specification: what R-Town is, what is decided, what is deferred, open questions |
| [adr.md](adr.md) | Decisions that apply to R-Town only (`adr-rt1` and on) |
| [art-tags.md](art-tags.md) | The `.aseprite` files, every frame and tag, the map kinds and their collision bases |

Related:
- How the game was started: `dev-docs/WORKFLOW.md`
- Shared code: `crates/game-kit/`, extensions in `dev-docs/extensions/README.md`
- Run it: `./dev-scripts/rust.sh run r-town [map]`. The optional map is a path or a name in `assets/maps/`. Edit the file, then press F1 in the game to reload it. A scripted run: `./dev-scripts/rust.sh script r-town "teleport:648:640;hold:up:2;screenshot:tree;quit"`. Arrow keys or WASD walk the player and X fires a projectile. F1 reloads the map and resets the game (player, enemies, projectiles), F2 toggles the info box, F3 the grid, collision overlay and movement debug (paths, goals, spawners). `R_TOWN_SEED=<number>` makes the random patrols repeat. Add `LOG_FILTER="enemy:|projectile:|player:|script:"` to a scripted run to see the enemy and projectile log lines.

Source layout (`games/r-town/src/`): `main.rs` builds the app, `camera.rs` has the view height, cell size and colours, `collision.rs` the player's sizes, `map.rs` loads the map and names the kinds, `trees.rs` spawns trees with their blocked bases, `player.rs` moves and animates the player, `input.rs` turns the keyboard into `PlayerInput`, `weapon.rs` fires projectiles (flight is `game_kit::projectile`), `enemy.rs` has the skeleton and the goblin and their spawners (movement is `game_kit::movement`, `strategies`, `nav`, `spawner`), `death.rs` removes the player on a touch, `walk_animation.rs` has the flip and bob that the player and the enemies share. The spawners and their strategies are legend entries of `assets/maps/test.txt` (`dev-docs/extensions/spawners.md`). Map: `assets/maps/test.txt`. Art: `assets/art/`.
