# SAGA architecture

SAGA is one crate, `saga`, with a library and a small binary. The layers below
only ever depend downwards, which is what keeps the whole game logic testable
without a GPU.

```
        ┌──────────────────────────────────────────────┐
        │ src/main.rs        CLI: run / validate / info │
        ├──────────────────────────────────────────────┤
        │ src/app.rs         winit window + event loop  │
        ├──────────────────────────────────────────────┤
        │ src/ui.rs          verb bar, inventory,       │
        │                    dialogue, captions, input  │
        │                    routing                    │
        ├──────────────────────────────────────────────┤
        │ src/render/        wgpu sprite renderer,      │
        │                    bitmap font, batching      │
        ├══════════════════════════════════════════════┤
        │ src/engine/        scenes, scripts, dialogue, │
        │                    inventory, animation       │  no rendering,
        ├──────────────────────────────────────────────┤  no windowing
        │ src/definition/    YAML schema, loader,       │
        │                    include merging, validation│
        ├──────────────────────────────────────────────┤
        │ src/geometry.rs    points, rects, polygons    │
        │ src/error.rs       error and validation types │
        └──────────────────────────────────────────────┘
```

Everything below the double line is pure data and logic. `tests/engine.rs` and
`tests/demo_game.rs` exercise it through the real frame loop; the layers above
are compile-checked and exercised by hand.

## Definition layer

`src/definition/mod.rs` is the authoritative YAML schema. Every struct uses
`#[serde(deny_unknown_fields)]`, so a mistyped key is an error rather than a
silently ignored setting. Two enums carry most of the expressiveness:

- `Action`, internally tagged with `action:` (`say`, `go_to_scene`, `if`, …)
- `Condition`, internally tagged with `check:` (`has_item`, `all`, `not`, …)

`loader.rs` reads a game file, resolves `include:` recursively and deep-merges
the documents (the including file wins, maps merge key by key, circular includes
are reported). It returns a `LoadedGame`, which keeps the base directory and
resolves asset paths through `asset_path()`. That function rejects absolute
paths and `..` components, so a game file cannot read files outside its own
directory.

`validate.rs` then walks the whole definition and collects *all* problems into a
single `ValidationErrors`: missing scenes, entry points, items, dialogues,
dialogue nodes, sprites, animations, duplicate hotspot or actor ids, the
reserved actor id `player`, empty animations, non-positive frame rates and
dialogue nodes or choices that would dead-end the conversation. Authors see
every mistake in one run instead of one per run.

## Engine layer

`Engine` owns an `Arc<GameDefinition>` and a `GameState`, and exposes a small
input/query surface:

```rust
engine.update(delta_seconds);         // advance time
engine.click(point, MouseButton);     // interact
engine.set_verb(Some(Verb::Take));    // force a verb
engine.click_inventory(index);        // select / combine
engine.choose_dialogue(index);        // answer
engine.advance();                     // skip the current line
```

No frame in the engine knows anything about pixels on screen: it takes input and
elapsed time and answers questions (`state()`, `choices()`, `hover_label()`,
`actor_bounds()`, `target_at()`). That is the reason a full playthrough of the
demo runs in a millisecond in CI.

### The script model

Interactions do not run immediately. They are compiled into a queue of
`ScriptStep`s (`src/engine/script.rs`) that the update loop drains one step per
state transition:

| Step | Meaning |
| --- | --- |
| `Action` | one YAML action |
| `EnterScene` | scene change; pushes the scene's `on_enter` |
| `EnterDialogueNode` | shows a node and pushes its actions |
| `ResolveDialogueNode` | follows `goto` / offers choices / ends |
| `ResolveChoice` | applies a picked answer |

The runner reports a `ScriptStatus` of `Idle`, `Waiting`, `Caption` or `Actor`.
Blocking actions (`say`, `wait`, `move_actor`) simply leave the queue paused
until their status clears, which is how "walk there, then talk" reads naturally
in YAML. Branching (`if`, choices, node expansion) uses `push_front_all`, so
nested scripts run before whatever was already queued. `MAX_STEPS_PER_FRAME`
caps a frame at 256 steps so a badly written loop degrades instead of hanging.

A left click is resolved in this order:

1. a selected inventory item → the target's `use_with`
2. a verb forced by the verb bar or the number keys
3. the first verb of `Verb::PRIORITY` (walk, talk, take, use, look) the target
   defines
4. otherwise a default line ("I cannot take the sand.")

A right click is always `look`. If the target declares `walk_to`, the player
walks there first and the interaction is stored as a `PendingInteraction` that
fires on arrival. Clicks on the floor clamp the destination into the scene's
`walkable_area` polygon (`geometry::clamp_to_polygon`), so the player can never
leave the ground.

### State

`GameState` (`src/engine/state.rs`) holds the current scene, visited scenes,
variables, inventory, actor states, hotspot and actor overrides, the current
caption, the dialogue state including choices already used (`once`), the
selected item, the forced verb and the ending. It is plain data with no handles
into the renderer, so persisting it is a matter of deriving `Serialize`.

## Rendering layer

The renderer (`src/render/renderer.rs`) is a single instanced-quad pipeline. All
drawing — backgrounds, sprites, solid rectangles, text — becomes an instance of

```rust
struct Instance { position, size, uv_offset, uv_size, color }
```

Draw calls are recorded in submission order, then contiguous runs that share a
texture are merged into one `draw(0..6, range)`. Solid rectangles use a 1×1
white texture, so a whole frame is usually a handful of draws. The instance
buffer grows to the next power of two when a frame needs more room.

Text uses a hand-built 5×7 bitmap font (`src/render/font.rs`) that is baked into
an RGBA atlas at startup; there is no font file and no text-shaping dependency.
Horizontal sprite flipping is done with a negative UV width rather than a second
pipeline.

### Coordinate systems

| Space | Meaning |
| --- | --- |
| virtual | what game files are authored in (`virtual_width` × `virtual_height`) |
| window | physical pixels |

The shader receives a letterbox uniform (scale and offset) and maps virtual
coordinates to clip space, so the aspect ratio is preserved and the border is
black. `Renderer::window_to_virtual` inverts the mapping for mouse input, which
means the UI and the engine only ever deal in virtual pixels.

## Presentation and application layers

`src/ui.rs` is the only place that knows what the game looks like: the bottom
panel with the five verb buttons and the two-row inventory grid, the dialogue
choice list, the hover label, and word-wrapped captions. It draws the scene
background, then the actors sorted by `position.y` so characters overlap
correctly, using placeholder rectangles whenever art is missing. The same file
routes clicks, so the layout constants used for drawing and for hit-testing
cannot drift apart.

`src/app.rs` is the winit 0.29 event loop: it creates the window, keeps the
surface configured on resize, tracks the cursor, translates mouse buttons and
keys, and per redraw calls `engine.update(dt)` → `ui.draw(...)` →
`renderer.render()`. Frame time is clamped to 100 ms so a stalled window cannot
teleport the player.

## Extension points

- **A new action or condition** — add a variant to `Action`/`Condition`, handle
  it in `Engine::run_action` / `ScriptRunner::evaluate`, add any reference checks
  to `validate.rs`, and document it in the README.
- **A new verb** — extend `Verb`, its `label()` and `PRIORITY`, and the verb
  button list in `ui.rs`.
- **Saving and loading** — derive `Serialize`/`Deserialize` on `GameState` and
  write it next to the game file; the definition is immutable at runtime, so the
  state is the only thing worth persisting.
- **A different front end** — the engine is a library. Anything that can call
  `update`, `click` and read `state()` (a web canvas, a terminal, a test) can
  host a SAGA game.
