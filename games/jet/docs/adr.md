# Jet: decisions

## adr-jt1. A port from Pyxel that improves the game and keeps the player's feel (2026-11-26)

**Decision.** Jet is ported from the user's Pyxel game into this workspace as the crate `jet`. The port is not 1:1: it adds a larger 16:9 screen, a new palette, path finding, particles, a HUD and a game over. It keeps the player's speed, the instant start and stop, the key rules and the "moving player is the weapon" combat exactly. The Pyxel port brief is the source of the rules; `SPEC.md` holds the rules the port keeps.

**Rationale.** The designer's intent is a player who is extremely fast and agile, and a swarm that grows. The Pyxel version proves the feel, but is limited by its palette, its 320 px screen and an engine bug that turns off path finding. A plain copy would carry those limits into Bevy.

**How to apply.** Change the feel rules in `SPEC.md` section 4 only with the user. Where the brief and `SPEC.md` differ, `SPEC.md` wins.

## adr-jt2. 960 × 540 window, 16 px cells, a 28 px HUD bar at the top (2026-11-26)

**Decision.** The window is 960 × 540 art pixels (16:9), one fixed screen with no scrolling. A cell is 16 px, the 8 px Pyxel tile at 2×. A 28 px HUD bar is at the top, and the 512 px high map is below it.

**Rationale.** 16:9 fits modern screens. 960 × 540 is a whole-number 2× on a 1080p screen, so the pixels stay even. Pixel art at 2× keeps the original proportions. 540 is not a whole number of 16 px cells: 32 rows (512 px) leave 28 px, which is the HUD bar.

**How to apply.** The view height of the game region is 512. The HUD uses the kit bar layout at the top (`ScreenMode::BarTop`).

**Updated 2026-11-28: 32 px cells.** A cell is now 32 px, and the art is drawn for it (the sprites resized 2× again, the trees redrawn by the user at 64 × 64). The view shows 1920 × 1024 art pixels for the map and a 56 px HUD bar. The window stays 960 × 540 logical pixels, so on a Retina screen (scale factor 2) one art pixel is one screen pixel. The map stays 60 × 32 cells. Why: the user wants more detail in the art. 64 px cells were rejected: the view would be 3840 × 2048 art pixels and the developer's 2560 × 1600 laptop would have to downscale it, which drops half the pixels. At 32 px cells there is no room for a whole-number 2×, so a 1080p screen shows 1×. The view height of the game region is now 1024.

## adr-jt3. A 40 × 32 field in the centre, with solid forest at the sides (2026-11-26)

**Decision.** The map is 60 × 32 cells. The play field is the middle 40 × 32 cells, with the house exactly in the centre. The 10 columns at each side are solid forest.

**Rationale.** The time a Spinner needs to reach the house sets how fast the swarm forms. A field of 40 × 32 gives 18 cells from the sides (4.8 s, as in the original) and 14 cells from the top and the bottom (3.7 s). A map that fills the 60-column screen would make enemies from the sides take about 7.5 s, and the swarm forms too slowly. 32 rows put the house exactly in the centre, so every side is the same; 33 rows would not. The side forest fills the 16:9 screen without making the field larger.

**How to apply.** Enemies spawn at the field edge, not the screen edge. Change the field size only with the user, and check the travel times again.

## adr-jt4. Speeds in cells per second, from the real Pyxel speeds (2026-11-26)

**Decision.** The player moves 30 cells/s (480 px/s) and the Spinner 3.75 cells/s (60 px/s). These are the real speeds of the Pyxel game (240 and 30 px/s at 8 px tiles), not the values in its code (220 and 50 px/s, which its rounding changed).

**Rationale.** The real speeds are what the game feels like now. Keeping cells per second keeps that feel at the new cell size. The values are then tuned by play.

**How to apply.** Write speeds as cells per second times the cell size, so a cell size change keeps the feel. With 32 px cells (`adr-jt2`, 2026-11-28) the speeds are 960 px/s and 120 px/s.

## adr-jt5. Art from the Pyxel `.aseprite` files, resized 2×, white line art on black, a new palette (2026-11-26)

