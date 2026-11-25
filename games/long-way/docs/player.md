# Player

The player character: values, controls, animation, hit points and what is still open. Decisions: `adr-lw15` (the Hunter sprite), `adr-lw19` and `adr-lw21` (start and arrival), `adr-lw28` (hit points), `adr-lw30` and `adr-lw38` (controls), `adr-lw42` (no invincibility frames). Code: `player.rs`, `hp.rs`, `collision.rs`. Art: `characters/player.aseprite` (`art-pipeline.md`, `art-tags.md`).

## Values

| Value | Now | Where |
|---|---|---|
| Walk speed | 300 px/s | `WALK_SPEED`, `player.rs` |
| Hit points | 3 at the start, 3 max | `START_HP`, `hp.rs` |
| Feet | 56 px below the canvas centre | `PLAYER_FEET_OFFSET`, `collision.rs` |
| Half width | 14 px (must stay above 12.8, `adr-lw15`) | `PLAYER_HALF_WIDTH`, `collision.rs` |
| Body height | 50 px | `PLAYER_BODY_HEIGHT`, `collision.rs` |
| Hit box (what enemies touch) | 28 × 50 px, standing on the feet | `HIT_BOX`, `player.rs` |
| Walk animation speed | 2.5 × the durations in the file | `WALK_ANIMATION_SPEED`, `player.rs` |
| Energy at the start | none, so the first cast needs a pickup | `weapons.md` |

Collision rules for the feet (trees, rocks, edges, sliding): `collision.md`.

## Controls

- **Click.** A left click sets a goal. The player walks there by the nav grid, around blocked cells. A click on a blocked place, or outside the walkable area, goes to the nearest place the player can reach. A new click replaces the goal (`adr-lw30`).
- **Arrow keys.** The player walks the way the keys point, with the collision and sliding rules of `move_feet`. While a key is held the player has no goal: the keys win over a click in the same frame. Two opposite keys cancel each other (`adr-lw38`).
- **Weapons.** 1 to 4 open the drawers, Z, X, C and V cast (`adr-lw38`, `weapons.md`). A weapon aims from the player's feet and the direction the player last walked in.
- The feet stay inside the map, and the body stays below the top edge (`limit_player`). While the screen fades the player stands still, and a map change clears the goal.

## Start and arrival

The player starts at the centre of the `player_start` cell (`@`) of the first map (`adr-lw19`). An `@` on an edge cell moves the start off the blocked half (`maps.md`, "Map edges"). After a map change the player stands on the `@` of the new map and keeps its facing (`adr-lw21`).

## Animation

`animate_player` in `player.rs`. The tags are in `art-tags.md`.

| State | Tag | Speed |
|---|---|---|
| Standing, any facing | `Idle` | 1.0 |
| Walking sideways | `Run` | 2.5 |
| Walking up | `Up` | 2.5 |
| Walking down | `Down` | 2.5 |
| Hurt | `Hurt`, once | 1.0 |

- Horizontal movement wins on a diagonal.
- The art faces right (`WALK_ART_FACES_RIGHT`). Walking left flips the sprite with `transform.scale.x`. Walking up or down keeps the last sideways flip.
- The game draws the `Body` layer. It draws `Weapon` only when `SHOW_WEAPON` is `true` (now `false`), and never draws `background` (`hunter_layers`).
- The art does not set the feet or the hit box. They come from the constants in `collision.rs`. The user moved the `Up` and `Down` frames up and down to make a bob, so the visible feet there are at rows 111 to 116, not 120. That is intended and changes no game value.

## Hit points and damage

- Other systems send `DamagePlayer { amount }` and never change `Hp` (`adr-lw28`). HP never goes below 0.
- No invincibility frames: every hit counts, also two in the same frame (b15, `adr-lw42`). A later invincibility time belongs to the player, as one rule for all damage.
- A loss sends `PlayerHurt`. The player plays `Hurt` once and can walk at once. The walk animation waits until `Hurt` ends. A hit does not stop a walk to a click.
- The HUD shows one icon per point of `max` (`hud.rs`).
- At 0 HP the player stays alive and damage stops. The death flow, `Die` and restart wait for `rm12`.
- What hurts the player: `enemies.md`.

## Open

- **Death:** play `Die` and restart (`rm12`). The lying `Die` frames have not been checked in the game yet.
- **Idle per direction:** only the side view has `Idle`. Name the tags (for example `IdleUp`, `IdleDown`) or keep one `Idle` (`rm1`).
- **Walk frames:** `Up` and `Down` have 6 frames, `Run` has 8. Keep or match.
- **Flip rule** when walking up or down, if the up and down art is not symmetric.
- **Weapon layer:** show it or not, and where the blade sits in the up and down views.
- **Speeds:** whether `WALK_ANIMATION_SPEED` and `WALK_SPEED` need a retune for the 128x128 sprite.
- **Sprite size:** whether 128x128 stays right next to the rest of the art (`rm1`).
- **Attacks:** the pack has melee, charge, special and dash sheets (`art-pipeline.md`). The weapons are spells cast from the HUD, so ask the user whether the Hunter needs melee or cast animations (`rm1`).
