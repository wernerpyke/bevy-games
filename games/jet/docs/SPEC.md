# Jet: specification

Status: draft (2026-11-26). The port from Pyxel is planned and not built. Decisions: `adr.md`.

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
| Speed | 30 cells/s, 480 px/s (`adr-jt4`). The player crosses the 40-cell field in about 1.3 s. |
| Acceleration | None. Full speed at once, full stop at once. |
| Grid | None. The player moves freely in pixels along one axis. |

The speed and the instant start and stop are the core of the feel. Keep them.

### Blocked

Running into a tree, the house or the field edge stops the player **and clears the held direction**. Holding a key against a wall does nothing.

| Blocked by | Result |
|---|---|
| Field edge: the map edge at the top and bottom, the side forest at the left and right | Stop. Camera shake: 0.1 s, 2 px along the direction of travel, flipping sides each frame (`adr-jt10`). |
| A tree inside the field | Stop. No shake. |
| The house | Stop. Heals the house if a heal is charged (section 7). No shake. |

The original tests one point: the cell just ahead of the leading edge, on the centre line, so the player slides past tree corners. The port uses the kit collision; the feel is tuned by play (plan question Q4).

### Animation and attached sprites

- Idle: one frame, not rotated.
- Start moving or change direction: the move animation plays once at 16 fps, rotated to the direction, and holds its last frame. A trail one cell behind and a shockwave one cell ahead (3 cells wide, across the direction) loop their frames, move with the player and turn with it. Stop: the trail and the shockwave go, and the idle frame shows.
- Art faces up. Right is 90° clockwise, down 180°, left 270° clockwise, about the sprite's centre.
- Heal charged: the player's white line art is drawn teal-green.

## 5. World and map

| Item | Value |
|---|---|
| Window | 960 × 540 art pixels, 16:9, 2× on a 1080p screen (`adr-jt2`) |
| HUD bar | 28 px at the top |
| Map | 60 × 32 cells of 16 px (960 × 512 px), below the bar, fixed. No scrolling. |
| Field | The middle 40 × 32 cells (640 × 512 px) (`adr-jt3`) |
| Side forest | 10 columns on each side, solid trees |
| Ground | Black |

- The map is a text file in the kit format (`dev-docs/extensions/map-files.md`, `adr-jt6`). The legend follows `adr-x37`: `@` is the player start, letters are features.
- **House:** 4 × 4 cells, exactly in the centre of the field. Its top row (roof and chimney) is passable; the lower three rows are solid. It loops a 4-frame chimney smoke at 4 fps.
- **Trees:** 2 × 2 cells. The original places them in groups every 2 columns and every 1 row, so the rows overlap and the clusters look dense. Each tree picks one of 8 images at random (the 5th twice as likely) and a small random offset (x −2 to +2, y 0 to +2 Pyxel pixels, so −4 to +4 and 0 to +4 px at 2×). The offset is only visual. Some edge cells of a group are passable, so sprites can overlap the forest edge a little. How trees are written in the map file is open (plan question Q3).
- The field layout is the brief's map (section 3 of the brief) fitted to 40 × 32. The brief's 40 × 40 collision map is the reference.
- The forest will change over time in a later version (section 12). Then the map file is only the start state.

### Draw order

Back to front: ground, trees, house, enemies, player with trail and shockwave, effects. Layers come from `game_kit::layers` (`adr-x9`).

## 6. Enemies

One kind now: the **Spinner**. More kinds come later (section 12).

### Spawning

- Each spawn puts a group of Spinners on the map. Each Spinner starts on a random free cell of a random field edge: the top row, the bottom row, or the field column next to the left or right forest (equal chance per edge).
- Each Spinner picks one of four target points next to the house at random. Relative to the house's top-left cell (x right, y down): left side (−1, +1), right side (+4, +1), below left (0, +4), below right (+4, +4).
- When it reaches its target it goes for the house. When it counts as hitting the house is open (plan question Q6).