**Decision.** The art is the Pyxel game's `.aseprite` files, copied into the game, resized 2× and loaded directly with `bevy_aseprite_ultra`. No exported sprite sheets. The files are not marked `_placeholder`. Their tags and layers are standardised with the user. Sprites stay thin white line art on a black background. The game gets a new palette, which starts with black, white and the colours the game recolours to (damage, healing, dying), and grows when particles are added.

**Rationale.** The `.aseprite` file is the source of truth in this workspace (`adr-a1`), so the art can be changed in one place. The white line art is the game's look. The Pyxel game recolours its white pixels for feedback, so the palette must hold those colours from the start.

**Updated 2026-11-28.** With 32 px cells (`adr-jt2`) the Pyxel sprites are resized 4× in all. The trees are new art by the user (`trees.aseprite`, 64 × 64); its 6 colours were added to the palette (indexes 8 to 13).

**Updated 2026-11-28 (later): black line art on snow.** The sprites are black line art, and the ground is a blue-tinted off-white, `#F4F8FB` (palette index 14). Why: the user wants Jet to take place in a snowy realm. The user will also give the sprites a more 3D look over time, by hand and a little at a time, so the art will be a mix of flat and shaded sprites for a while. A colour tint cannot recolour black pixels (a tint multiplies, and black stays black), so the colour feedback (dying, house damage, heal charge) needs a palette swap or coloured frames: open question Q9 in step 5.

**How to apply.** Frame durations are set in the files (`adr-a6`). Every new colour is added to the palette file first.

## adr-jt6. The map is a kit text file now, and the forest becomes dynamic later (2026-11-26)

**Decision.** The map is a text file in the kit format (`dev-docs/extensions/map-files.md`), like Long Way. A later version changes the forest while the game runs; the map file is then the start state only.

**Rationale.** The kit format and its tools (F1 reload, the F3 grid, errors with line numbers) work at once. The dynamic forest is a later feature, so the port only keeps the way open: trees and their blocked areas must be able to change at run time.

**How to apply.** Do not assume the forest is fixed after load: the collision world and the nav grid must rebuild when a tree is added or removed. The user may later change the format so the file gives the outline of a forest area and the game fills it with trees.

## adr-jt7. Enemies use path finding and spread out (2026-11-26)

**Decision.** Spinners find a path around trees and the house, at the same speed in every direction. They should spread out and not stack. A flow field (one search from the house that every Spinner reads) is explored during the port.

**Rationale.** In the Pyxel game an engine bug turns off path finding, so Spinners go in straight lines and freeze when a tree blocks them. A swarm that flows around the forest fits "the enemies swarm in on the tower". A flow field costs one search for all enemies, which suits a large swarm with one goal and a forest that changes.

**How to apply.** Use the kit movement (`Mover`, a Jet strategy) first. A flow field or spreading in the kit is a kit change: ask first.

## adr-jt8. A game-over screen with restart, and no title screen (2026-11-26)

**Decision.** On the 4th hit the house is destroyed. Play stops, a screen shows "house destroyed" and the survival time, and a key restarts the game. The port has no title screen; the game starts playing.

**Rationale.** The Pyxel game only logs the destroyed house and plays on forever, so there is no stake. A restart with the survival time gives a goal. A title screen adds nothing to the port.

**How to apply.** Use the kit app states (`AppState::GameOver`, `state_root`). A restart resets everything: enemies, house, timers, difficulty, player and HUD counts.

## adr-jt9. The HUD shows survival time and kill count (2026-11-26)

**Decision.** The HUD bar shows the survival time and the kill count. It does not show the house's HP.

**Rationale.** The house colour already shows its damage level. Time and kills give the player a score to beat.

**How to apply.** Use the kit HUD widgets in the HUD bar (`adr-jt2`).

## adr-jt10. The camera shakes when the player hits a wall, with a kit screen effect (2026-11-26)

**Decision.** The camera shakes on every block except the house: any tree (in the field or the side forest) and the map edge. The shake is a kit screen effect (`screen_fx`), with Jet's values: 0.1 s, 2 px along the direction of travel, flipping sides each frame. Changed on 2026-11-26 after the user's play test; at first only the field edge shook. With 32 px cells (2026-11-28) the shake is 4 px, the same distance on screen.

