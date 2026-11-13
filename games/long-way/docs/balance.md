# Balance

Rules that set how the player, the enemies and the weapons compare. Use them when you tune a value (`weapons.md`, "Tuning") or add a weapon or an enemy. A value change that breaks a rule is a bug, unless the user changes the rule.

The rules are set by the user. Each rule has a stable ID `b<n>` (never reused or renumbered). Each rule says how it is enforced: a test that fails when a value breaks the rule, or "by hand" when no test can check it.

Values: `weapons/values.rs` (weapons), `ghost.rs` (`HIT_POINTS`), `hp.rs` (the player).

## Rules

| ID | Rule | Since | Enforced by |
|---|---|---|---|
| b1 | **The fire ring always kills a ghost it touches.** Where the ghost stands at the cast, how far it is pushed and whether it walks out of the ring make no difference. | 2026-11-13 | `b1_the_first_tick_of_the_fire_ring_kills_a_ghost` in `weapons/values.rs` |
| b2 | **The fire ring does not promise a one-hit kill on any enemy other than the ghost.** Such an enemy can die if it stays in the burn band for enough ticks, but that is not guaranteed. | 2026-11-13 | By hand: check it when a new enemy kind is added |
| b3 | **One ice ring on its own never kills a ghost. A second ice ring can.** | 2026-11-13 | `b3_one_ice_ring_never_kills_a_ghost` in `weapons/values.rs` (`push + touch < ghost HP`) |
| b4 | **The tornado lasts twice as long as the ice ring** (its growth plus its hold). | 2026-11-13 | `b4_the_tornado_lasts_twice_as_long_as_the_ice_ring` in `weapons/values.rs` |
| b5 | **A ghost dies from one tornado only once its total contact passes 60% of the tornado's duration.** | 2026-11-13 | `b5_a_ghost_dies_from_one_tornado_only_after_60_percent_contact` in `weapons/values.rs`; total contact: `contact_is_counted_in_total_across_a_break` in `weapons/tornado.rs` |
| b6 | **One tornado can deal more damage than one ice ring.** | 2026-11-13 | `b6_one_tornado_can_deal_more_than_one_ice_ring` in `weapons/values.rs` |
| b7 | **A ghost walking at a standing player gets through the tornado.** | 2026-11-13 | `b7_a_ghost_walking_at_a_standing_player_gets_through` in `weapons/values.rs`; at work: `a_ghost_walking_at_a_standing_player_is_bumped_back_at_least_twice_and_gets_through` in `weapons/tornado.rs` |
| b8 | **A moving player is safe: the player's own movement never carries a ghost deeper into the tornado.** | 2026-11-13 | `the_players_own_movement_never_takes_an_enemy_deeper` in `weapons/tornado.rs` |
| b9 | **A ghost walking at a standing player is bumped back at least twice before it gets through the tornado.** | 2026-11-13 | `b9_a_ghost_walking_at_a_standing_player_is_bumped_back_at_least_twice` in `weapons/values.rs`; at work: the b7 test in `weapons/tornado.rs` |
| b10 | **A direct fireball hit kills a ghost.** | 2026-11-13 | `b10_a_direct_fireball_hit_kills_a_ghost` in `weapons/values.rs` |
| b11 | **The fireball blast alone kills a ghost.** | 2026-11-13 | `b11_the_fireball_blast_alone_kills_a_ghost` in `weapons/values.rs` |
| b12 | **The fireball blast deals less than a direct hit.** | 2026-11-13 | `b12_the_fireball_blast_deals_less_than_a_direct_hit` in `weapons/values.rs` |
| b13 | **A direct fireball hit does not kill every enemy kind.** | 2026-11-13 | By hand: check it when a new enemy kind is added (the ghost is the only enemy now) |
| b14 | **The enemy a fireball hits directly takes only the direct hit, never the blast as well.** | 2026-11-13 | `the_blast_hits_every_other_living_enemy_within_its_radius_and_no_one_else` and `an_enemy_in_the_way_takes_the_direct_hit_and_the_target_near_it_the_blast` in `weapons/fireball.rs` |

## b1. The fire ring always kills a ghost

**How.** The first tick kills: `dps * tick >= ghost HP` (the user chose this on 2026-11-13). With a ghost at 100 HP and a tick of 0.5 s, the fire ring deals 200 per second, so 100 per tick. The first frame in which a ghost touches the ring deals 100, wherever the ghost stands and at any frame rate. The test fails when the ring damage, its tick or the ghost's `HIT_POINTS` moves so that the first tick no longer kills.

**Why not more ticks.** Before b1, the ring dealt 40 per second (20 per tick), so a ghost needed 5 ticks (2.5 s in the ring). In a manual test, ghosts close to the player at the cast were hit once by the growing front, pushed outward, and left behind when the band jumped out to the full radius. They walked back with 80 HP and attacked. A ghost that walks through the burn band, or walks in late, also gets too few ticks.