### Movement

| Item | Value |
|---|---|
| Speed | 3.75 cells/s, 60 px/s (`adr-jt4`). The player is 8 times faster. |
| Path | Path finding around trees and the house (`adr-jt7`). The same speed in every direction. A blocked Spinner finds a new path and never freezes. |
| Collisions | Trees, the house and the field edge. Spinners pass through the player. Spreading (not stacking) is wanted (`adr-jt7`). |
| Animation | A 4-frame spin, looped at 16 fps, while moving. The same frames in every direction, not rotated. |

### Events

| Event | Result |
|---|---|
| Hits the house | Stops. Plays the red kill X once (2 frames). The house takes 1 damage. Removed when the animation ends. |
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
| 0 | White |
| 1 | Pink `#FF9798` |
| 2 | Magenta-red `#D4186C` |
| 3 | Purple `#7E2072` |
| 4th hit | Destroyed: game over (section 9) |

Each damage and each heal makes the house pulse: scale 1.0 → 1.2 → 1.0 about its centre, 0.1 s in all.

### Healing

1. A kill gives a heal charge for 2 s. The player is teal-green while charged.
2. While charged, the player runs into a solid house cell (not the roof row).
3. House damaged: the level goes down by 1, the colour follows, the charge is used and the player turns white, the house pulses.
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
- Every 10 s (level-up): group size +0.02; shortest gap −0.2 s while above 1.0 s; longest gap −0.4 s while above 1.5 s; a new gap is chosen and the spawn timer restarts.

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
- HUD bar (28 px, top): survival time and kill count (`adr-jt9`).
- Pause: the kit pause. The key is open (plan question Q11).
- Game over: the 4th hit on the house stops play, and a screen shows "house destroyed" and the survival time. A key restarts everything (`adr-jt8`).
- Kit debug tools: F1 reload, F2 info box, F3 grid and overlays, scripted runs.

## 10. Art

- Sources: the Pyxel game's `.aseprite` files (`sprites`, `house`, `forest`, `shockwave`), copied into `games/jet/input/`, resized 2×, and loaded directly. No exported sprite sheets (`adr-jt5`).
- Style: thin white line art on black. Colour is used only for feedback.
- Palette: a new Jet palette. It starts with black, white `#EEEEEE` and the feedback colours: grey `#A3A3A3` (dying), magenta-red `#D4186C` (kill X, damage 2), pink `#FF9798` (damage 1), purple `#7E2072` (damage 3), teal-green `#19959C` (heal charge). More colours are agreed with the particles.
- Animation speed is set as frame durations in the `.aseprite` files (`adr-a6`): 16 fps for the player, trail, shockwave and Spinner; 4 fps for the house.
- The files need work before use (plan step 1): `sprites.aseprite` holds the player above the trail in each 8 × 16 frame, every file has a visible `Background` layer, the tags do not match the brief, and every frame is 100 ms.
- Particles: a burst when an enemy dies, at the house on damage, and on a heal (Enoki, `dev-docs/extensions/particles.md`).

## 11. Audio

None.

## 12. Scope

- **Port:** everything above.
- **Later (stubbed in the port, `adr-jt12`):** more enemy kinds; pickups that spawn while playing (for example one that kills every enemy on screen); power-ups (for example a larger shockwave); a forest that changes over time.
- **Out of scope:** the web build, audio, a title screen.

## 13. Open questions

Asked at their plan step (`dev-docs/PLAN-jet-port.md`):

- Q3: how trees are written in the map file (one character per tree, or blocked cells filled with tree sprites at random).
- Q4: collision point and sliding for the player.
- Q5: the attack zone size, and how a power-up changes it.
- Q6: when a Spinner counts as hitting the house.
- Q7: spawning with the kit spawner (extended) or a Jet director.
- Q8: the flow field and spreading: in the kit or in Jet first.
- Q9: colour feedback by sprite tint or a palette swap shader.
- Q11: the pause key and the restart key.