**Rationale.** The Pyxel game shakes at the screen edge. After the first play test of the port, the user chose a shake for every wall the player hits, not only the field edge. The user wants screen effects as a kit concept so other games can use them.

**How to apply.** The player's blocked check tells only the house from a wall (`PlayerBlocked { by: House | Wall }`). The kit decision is `adr-x61`.

## adr-jt11. Bugs of the Pyxel game that the port fixes (2026-11-26)

**Decision.** A dying enemy is marked dead at once and no hit check sees it again. Enemies move at the same speed in every direction. These follow from the brief's list of known issues.

**Rationale.** In the Pyxel game a dying enemy can be killed again, which makes a second splatter and refreshes the heal charge. A diagonal Spinner moves about 1.4 times faster than a straight one. Neither is part of the design.

**How to apply.** Every hit check skips dead enemies. Test it.

## adr-jt12. Extension points are stubbed, and the kit patterns are used from the start (2026-11-26)

**Decision.** The port stubs the shape of the planned features: more enemy kinds, pickups that spawn while playing, power-ups that change the player's stats, and a forest that changes over time. The player and the enemies use the kit patterns from the start (`Mover` and strategies, `HitBox`, `Rng`, `layers`, `fx`, the app states, scripted runs, the debug tools).

**Rationale.** The user will add these features soon after the port. A port that hard-codes one enemy, fixed stats and a fixed forest would need rework. Kit patterns used from the start avoid the bugs that the kit already fixes.

**How to apply.** Enemy kinds are rows of a table, power-ups change one stats resource, pickups use the kit `Collectable`, and a forest change rebuilds collision and the nav grid. Stubs compile and are documented, with no gameplay.

## adr-jt13. One file per character, a hidden black backdrop, Pyxel animation speeds (2026-11-26)

**Decision.** The Pyxel `sprites.aseprite` is split into `player.aseprite` (tags `Idle`, `Move`, `Trail`) and `spinner.aseprite` (tags `Move`, `Die`, `Kill`), one sprite per frame. Every art file keeps a hidden black layer `backdrop` at the bottom, for drawing; the Pyxel background layers are removed. Frames last 62 ms (16 fps), the house 250 ms (4 fps), as in the Pyxel game. The palette uses the exact Pyxel colours.

**Rationale.** One file per character with one sprite per frame is how Long Way and R-Town load art, and tags name each animation. A black backdrop lets the user see white line art while drawing, and the loader draws only visible layers, so the game ignores it with no code. The Pyxel speeds keep the original look; the files had 100 ms everywhere, which did not match the game.

**Updated 2026-11-28.** The backdrop is now the snow colour `#F4F8FB`, so the user draws black art on the game's ground.

**How to apply.** Keep `backdrop` hidden. A new enemy gets its own file with the same tag names where they fit (`Move`, `Die`, `Kill`). The file list and tags are in `art-tags.md`.

## adr-jt14. The forest is a kit cell area with no passable edge cells (2026-11-26)

**Decision.** The forest (the side forest and the groups in the field) is a kit cell area (`adr-x62`), drawn in the map file as outlines. Every forest cell is solid. There are no passable edge cells. The player passes forest corners by its centre-line collision (`adr-jt16`), not by the kit sliding.

**Rationale.** The Pyxel game made some edge tiles passable only because its collision tested one point on the centre line and caught on corners. The kit sliding solves that problem directly. A cell area also gives the later dynamic forest its model.

**How to apply.** Tree sprites are placed by the game from the area's cells (a tree every 2 columns and every row, random variant and offset). The forest edge feel is set by `adr-jt16`.

## adr-jt15. Trees, the house and actors sort by their feet, with the kit depth sort (2026-11-26)

**Decision.** Jet uses the kit depth sort, as Long Way and R-Town do. Trees, the house, enemies and the player are one world layer, sorted by their feet. It does not use the fixed layers of the Pyxel game (trees, house, enemies, player, effects).

**Rationale.** The user chose the kit pattern over the Pyxel draw order. Trees in lower rows are drawn over the rows above them, which suits the dense, overlapping clusters, and an actor just above a tree is drawn behind its top.