**Results.**
- The fire ring does not push (`Pushback::None`): a ghost dies in the frame of the first touch, and the ring skips a dead enemy (D15), so a push would never show.
- The burn time only matters for a ghost that walks in after the cast.
- b1 is for the ghost only. Other enemies: see b2.

**Rejected:** an afterburn (a touched ghost keeps burning until it dies, even outside the ring). It shows a burn before death, but it is a new mechanic.

## b2. The fire ring is not a one-hit kill for other enemies

**Rule.** Only the ghost is sure to die from the fire ring (b1). Another enemy kind can have more HP than one tick deals (100). It dies only if it stays in the burn band for enough ticks, and nothing makes sure that it does. The rule is for the future: the ghost is the only enemy now.

**When a new enemy kind is added:**
- Do not raise the fire ring damage to one-hit kill it. That needs a new rule from the user.
- Give it a `max` HP of its own (`EnemyHp`). The ticks it needs are `HP / 100`, rounded up. The burn band is 48 px wide and is active for about 2.4 s (from 0.34 s after the cast to the end at 2.77 s). An enemy that stands in it the whole time gets 5 ticks (the unit test `a_body_in_the_burn_band_is_hurt_again_every_tick_until_the_ring_ends`), so an enemy with more than 500 HP cannot die from one ring.
- Keep b1: the new enemy's values must not change the ghost's HP or the ring's damage.

## b3. One ice ring never kills a ghost

**Rule.** The ice ring is a shield, not a damage weapon. It deals a medium amount once to an enemy it pushes, and a small amount once per cast to an enemy that walks into it. One ring on its own must not kill a full-HP ghost, but a second ring may finish a ghost that the first one hurt.

**How.** The most one ring can deal to one ghost is the push plus the touch. So `push + touch < ghost HP`. Start values: push 50, touch 10, so one ring deals at most 60 of 100, and two pushes deal 100.

## b4 to b9. The tornado

**Rules.** The user's intent (2026-11-13, `weapons.md`, "Tornado"): a mobile shield that lasts twice as long as the ice ring (b4), can kill a ghost that stays in contact with it for more than 60% of its duration (b5), does more damage than the ice ring (b6), lets a ghost reach a player who stands still (b7), and keeps a moving player safe (b8). Contact is total contact time, so a player can kill a ghost by holding the tornado against it. After the first play test (the user, 2026-11-13): the ghost is bumped back a couple of times on the way in (b9).

**How (start values, built in W3).**
- b4: duration 2 × (0.767 + 2) = 5.53 s, computed from `ICE_RING`, so it follows any change to the ice ring.
- b5: 26 per second in ticks of 13 every 0.5 s. A ghost (100 HP) dies on the 8th tick, at 3.5 s of contact, which is after 60% (3.32 s). The tick before, at 3.0 s, leaves 9 HP. The test: the killing tick is after 60% of the duration, and the tick before it does not kill. With ticks of 0.5 s the kill lands between 54% and 63%.
- b6: at most 12 ticks in 5.53 s, so 156 per cast. The ice ring deals at most 60 (push plus touch).
- b7: the steady push (20 px/s) is slower than the ghost (90 px/s), so between bumps a ghost walking in gets closer at 70 px/s. A bump takes back 40 px on every second tick (b9). The test walks a ghost across the whole radius (96 px) in contact time: it takes about 2.4 s and 5 ticks (65 of 100), so the ghost survives and arrives before the tornado ends. The real crossing ends at the player's body, which is shorter. In the game the ghost reaches the player, attacks and dies as usual.
- b8: the depth of an enemy in the tornado may only grow by its own steps, less the push. The player's movement adds nothing, so the edge shoves an enemy ahead of the player. The player walks at 300 px/s and a ghost at 90 px/s, so a moving player leaves ghosts behind. A bump only moves an enemy outward. Accepted gap (the user): walking straight at one ghost lets it through, in about the time of b7 (about 2.3 s of contact).
- b9: on the way in of b7, bumps land on the 2nd and 4th ticks (at 0.5 s and 1.5 s of contact). The test counts them in the same walk. In the game a bump is a 40 px shove over 0.1 s, so it reads as a knock back, together with the ghost's hurt animation.

## b10 to b14. The fireball

**Rules.** The user's intent (2026-11-13, `weapons.md`, "Fireball"): a high-power shot. A direct hit kills many enemies but not all (b13). The explosion deals a second, smaller damage to every enemy around it (b12). A ghost dies from either a direct hit or the blast alone (b10, b11). The enemy hit directly does not also take the blast (b14, the user the same day): a strong enemy that survives the direct hit is not punished a second time.

**How (start values, built in W4).**
- b10: direct 150 against a ghost's 100.
- b11: blast 100 against a ghost's 100, so a ghost anywhere in the blast (1.25 cells from the explosion to the nearest side of its body) dies.
- b12: 100 < 150.
- b13: no test yet. When an enemy kind with more than 150 HP exists, or the direct hit grows, check that at least one kind survives a direct hit.
- b14: the explosion deals the direct hit to the enemy touched and the blast only to the others (`explode` in `weapons/fireball.rs`).
