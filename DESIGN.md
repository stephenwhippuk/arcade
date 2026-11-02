# Arcade Engine Design

## 1. Purpose

Arcade is an educational Rust project for creating and reimagining classic
arcade games within a consistent 2D framework. The project should help its
developers learn how game systems fit together while remaining small enough to
understand and extend.

The engine will provide shared infrastructure for a range of games. Individual
games will retain ownership of their rules and world models rather than being
forced into one universal representation.

## 2. Goals

- Build a reusable foundation for multiple small 2D arcade games.
- Keep the codebase approachable for people learning Rust and game development.
- Separate engine services from game-specific rules.
- Support menus, gameplay, pause overlays, and settings without hard-coding a
  single screen sequence.
- Start with keyboard controls while keeping game logic independent of physical
  input devices.
- Prefer simple, explicit designs and introduce general-purpose systems only
  when experience with multiple games justifies them.

## 3. Non-goals for the initial implementation

- A general-purpose engine intended to support every genre or platform.
- A custom graphics API or a hand-written OpenGL backend.
- A full-featured physics simulation.
- A plugin architecture, multiple rendering backends, or a general-purpose ECS
  before they are needed.
- An embedded scripting language before a concrete game or authoring need
  demonstrates its value.

## 4. Design principles

### 4.1 Keep game rules game-specific

Pac-Man and Space Invaders are useful reference genres. The first implementation
is an original maze-chase game; a projectile-based game remains a possible
second implementation. They exercise different needs:

- Pac-Man suggests tile maps, grid-constrained movement, maze collision, and
  enemy navigation.
- Space Invaders suggests continuous movement, formations, projectiles, and
  frequent collision checks.

The shared engine should support both styles without requiring either game to
adopt the other's world model. A game may use a tile map, moving entities, or a
combination.

### 4.2 Prefer understandable Rust

Begin with explicit structs, enums, traits, and modules. Keep related behavior
visible and easy to follow. Add abstractions when they solve a demonstrated
problem, not merely to anticipate every possible future game.

### 4.3 Separate responsibilities at useful boundaries

Game code should depend on concepts such as input actions, drawing operations,
and audio playback rather than directly on windowing, keyboard events, or
library-specific handles. These boundaries should remain small and should not
become frameworks of their own.

## 5. High-level architecture

Initially, implement the engine and games as modules in one Rust crate. Split
the project into workspace crates only when a concrete need arises, such as
building the engine as a separate reusable library.

```text
Application
├── State stack: menu, game, pause, settings
├── Engine services: input, rendering, audio, assets, time
└── Active game: world, entities, collision, NPC behavior
```

### 5.1 Application and game loop

The application owns the main loop and coordinates state updates, input, and
drawing. Use a fixed update timestep for gameplay and collision consistency;
render as often as practical. Keep input collection separate from game logic.

### 5.2 State stack

Represent interface and game flow as a stack of states rather than a fixed
sequence of screens. Candidate states include the main menu, a game, a pause
overlay, and settings.

States handle their own update, drawing, and input. They request transitions
such as push, pop, or replace; the application applies those requests. This
allows a pause menu to appear over a running game's view and then be removed to
resume play. State behavior should make it clear whether states below an
overlay continue updating or drawing.

### 5.3 2D rendering

Use Macroquad for the initial windowing and 2D rendering prototype. It should
make basic shapes and sprites straightforward to draw while keeping the first
implementation approachable. Reassess the choice after the first game rather
than treating it as a permanent engine constraint. Keep game code behind a
small renderer-facing interface as the architecture develops so it does not
depend unnecessarily on a particular graphics API.

Do not implement a custom OpenGL backend for the initial version.

### 5.4 Input

Game logic consumes semantic actions such as `MoveLeft`, `Confirm`, and
`Pause`, not raw key codes. The initial input implementation maps keyboard
events to those actions. This leaves room for remapping and other control
devices without requiring game rules to change.

### 5.5 World, movement, and collision

Each game owns its world representation and movement rules. The engine may
provide basic collision queries, starting with simple 2D axis-aligned
bounding-box checks where appropriate.

Keep collision detection separate from movement decisions and collision
resolution. For example, a grid-based game can constrain movement to its maze,
while a game with freely moving ships can use continuous movement and projectile
hit checks. Add only the physics and resolution behavior required by an
implemented game.

### 5.6 NPC behavior

Start NPC behavior as Rust code, using simple functions or state machines as
appropriate. Keep behavior behind clear game-level boundaries, but do not
introduce a scripting runtime until there is a specific gameplay, educational,
or authoring benefit that justifies its complexity.

### 5.7 Assets and audio

Load sprites, sounds, and music through engine services rather than scattering
file paths or library-specific playback calls through game logic. Provide
simple audio playback for effects and music, and make volume settings available
to the settings interface.

