# Jet: specification

Status: draft (2026-11-26). The port from Pyxel is in progress: the map, the house, the forest, the player, the Spinners and the spawning are built. The combat (kills, house damage) is not. Decisions: `adr.md`.

Source: the Pyxel game Jet and its port brief, `~/Dark/pyxel/games/jet/BRIEF-port.md` (outside this repo). This file copies the rules of the brief that the port keeps, converted to the new cell size, and the changes the user decided. Where this file and the brief differ, this file wins.

## 1. Summary

Jet is a tower defence game in which the tower does not defend itself. The player character moves around the tower (a house) and kills the enemies that attack it.

The designer's intent:

> - the player character is extremely fast and agile
> - the enemies swarm in on the tower, at first only a few but they increase in number

The port is not 1:1. It keeps the player's feel and the core loop, and adds a larger 16:9 screen, a new palette, path finding, particles, a HUD and a game over (`adr-jt1`).

## 2. Targets and timing model

- Target: macOS native (workspace rule; the web build is paused, `adr-e11`).
- Timing model: real time. Movement uses `time.delta_secs()`. Not turn-based and not grid-locked. Timers use game time, so pause stops them.
- Distribution: personal, in this repo. The art is the user's own.

## 3. Core loop

1. Spinners come in from the edges of the field and move to the house.
2. The player dashes through them. The moving player is the weapon.
3. A Spinner that reaches the house damages it. A kill gives a short heal charge, and the player spends it by running into the house.
4. Spawns speed up over the first minute, then the groups slowly grow.
5. On the 4th hit the house is destroyed and the game is over. A key restarts.

## 4. Player

### Input

- The four arrow keys only. No attack button. Four directions, no diagonal.
- The player moves only while a key is held. Release all keys and the player stops at once.
- Key rules (the "agile" feel):
  1. Still, and an arrow key is **pressed**: move that way.
  2. Moving, and a **different** arrow key is pressed: switch to it at once. The old direction becomes the fallback.
  3. The key of the current direction is released: go on in the fallback direction if its key is still held, or stop.
  4. Only one fallback direction is remembered.
- A press starts movement, not a held key. A key held from before (for example after a wall stop) does not start movement.

### Movement