**How to apply.** A tree sorts at its cell (`SortAtArea`), the house at its blocked area (`SortAtBlocker`), and actors by their feet (`DepthSort`). Effects use the kit layers above the world.

## adr-jt16. The player tests one point at its leading edge, on its centre line, with no grown areas (2026-11-26)

**Decision.** The player's collision tests one point: the middle of its leading edge, which is on its centre line. The blocked areas are not grown by the player's size. The test runs from the player's centre to that point plus the step, with the kit `resolve`. The kit assisted sliding is not used. The user confirmed it in the first play test (2026-11-26).

**Rationale.** This is the Pyxel rule: it tests the cell just ahead of the leading edge, on the centre line. It keeps the feel the port must keep (`adr-jt1`): the player slides past a tree corner that only its side overlaps, and fits through a gap of one cell. A grown area or a feet point with sideways growth would close the one-cell corridors of the map, and at 8 px a frame the player could not line up with a narrower gap. The kit sliding starts only after the player pushes against a face for a while, and a Jet player never pushes: a block stops it and clears the held keys. The test starts at the centre, which never stands in a blocked area, so a tree that already overlaps the front half after a turn still stops the player.

**How to apply.** The forest cell area and the house have no growth. Enemies choose their own collision rule. This changes the sliding part of `adr-jt14`: the player passes forest corners by this rule, not by the kit sliding.

## adr-jt17. A Spinner hits the house when it arrives at its target point (2026-11-26)

**Decision.** A Spinner walks to one of four target points next to the house and hits the house the moment it arrives there. The targets are the cells left (−1, +1), right (+4, +1), below left (0, +4) and below right (+3, +4) of the house's top left cell, with the feet at the bottom middle of the cell. The brief's below right target (+4, +4) is moved to (+3, +4). A Spinner whose target has no path picks another of the four.

**Rationale.** The user chose the simplest rule (plan question Q6). A Spinner is one cell wide, so on its target its sprite already touches the house wall, and the hit looks like contact. It never pushes into the house, so the roof row needs no rule. The brief's (+4, +4) touches the house only at a corner, so it is moved under the house to touch a wall like the others.

**How to apply.** The hit marks the Spinner `Dead` at once (`adr-jt11`), stops it, plays `Kill` once and sends `HouseHit`. The Spinner is removed when the animation ends.

## adr-jt18. Spawning uses the extended kit spawner, and the difficulty is worked out from the level number (2026-11-26)

**Decision.** One kit `Spawner` puts the Spinners on the field (`adr-x63`): groups, the first group one gap after the start, and places on the free cells of the four field edges, one group of places per edge. Jet keeps the difficulty curve: every 10 s of game time the level goes up, Jet draws a new gap from the level's range and sets it and the group size on the spawner. The level's values come from the level number in whole hundredths and tenths.

**Rationale.** The user chose to extend the kit spawner rather than write a Jet spawner (plan question Q7). Whole-number maths gives exactly the table of the SPEC (0.8 to 1.2 s from level 7, group 2 from level 25), which float steps do not.

**How to apply.** The spawner is spawned in the map's `load` hook, so F1 starts it and the difficulty again. The script command `level:<n>` jumps to a level for tests.

## adr-jt19. Colour feedback is drawn in the art, not recoloured in code (2026-11-28)

**Decision.** Every colour change of the brief (a dying Spinner, the house damage levels, the player's heal charge) is drawn by the user as frames and tags in the `.aseprite` files. The game only switches tags. There is no sprite tint and no palette swap shader.

**Rationale.** The art is now black line art with hand shading (`adr-jt5`). A tint multiplies colours, so it cannot recolour black, and it would change the shading too. A palette swap would need a mapping for every shade the user adds. Drawn frames need no shader code, and they let a state look different in more than colour (cracks or smoke on a damaged house, for example).

**How to apply.** A colour state is a tag. The names are agreed in `art-tags.md` before the code plays them: the house has one idle loop per damage level (`Idle1` to `Idle3`), and the player has `IdleCharged` and `MoveCharged`. When a tag is missing, the game logs a warning and keeps the current look; it does not recolour. This answers Q9 of the port plan.