## 6. Implementation sequence

1. **Window, loop, and renderer:** Create a window, establish update and draw
   flow, and render basic shapes.
2. **State stack:** Add a main menu, a game state, and a pause overlay. Verify
   that pause and resume work without coupling the menu to game internals.
3. **First game vertical slice:** Select a small game and implement its core
   loop using semantic input, game-owned world data, and the simplest useful
   collision behavior.
4. **Assets and audio:** Load sprites and sounds, play effects and music, and
   expose volume controls through settings.
5. **Second game:** Implement a game with meaningfully different movement or
   world structure. Use it to validate that the engine boundaries support more
   than one design without becoming overly generic.
6. **NPC behavior:** Add the behavior model needed by the selected games, then
   reassess whether scripting is warranted.

## 7. Decisions intentionally left open

- Which game to implement second to validate engine reuse beyond maze gameplay.
- Whether Macroquad remains suitable after the first game is implemented.
- Whether a game needs a tile-map helper or other reusable world utilities.
- Whether later games justify an ECS, a more complete physics system, or
  embedded scripting.
- The exact asset formats, audio library, and settings persistence approach.

Resolve these decisions against the needs of the first implementation, while
using a second game to test whether shared systems are genuinely reusable.

## 8. Success criteria

- A game can be entered from a menu, paused with an overlay, resumed, and
  exited without hard-coded navigation between screens.
- Game rules use semantic actions rather than physical keyboard keys.
- At least two games with different world or movement requirements can use the
  shared application loop and services.
- Core engine concepts remain understandable to a Rust learner, and new
  abstractions are supported by concrete use cases.

## 9. Current prototype

The first playable vertical slice is Maze Chase, an original grid-based game
with a custom maze and primitive graphics, rather than copied maps or assets.

- `main` owns the Macroquad window, input polling, and application loop.
- `input` maps keyboard keys to held and newly pressed semantic actions.
- `state` owns a stack of states and applies their requested transitions.
- `screens` implements the main menu, maze game presentation, pause overlay, and
  win/loss presentation.
- `maze` owns the map, movement, pellets, scoring, enemies, and game outcomes,
  independently of Macroquad.
- `time` accumulates frame time into fixed 60 Hz simulation updates. Catch-up
  time is limited to 250 ms per frame to avoid unbounded work after a stall.

Only the top state receives input and updates in this prototype. Drawing begins
at the highest opaque state and includes any transparent states above it.
Consequently, pausing keeps the maze visible but freezes its simulation.
Transitions are applied once per rendered frame, before simulation updates, and
reset accumulated time to avoid carrying old time into a new or resumed screen.
`Push` adds an overlay, `Pop` removes the top state, `Replace` changes the top
state, and `Reset` discards the stack and installs a new root. Removing the last
state exits the application.

The window is fixed at 800 by 600 pixels. Rendering remains directly in the
prototype screens; a shared renderer-facing API is deferred until its needs
are established.

### Maze rules

- Collect all pellets to win; each pellet scores 10 points.
- The player has three lives. Contact with an enemy costs one life. The final
  collision ends the game; otherwise actors return to their starting positions
  after a one-second ready interval. Score and collected pellets are preserved.
- Movement follows one grid direction until blocked or redirected. A requested
  turn remains buffered until an open junction allows it; the most recent input
  request replaces the pending turn. Direction presses are captured even in
  rendered frames without a simulation update, and held directions also work.
- The player moves one tile every 140 ms. Actor positions are interpolated
  between tiles for rendering.
- Two enemies pursue the player's position at the start of a movement step,
  using breadth-first search along open corridors. They move on alternate
  player movement steps, providing a speed advantage to the player.
- Collision checks include shared destinations and actors swapping adjacent
  tiles. Collision takes priority over pellet collection on the same step.
- Win and loss freeze gameplay. Restart creates a fresh game; returning to the
  menu discards the current game.

This slice deliberately omits power-ups, audio, multiple levels, and distinct
enemy personalities.

### Running and checking

From the project directory:

```sh
cargo run
cargo test
cargo build
```

Controls:

| Screen | Action | Keys |
|--------|--------|------|
| Main menu | Start game | Enter |
| Main menu | Quit | Escape |
| Maze | Turn | Arrow keys or WASD |
| Maze | Pause | Escape or P |
| Pause overlay | Resume | Escape, P, or Enter |
| Pause overlay | Return to main menu | M |
| Win/loss screen | Play again | Enter |
| Win/loss screen | Return to main menu | M or Escape |

Tests cover transition flow, update blocking under an overlay, drawing
visibility, fixed-step accumulation, input press semantics, maze connectivity,
buffered turns, wall blocking, pellet scoring, enemy navigation, collisions,
respawn, and game outcomes.