| Item | Value |
|---|---|
| Start | Next to the house: 2 cells left of its left edge, on its bottom row (the brief's tile (18, 23) against the house at (20, 20)). The map file marks it with `@`. |
| Speed | 30 cells/s, 960 px/s (`adr-jt4`). The player crosses the 40-cell field in about 1.3 s. |
| Acceleration | None. Full speed at once, full stop at once. |
| Grid | None. The player moves freely in pixels along one axis. |

The speed and the instant start and stop are the core of the feel. Keep them.

### Blocked

Running into a tree, the house or the field edge stops the player **and clears the held direction**. Holding a key against a wall does nothing.

| Blocked by | Result |
|---|---|
| A wall: any tree (in the field or the side forest), or the map edge | Stop. Camera shake: 0.1 s, 4 px along the direction of travel, flipping sides each frame (`adr-jt10`). |
| The house | Stop. Heals the house if a heal is charged (section 7). No shake. |

Collision tests one point, as in the original: the middle of the leading edge, on the centre line. The blocked areas are not grown. So the player slides past a tree corner that only its side overlaps, and fits through a gap of one cell (`adr-jt16`). A long frame cannot carry the player through a tree: the whole step is tested.

### Animation and attached sprites

- Idle: one frame, not rotated.
- Start moving or change direction: the move animation plays once at 16 fps, rotated to the direction, and holds its last frame. A trail one cell behind and a shockwave one cell ahead (3 cells wide, across the direction) loop their frames, move with the player and turn with it. Stop: the trail and the shockwave go, and the idle frame shows.
- Art faces up. Right is 90° clockwise, down 180°, left 270° clockwise, about the sprite's centre.
- Heal charged: the player is drawn teal-green (the method is open: Q9, `adr-jt5`).

## 5. World and map

| Item | Value |
|---|---|
| Window | 960 × 540 logical pixels, 16:9; 1920 × 1080 art pixels, 1× on a Retina or 1080p screen (`adr-jt2`) |
| HUD bar | 56 px at the top |
| Map | 60 × 32 cells of 32 px (1920 × 1024 px), below the bar, fixed. No scrolling. |
| Field | The middle 40 × 32 cells (1280 × 1024 px) (`adr-jt3`) |
| Side forest | 10 columns on each side, solid trees |
| Ground | Snow, a blue-tinted off-white `#F4F8FB` (2026-11-28) |

- The map is a text file in the kit format (`dev-docs/extensions/map-files.md`, `adr-jt6`). The legend follows `adr-x37`: `@` is the player start, letters are features.
- **House:** 4 × 4 cells, exactly in the centre of the field. Its top row (roof and chimney) is passable; the lower three rows are solid. It loops a 4-frame chimney smoke at 4 fps.
- **Trees:** 2 × 2 cells. The original places them in groups every 2 columns and every 1 row, so the rows overlap and the clusters look dense. Each tree picks one of the frames of `trees.aseprite` at random (5 on 2026-11-28, each as likely; the Pyxel game had 8, the 5th twice as likely) and a small random offset: x −16 to +16 px (twice the Pyxel range at 4×, so the columns of trees do not line up) and y 0 to 8 px up. Each tree stands on the bottom of its cell, so the trunk ends on the blocked area's edge and on its sort line; the canopy covers the row above, and the top row of a cluster reaches one cell above it (2026-11-26). 15% of the tree places stay empty at random, to break up the rows, but only inside the forest, where the trees beside and below hide the gap; a tree at the edge is always drawn. A tree at the end of a row shifts only outwards, so its thin art does not leave the end cell bare (2026-11-26). The offset and the gaps are only visual: every forest cell still blocks. Every forest cell is solid (`adr-jt14`); the player passes forest corners by its centre-line collision (`adr-jt16`). The Pyxel game's passable edge tiles are dropped.
- **Forest in the map file:** the forest is a kit cell area (`adr-x62`). The file draws only the outline of each forest area with `F`; the loader fills the inside. The game places the tree sprites over the forest cells. The forest can change while the game runs (section 12).
- The field layout is the brief's map (section 3 of the brief) fitted to 40 × 32. The brief's 40 × 40 collision map is the reference.
- The forest will change over time in a later version (section 12). Then the map file is only the start state.

### Draw order

Trees, the house, enemies and the player are one world layer, sorted by their feet with the kit depth sort (`adr-jt15`): a tree at its cell, the house at its blocked area. The ground is below it and effects above it. Layers come from `game_kit::layers` (`adr-x9`). The Pyxel game drew fixed layers instead.

## 6. Enemies

One kind now: the **Spinner**. More kinds come later (section 12).

### Spawning

- Each spawn puts a group of Spinners on the map. Each Spinner starts on a random free cell of a random field edge: the top row, the bottom row, or the field column next to the left or right forest (equal chance per edge). The kit spawner picks the cell and skips forest cells (`adr-jt18`, `adr-x63`).
- Each Spinner picks one of four target points next to the house at random. Relative to the house's top-left cell (x right, y down): left side (−1, +1), right side (+4, +1), below left (0, +4), below right (+3, +4). The brief has (+4, +4) for below right, which touches the house only at a corner; the port uses the cell under the bottom right cell, so every target touches a wall (`adr-jt17`). The feet go at the bottom middle of the target cell, so the sprite fills the cell.
- When it arrives at its target point it hits the house (`adr-jt17`). When the path finding finds no way to its target, it picks another of the four.

### Movement

| Item | Value |
|---|---|
| Speed | 3.75 cells/s, 120 px/s (`adr-jt4`). The player is 8 times faster. |
| Path | Path finding around trees and the house (`adr-jt7`). The same speed in every direction. A blocked Spinner finds a new path and never freezes. |
| Collisions | Trees, the house and the field edge, with the kit feet point (the bottom middle of the sprite) against the same ungrown blocked areas as the player (`adr-jt16`), so a Spinner can overlap a tree sideways by up to half a cell. Spinners pass through the player and through each other. Spreading (not stacking) is wanted (`adr-jt7`). |
| Animation | A 4-frame spin, looped at 16 fps, while moving. The same frames in every direction, not rotated. |

### Events

| Event | Result |
|---|---|
| Hits the house | Stops and is marked dead at once. Plays the red kill X once (2 frames). The house takes 1 damage (logged only until the combat is built). Removed when the animation ends. |
| In the player's attack zone | Dies (section 7). |

## 7. Combat and the house

### The player attacks

- The moving player is the weapon. The attack zone is a strip 3 cells wide, one cell ahead of the player, across the direction of travel: where the shockwave is drawn. The exact zone is tuned by play (plan question Q5).
- Every live enemy in the zone dies: it stops, is marked dead at once (so no check sees it again, `adr-jt11`), plays the grey die animation once (2 frames, 16 fps), shows a burst of particles, and is removed when the animation ends.
- Each kill gives the player a heal charge for 2 s. Each kill resets the 2 s.
- A player standing still kills nothing. Enemies never hurt the player. The player has no HP.

### House damage

| Damage level | House colour |
|---|---|
| 0 | Black, as drawn |
| 1 | Pink `#FF9798` |
| 2 | Magenta-red `#D4186C` |
| 3 | Purple `#7E2072` |
| 4th hit | Destroyed: game over (section 9) |

Each damage and each heal makes the house pulse: scale 1.0 → 1.2 → 1.0 about its centre, 0.1 s in all.

### Healing

1. A kill gives a heal charge for 2 s. The player is teal-green while charged.
2. While charged, the player runs into a solid house cell (not the roof row).
3. House damaged: the level goes down by 1, the colour follows, the charge is used and the player is drawn as normal again, the house pulses.
4. House not damaged: nothing happens and the player keeps the charge.
5. 2 s with no kill: the charge is lost.

## 8. Difficulty

No waves or levels on screen. Difficulty rises quietly.

| Value | Start |
|---|---|
| Group size | 1.0 (rounded to a whole number at each spawn) |
| Shortest spawn gap | 2.0 s |
| Longest spawn gap | 4.0 s |

- A repeating timer spawns a group. The gap is chosen at random between the shortest and longest gap, once, and stays until the next level-up. The first group spawns one gap after the start.
- Every 10 s of game time (level-up): group size +0.02 (rounded to a whole number); shortest gap −0.2 s down to 0.8 s; longest gap −0.4 s down to 1.2 s; a new gap is chosen and the spawn timer restarts. The values are worked out from the level number in whole hundredths and tenths, so they match the table exactly (`adr-jt18`). The brief says "while above 1.0 s", but the Pyxel game reached 0.8 s through float noise, and the table below is the rule.

| Time | Group size | Spawn gap |
|---|---|---|
| 0 s | 1 | 2.0 – 4.0 s |
| 30 s | 1 | 1.4 – 2.8 s |
| 60 s | 1 | 0.8 – 1.6 s |
| 70 s on | 1 | 0.8 – 1.2 s |
| ~250 s | 2 | 0.8 – 1.2 s |
| ~750 s | 3 | 0.8 – 1.2 s |

These numbers are settings the designer tunes.

## 9. UI and flow

- No title screen in the port (`adr-jt8`). The game starts playing.
- HUD bar (56 px, top): survival time and kill count (`adr-jt9`).
- Pause: the kit pause. The key is open (plan question Q11).
- Game over: the 4th hit on the house stops play, and a screen shows "house destroyed" and the survival time. A key restarts everything (`adr-jt8`).
- Kit debug tools: F1 reload, F2 info box, F3 grid and overlays, scripted runs.

## 10. Art

- Files: `player`, `spinner`, `house`, `shockwave` and `palette.aseprite` in `assets/art/`, built from the Pyxel game's `.aseprite` files, resized 4×, and loaded directly; `trees.aseprite`, drawn new for 32 px cells. No exported sprite sheets (`adr-jt5`, `adr-jt13`). Every file, frame and tag: `art-tags.md`.
- Style: black line art on snow (`#F4F8FB`), since 2026-11-28; Jet takes place in a snowy realm. The trees are coloured. The user will give the sprites a more 3D look over time, by hand, a little at a time (`art-tags.md`). Colour is otherwise used for feedback.
- Palette: a new Jet palette. It starts with black, white `#EEEEEE`, the snow `#F4F8FB`, the tree colours and the feedback colours: grey `#A3A3A3` (dying), magenta-red `#D4186C` (kill X, damage 2), pink `#FF9798` (damage 1), purple `#7E2072` (damage 3), teal-green `#19959C` (heal charge). More colours are agreed with the particles.
- Animation speed is set as frame durations in the `.aseprite` files (`adr-a6`): 16 fps for the player, trail, shockwave and Spinner; 4 fps for the house.
- Particles: a burst when an enemy dies, at the house on damage, and on a heal (Enoki, `dev-docs/extensions/particles.md`).

## 11. Audio

None.

## 12. Scope

- **Port:** everything above.
- **Later (stubbed in the port, `adr-jt12`):** more enemy kinds; pickups that spawn while playing (for example one that kills every enemy on screen); power-ups (for example a larger shockwave); a forest that changes over time.
- **Out of scope:** the web build, audio, a title screen.

## 13. Open questions

Asked at their plan step (`dev-docs/PLAN-jet-port.md`):

- Q4: collision point and sliding for the player. Answered: `adr-jt16` (the Pyxel rule), confirmed in play.
- Q5: the attack zone size, and how a power-up changes it.
- Q6: when a Spinner counts as hitting the house. Answered: when it arrives at its target point (`adr-jt17`).
- Q7: spawning with the kit spawner (extended) or a Jet director. Answered: the kit spawner, extended (`adr-jt18`, `adr-x63`).
- Q8: the flow field and spreading: in the kit or in Jet first.
- Q9: colour feedback by sprite tint or a palette swap shader.
- Q11: the pause key and the restart key.
