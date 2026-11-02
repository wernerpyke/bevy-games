# R-Town art catalogue

What is in each `.aseprite` file in `games/r-town/assets/art/`, and the tag names the code uses. Tag names are the contract between art and code (`games/long-way/docs/art-pipeline.md`, rule 7). They are case sensitive.

Source: the "Superpowers Medieval Fantasy Asset Pack (part 1)" in `input/` (gitignored). The files below are built from its single PNGs with `dev-scripts/aseprite/pack_pngs.lua` (`adr-rt1`, `adr-rt6`). The pack has its own palette, not a game palette. The PNGs are not edited.

## How the files are packed

- One canvas per file: the largest width and the largest height of its images.
- One frame per image. Each image sits at the **bottom centre** of its frame, so the feet of a character and the base of a tree share one line. The position is `x = (canvas width - width) / 2` (rounded down) and `y = canvas height - height`.
- One tag per frame, named `<prefix><source file name>`. A tag is a variant to pick, not an animation.
- In code: `Animation::tag("char_9")`. The entity is placed at the feet, and the sprite is a child lifted by half the canvas height.

To rebuild a file, run Aseprite in batch mode with the script (parameters `out`, `files`, `prefix`). The script prints the table below.

## `characters/characters.aseprite`

Canvas 16x17, 10 frames, prefix `char_`. Source: `medieval-fantasy-1/characters/1.png` to `10.png`. The sheet `0-characters.png` (11 characters) and `characters.gif` are not in the file.

| Tag | Image | Position in frame | What it shows (read from a scaled sheet) |
|---|---|---|---|
| `char_1` | 10x10 | 3,7 | boy with brown hair, blue shirt |
| `char_2` | 9x10 | 3,7 | child with blond hair, blue shirt |
| `char_3` | 12x10 | 2,7 | grey armoured creature or golem |
| `char_4` | 10x11 | 3,6 | bandit in a black hat, red scarf |
| `char_5` | 10x11 | 3,6 | person in a brown hat, orange clothes |
| `char_6` | 12x12 | 2,5 | person in an orange cone hat, pink robe |
| `char_7` | 16x17 | 0,0 | witch in black with a broom |
| `char_8` | 13x12 | 1,5 | pink hooded figure with a tool |
| `char_9` | 14x16 | 1,1 | **the hero**: blue wizard with a staff |
| `char_10` | 13x16 | 1,1 | dark wizard with an orb staff |

Every character is one static pose (`adr-rt3`). The hero is animated in code: flip for left and right, a one pixel bob while walking.

## `features/background-elements.aseprite`

Canvas 30x39, 21 frames, prefix `bg_`. Source: `medieval-fantasy-1/background-elements/1.png` to `21.png`. The terrain sheet `0-tileset.png` (320x192) is not in the file.

| Tag | Image | Position in frame | What it shows (read from a scaled sheet) |
|---|---|---|---|
| `bg_1` | 30x29 | 0,10 | stone gatehouse with an orange arched door |
| `bg_2` | 14x21 | 8,18 | small stone turret |
| `bg_3` | 12x13 | 9,26 | small orange cone roof |
| `bg_4` | 12x11 | 9,28 | small stone hut with an orange top |
| `bg_5` | 14x15 | 8,24 | orange tent or roof peak |
| `bg_6` | 14x14 | 8,25 | small round green tree |
| `bg_7` | 16x16 | 7,23 | small house with an orange roof |
| `bg_8` | 16x16 | 7,23 | stone building with a door |
| `bg_9` | 16x19 | 7,20 | house with an orange gable roof |
| `bg_10` | 14x13 | 8,26 | orange dome |
| `bg_11` | 14x12 | 8,27 | stone basin with water |
| `bg_12` | 16x16 | 7,23 | fountain |
| `bg_13` | 14x16 | 8,23 | round stone tower with a dark door |
| `bg_14` | 16x26 | 7,13 | knight statue |
| `bg_15` | 13x18 | 8,21 | orange beehive shaped hut |
| `bg_16` | 16x23 | 7,16 | house with an orange roof and cross windows |
| `bg_17` | 30x39 | 0,0 | castle with a red roofed tower |
| `bg_18` | 30x31 | 0,8 | **big green tree** |
| `bg_19` | 14x15 | 8,24 | **dead brown tree** |
| `bg_20` | 14x15 | 8,24 | small green pine tree |
| `bg_21` | 30x29 | 0,10 | stone arena with a red flag |

The descriptions come from looking at a scaled contact sheet. Check a sprite on screen before relying on its description.

## Map kinds

Kinds are the names a map legend uses (`assets/maps/test.md`). Each is defined in code (`src/map.rs`, `src/trees.rs`). A tree stands on its cell: the bottom centre of the art is the bottom centre of the cell.

| Kind | Tag | Blocking base (pixels from the tree centre) | Source of the numbers |
|---|---|---|---|
| `tree_big` | `bg_18` | -9 to 8 (columns 6 to 22 of 30) | bottom rows of `18.png` |
| `tree_dead` | `bg_19` | -4 to 3 (trunk columns 3 to 9 of 14) | bottom rows of `19.png` |

The blocked area is the base grown sideways by 5 (the hero's feet half width) and 4 deep (`adr-rt5`, `adr-rt6`). Above it the hero walks behind the tree.
