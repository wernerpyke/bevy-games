# Balance

Rules that set how the player, the enemies and the weapons compare. Use them when you tune a value (`weapons.md`, "Tuning") or add a weapon or an enemy. A value change that breaks a rule is a bug, unless the user changes the rule.

The rules are set by the user. Each rule has a stable ID `b<n>` (never reused or renumbered). Each rule says how it is enforced: a test that fails when a value breaks the rule, or "by hand" when no test can check it.

Values: `weapons/values.rs` (weapons), `ghost.rs` (`HIT_POINTS`), `hp.rs` (the player).

## Rules

| ID | Rule | Since | Enforced by |
|---|---|---|---|
| b1 | **The fire ring always kills a ghost it touches.** Where the ghost stands at the cast, how far it is pushed and whether it walks out of the ring make no difference. | 2026-11-13 | `b1_the_first_tick_of_the_fire_ring_kills_a_ghost` in `weapons/values.rs` |
| b2 | **The fire ring does not promise a one-hit kill on any enemy other than the ghost.** Such an enemy can die if it stays in the burn band for enough ticks, but that is not guaranteed. | 2026-11-13 | By hand: check it when a new enemy kind is added |

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
