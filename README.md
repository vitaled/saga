# SAGA

**SAGA's Adventure Game Architecture** — a 2D point-and-click adventure framework
written in Rust, rendered on the GPU with [wgpu](https://wgpu.rs), where whole
games are written in YAML and no Rust code at all.

```
cargo run --release -- run games/demo/game.yaml
```

| Feature | Status |
| --- | --- |
| Point-and-click movement with walkable polygons | ✅ |
| Verb-based interaction (look / use / talk / take / walk) | ✅ |
| Branching dialogue trees with conditions | ✅ |
| Inventory, item-on-object and item-on-item puzzles | ✅ |
| Multiple scenes with entry points and enter/exit scripts | ✅ |
| Sprite sheets and frame animation | ✅ |
| Scalable background art, letterboxed virtual resolution | ✅ |
| Game state: variables, visited scenes, toggleable hotspots | ✅ |
| Games defined entirely in YAML, with `include:` for splitting files | ✅ |

## Contents

- [Installation](#installation)
- [Command line](#command-line)
- [Controls](#controls)
- [Your first game](#your-first-game)
- [YAML reference](#yaml-reference)
- [Example games](#example-games)
- [Development](#development)
- [Architecture](docs/ARCHITECTURE.md)

## Installation

SAGA needs a stable Rust toolchain (2021 edition) and a GPU backend supported by
wgpu 0.19 (Vulkan, Metal, DX12 or GL).

```sh
git clone https://github.com/vitaled/saga.git
cd saga
cargo build --release
```

On Linux the usual desktop development packages are required
(`libx11`, `libxkbcommon`, `libwayland` and a Vulkan or GL driver).

## Command line

```
saga run <game.yaml>       Play a game (this is the default command)
saga validate <game.yaml>  Load and check a game definition, then exit
saga info <game.yaml>      Print a summary of a game definition
```

`validate` is the fastest way to catch mistakes: it reports unknown fields,
missing scenes, items, dialogue nodes, animations and dead-end conversations
before the window is ever opened.

```sh
cargo run -- validate games/demo/game.yaml
cargo run -- info games/demo/game.yaml
```

## Controls

| Input | Effect |
| --- | --- |
| Left click on the floor | Walk there |
| Left click on an object | Apply the selected verb, or the object's most specific verb |
| Right click on an object | Look at it |
| Click a verb button | Force that verb for the next interaction |
| Click an inventory item | Select it; the next click uses it on the target |
| Click the selected item again | Deselect it |
| Right click an inventory item | Look at it |
| Click another inventory item while one is selected | Combine the two |
| `1`–`5` | Select a verb, `0` clears the selection |
| Space / Enter / click | Skip the current line of dialogue |
| Escape | Quit |

## Your first game

A complete, playable game fits on one page — this is `games/minimal/game.yaml`:

```yaml
title: One Room
start_scene: cell

player:
  name: Prisoner

items:
  key:
    name: key
    description: Small, cold and very welcome.

scenes:
  cell:
    name: Cell
    walkable_area:
      - { x: 40, y: 260 }
      - { x: 600, y: 260 }
      - { x: 600, y: 340 }
      - { x: 40, y: 340 }
    entry_points:
      default: { x: 200, y: 320 }
    on_enter:
      - action: say
        text: A cell, a door and a loose stone.
    hotspots:
      - id: stone
        name: loose stone
        area: { x: 120, y: 220, width: 60, height: 40 }
        walk_to: { x: 150, y: 300 }
        interactions:
          take:
            - action: say
              actor: player
              text: There is a key behind it.
            - action: give_item
              item: key
            - action: set_hotspot
              hotspot: stone
              enabled: false

      - id: door
        name: door
        area: { x: 420, y: 180, width: 70, height: 120 }
        walk_to: { x: 455, y: 320 }
        use_with:
          key:
            - action: end_game
              text: The door swings open. THE END.
```

```sh
cargo run -- run games/minimal/game.yaml
```

Add art by pointing `background`, `sprites` and `icon` at PNG files relative to
the game file. Everything renders as coloured placeholder boxes until you do, so
a game is playable long before it is drawn.

## YAML reference

### Top level

| Key | Type | Description |
| --- | --- | --- |
| `title` | string | Window title. **Required.** |
| `version`, `author` | string | Metadata shown by `saga info`. |
| `window` | map | Window and virtual resolution, see below. |
| `start_scene` | string | Scene the game starts in. **Required.** |
| `start_entry` | string | Entry point in that scene; defaults to `default`. |
| `player` | map | The player character. |
| `variables` | map | Initial variables (bool, integer, float or string). |
| `items` | map | Inventory items by id. |
| `dialogues` | map | Conversations by id. |
| `sprites` | map | Sprite sheets by id. |
| `scenes` | map | Scenes by id. **Required.** |
| `include` | list | Other YAML files to merge into this one. |

Unknown keys are always an error, so typos never fail silently.

```yaml
window:
  width: 1280          # initial window size in pixels
  height: 720
  virtual_width: 640   # the resolution the game is authored in
  virtual_height: 360
  fullscreen: false
```

All coordinates in a game file are in virtual pixels. The renderer scales the
virtual canvas to the window and letterboxes the remainder, so a game looks the
same at any window size.

```yaml
player:
  name: Nara
  sprite: player      # id of a sprite sheet
  walk_speed: 110     # virtual pixels per second
  scale: 1.0
```

### Scenes

```yaml
scenes:
  harbour:
    name: Stone Quay            # shown to the player
    background: art/harbour.png # relative to the game file
    walkable_area:              # polygon the player is clamped to
      - { x: 40, y: 250 }
      - { x: 600, y: 250 }
      - { x: 600, y: 340 }
      - { x: 40, y: 340 }
    entry_points:
      default: { x: 120, y: 320 }
      from_cliff: { x: 560, y: 300 }
    on_enter: []   # actions run when the scene starts
    on_exit: []    # actions run when the scene is left
    hotspots: []
    actors: []
```

A hotspot is a static, clickable region:

```yaml
hotspots:
  - id: barrel
    name: rain barrel            # shown in the hover label
    area: { x: 466, y: 236, width: 42, height: 48 }
    # or: area: { polygon: [{ x: 0, y: 0 }, ...] }
    walk_to: { x: 470, y: 316 }  # the player walks here first
    enabled: true
    interactions:                # look | use | talk | take | walk
      look:
        - action: say
          actor: player
          text: Rain water, and something metallic at the bottom.
    use_with:                    # reactions to inventory items
      rope:
        - action: give_item
          item: rusty_key
```

Hotspots declared **later** in the list are tested first, so small objects
should come after the large background regions they sit on.

An actor is a positioned, optionally animated character with the same
`interactions` and `use_with` maps:

```yaml
actors:
  - id: marla
    name: Marla
    sprite: keeper
    position: { x: 300, y: 300 }  # feet position
    animation: idle
    scale: 1.0
    walk_speed: 90
    visible: true
    interactions:
      talk:
        - action: start_dialogue
          dialogue: marla
```

### Items

```yaml
items:
  lantern:
    name: storm lantern
    description: Dry, and the wick is good.  # shown when looked at
    icon: art/items.png                      # image or sprite sheet id
    icon_frame: 2                            # frame index inside a sheet
    combine:                                 # item + item puzzles
      oil_can:
        - action: remove_item
          item: oil_can
        - action: say
          text: The lantern is full again.
```

### Sprites and animation

```yaml
sprites:
  player:
    image: art/player.png
    frame_width: 24
    frame_height: 48
    animations:
      idle:  { frames: [0], fps: 1 }
      walk:  { frames: [1, 2, 3, 2], fps: 8 }
      wave:  { frames: [4, 5], fps: 4, looping: false }
```

Frames are numbered left to right, top to bottom. `idle` and `walk` are used
automatically for walking characters; any other animation is played with
`play_animation`.

### Dialogues

```yaml
dialogues:
  marla:
    start: greeting
    nodes:
      greeting:
        speaker: Marla
        text: Back again?
        choices:
          - text: What happened to the light?
            once: true            # disappears after it is picked
            goto: story
          - text: Could I borrow that rope?
            condition:
              check: not
              condition: { check: has_item, item: rope }
            actions:
              - action: give_item
                item: rope
            goto: greeting
          - text: Just enjoying the view.
            end: true
      story:
        speaker: Marla
        text: The keeper never came back down.
        goto: greeting
```

A node shows its `text`, runs its `actions` and then either offers `choices`,
jumps to `goto` or ends the conversation (`end: true`). Nodes without any of
those are rejected by validation, so a conversation can never get stuck.

### Actions

Every action is a map with an `action` key.

| Action | Fields | Effect |
| --- | --- | --- |
| `say` | `text`, `actor?`, `duration?` | Shows a line; without `actor` it is narration. |
| `go_to_scene` | `scene`, `entry?` | Changes scene. |
| `start_dialogue` | `dialogue` | Starts a conversation. |
| `end_dialogue` | — | Ends the running conversation. |
| `give_item` | `item` | Adds an item to the inventory. |
| `remove_item` | `item` | Removes an item. |
| `set_variable` | `name`, `value` | Sets a variable. |
| `set_hotspot` | `hotspot`, `enabled`, `scene?` | Enables or disables a hotspot. |
| `set_actor_visible` | `actor`, `visible` | Shows or hides an actor. |
| `play_animation` | `actor`, `animation`, `looping?` | Plays an animation. |
| `move_actor` | `to`, `actor?` | Walks a character somewhere and waits for it. |
| `wait` | `seconds` | Pauses the script. |
| `if` | `condition`, `then?`, `else?` | Runs actions conditionally. |
| `end_game` | `text?` | Ends the game with an optional closing line. |

Use `player` as the actor id for the player character.

### Conditions

Every condition is a map with a `check` key.

| Condition | Fields |
| --- | --- |
| `has_item` | `item` |
| `variable_equals` | `name`, `value` |
| `visited_scene` | `scene` |
| `hotspot_enabled` | `hotspot`, `scene?` |
| `not` | `condition` |
| `all` | `conditions` |
| `any` | `conditions` |

A scene only counts as visited once its `on_enter` script has finished, so an
`on_enter` can check `visited_scene` for its own scene to tell a first visit
from a return visit.

### Splitting a game over several files

```yaml
# games/demo/game.yaml
include:
  - scenes/harbour.yaml
  - scenes/cliff.yaml
```

Included files are merged into the including file, key by key and map by map;
the including file wins on conflicts. Includes may nest, and circular includes
are reported instead of hanging. Asset and include paths are resolved relative
to the game file and may not leave its directory.

## Example games

- `games/demo/` — **The Keeper's Lamp**, a three-scene game with a conversation,
  an inventory chain and an ending. The art in `games/demo/art/` is procedurally
  generated placeholder pixel art; replace it with your own PNGs.
- `games/minimal/` — the single-file game shown above.

## Development

```sh
cargo build          # build the library and the `saga` binary
cargo test           # unit, integration and demo playthrough tests
cargo clippy         # lints
cargo fmt            # formatting
```

The engine is deliberately free of rendering code, so the whole game logic runs
headless: `tests/engine.rs` drives a small YAML game through the real frame loop
and `tests/demo_game.rs` plays the bundled demo from the first scene to the
ending. Only the wgpu and winit layers need a GPU and a display.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the design of the engine
and for the extension points.
