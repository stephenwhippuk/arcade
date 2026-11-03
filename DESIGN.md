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
is an original maze-chase game; the second is an original formation shooter.
They exercise different needs:

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

- Which additional games or features should guide the next engine improvements.
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

Three playable vertical slices are available: Maze Chase, an original grid-based
game; Star Invaders, a formation shooter; and Key Cavern, a timed platform game.
All use original pixel-art sprites, primitive scenery, and original layouts
rather than copied maps or assets. The opening screen selects which game to launch.

- `main` owns the Macroquad window, input polling, and application loop.
- `input` maps keyboard keys to held and newly pressed semantic actions.
- `controls` defines separate validated gameplay profiles for each game and
  persists remappings in the user's configuration directory.
- `state` owns a stack of states and applies their requested transitions.
- `screens` implements game selection, all three game presentations, a shared pause
  overlay, and shared win/loss drawing.
- `maze` owns the map, movement, pellets, scoring, enemies, and game outcomes,
  independently of Macroquad.
- `invaders` owns the ship, formation, projectiles, collision checks, scoring,
  and shooter outcomes, independently of Macroquad.
- `platform` owns platform movement, gravity, jumping, keys, the exit, and air
  countdown, independently of Macroquad.
- `geometry` provides the rectangle and overlap query shared by the shooter and
  platform game, extracted from the original shooter implementation.
- `events` defines gameplay notifications for collection, shooting, jumping,
  hits, and win/loss. Rules emit these without depending on the audio backend.
- `audio` loads and plays original synthesized effects once at startup and
  applies a shared effects volume and mute setting.
- `renderer` defines the shared drawing interface and its Macroquad backend.
- `assets` validates embedded sprite sources and creates reusable GPU textures
  once at startup.
- `levels` reads and validates maze and platform JSON files at game start and
  restart. Each running game owns its loaded level data.
- `time` accumulates frame time into fixed 60 Hz simulation updates. Catch-up
  time is limited to 250 ms per frame to avoid unbounded work after a stall.

Only the top state receives input and updates in this prototype. Drawing begins
at the highest opaque state and includes any transparent states above it.
Consequently, pausing keeps the selected game visible but freezes its simulation.
Transitions are applied once per rendered frame, before simulation updates, and
reset accumulated time to avoid carrying old time into a new or resumed screen.
`Push` adds an overlay, `Pop` removes the top state, `Replace` changes the top
state, and `Reset` discards the stack and installs a new root. Removing the last
state exits the application.

The window is fixed at 800 by 600 pixels. Screens describe presentation through
the shared renderer interface rather than calling Macroquad drawing functions.
Game rules remain independent of rendering.

### Rendering and sprite assets

`Renderer` provides only the operations currently needed: text (with baseline
coordinates), filled rectangles, rectangle outlines, filled circles, and tinted
sprites. Rectangles and sprites use top-left coordinates and explicit destination
width/height in the existing 800 by 600 logical space. The interface uses
engine-owned colors, rectangle bounds, and sprite identifiers, not Macroquad
textures or vector types. Draw calls are immediate and ordered, preserving the
state stack's underlying-screen/overlay layering.

The Macroquad implementation owns one `Assets` collection. Seven original
8-by-8 monochrome masks live in `assets/sprites/`: orb, pursuer, ship, alien,
miner, key, and exit. Each `.sprite` file contains exactly eight rows of eight
characters: `#` is an opaque white pixel and `.` is transparent. This minimal
text representation keeps the first pixel art editable without an image editor
or another dependency; it is not intended as a general-purpose image format.
Sprites are tinted with the games' existing colors, including red/green for the
locked/open exit.

Sources are embedded at compile time, so launching from another directory does
not break asset lookup. Edit a source and rebuild to change a sprite. At startup,
all sources are validated before GPU resources are allocated. Errors name the
asset and appear in the terminal and the startup error screen. Missing source
files fail compilation. Loaded textures use nearest-neighbor sampling and are
reused across games, restarts, and overlays, rather than loaded in draw loops.

Actor sprites replace the earlier primitive actor drawings; walls, platforms,
projectiles, pellets, text, and overlays remain primitives. The ship sprite now
fits entirely inside the ship's existing collision rectangle, without the old
decorative protruding barrel. Maze actor sizes, interpolation, all collision
rules, controls, scores, and timers remain unchanged. Transparent areas in the
art are visual only; collision still uses the existing game-specific geometry.

Headless drawing tests use a recording implementation to inspect sprite
identifiers, positions, sizes, tints, HUD text, and overlays without a window.
PNG loading, animation sheets, cameras, resolution scaling, and hot reload are
deferred until needed.

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

### Star Invaders rules

- Move the ship horizontally and shoot upward to destroy a formation of 24
  invaders. Each destroyed invader scores 100 points; clearing the formation wins.
- The formation moves sideways, reverses and descends at the playfield edges,
  and speeds up as invaders are destroyed.
- The lowest surviving invader in each column can fire downward. Firing rotates
  through occupied columns every 900 ms.
- Hold Space to shoot, with a 300 ms cooldown. A brief press is remembered until
  the next simulation update, even if its rendered frame had no update.
- Projectile collision uses swept vertical bounds so a fast shot cannot skip
  over a target. An upward shot hits only the nearest intersected invader.
- The ship has three lives. One or more hits in the same update cost one life.
  Surviving a hit resets the ship and clears all projectiles, with a one-second
  ready interval during which simulation is frozen. Score and formation progress
  are preserved.
- Losing the last life or allowing the formation to reach ship height ends the
  game. Loss takes priority if the final life and final invader are lost together.
- Win/loss screens offer restart or return to selection. Returning from either
  game selects that game in the menu; restarting always restarts the same game.

This slice omits shields, audio, bonus ships, and additional waves. The games
share the application loop, semantic input, state stack, and UI flow but retain
separate world models. A common physics or entity framework is not required.

### Key Cavern rules

- Collect all five keys and enter the exit before the 60-second air supply runs
  out. Each key scores 100 points. Collecting the last key opens the exit but
  does not itself win the game.
- Move horizontally with Left/Right or A/D. Press Space, Up, or W to jump from
  a platform. Jump presses are remembered until the next simulation update;
  holding the key does not automatically repeat jumps, and airborne presses
  do not allow a second jump.
- Gravity and horizontal movement update at the shared fixed 60 Hz timestep.
  Solid platforms block sides and upward movement and catch downward falls.
  Walking off an edge causes the player to fall to a lower platform.
- The original single-screen level has a continuous floor and four raised
  platforms arranged in a zigzag. Jump from beside a higher platform, not
  directly beneath it, to avoid hitting its underside.
- Air is simulation time: pause freezes it along with movement. The HUD displays
  seconds remaining and an air bar, which turns red for the last ten seconds.
- Exhausting air ends the attempt immediately, taking priority over entering
  the exit on that update. Win and loss freeze the game. Restart restores every
  key, the full air supply, and the initial player position.
- Returning to selection keeps Key Cavern selected.

This slice focuses on movement, jumping, collection, and timing. It deliberately
omits enemies, spikes, extra lives, moving platforms, additional levels, and
audio. Its layout and graphics are original, not a reproduction of Manic Miner.

### Disk-loaded levels

Maze and platform layouts are stored in `assets/levels/maze.json` and
`assets/levels/platform.json`. Game selection and restart read the corresponding
file anew, so edits take effect without recompilation. A running or paused game
keeps its snapshot; editing the file does not change that attempt.
Invaders does not use these files.

By default, the loader uses the project's `assets/levels` directory, located
relative to the project path recorded at build time, not the current working
directory. For a relocated executable or a different level set, supply an
absolute directory with both JSON files:

```sh
ARCADE_LEVEL_DIR=/absolute/path/to/levels cargo run
```

These files must accompany a distributed executable, unlike embedded sprites.
There is no fallback layout. Read, parse, and validation errors name the file,
are logged to the terminal, and show a game-specific error screen. Fix the file
and press Enter to retry, or Escape/M to return to selection. One broken level
does not prevent playing the other games.

Both formats require `"version": 1`, reject unknown fields, and limit each file
to 64 KiB.

**Maze format**

- `rows`: exactly 15 strings of 19 ASCII cells each. `#` is a wall, `.` is a
  pellet corridor, and a space is an empty corridor.
- `player_start`: an object with zero-based integer `x` and `y` tile coordinates.
- `enemy_starts`: exactly two coordinate objects.

The outer boundary must be walls. Spawns must be distinct open cells, all open
cells must be reachable from the player spawn, and at least one pellet must
remain outside the spawn cells. Spawn cells never contain pellets. Movement,
wall checks, enemy pathfinding, rendering, and respawn all use the loaded map
and spawns. Maze dimensions and movement timing remain engine constraints for
this milestone.

**Platform format**

- `player_start`: player rectangle.
- `platforms`: array of solid rectangles.
- `keys`: array of collectible rectangles.
- `exit`: exit rectangle.
- `air_seconds`: positive initial air supply, at most 3600 seconds.

Rectangles have `x`, `y`, `width`, and `height`, in top-left screen coordinates.
Sizes must be positive and finite; rectangles must fit within the existing
playfield (`x: 40..760`, `y: 136..570`). There must be 1..128 platforms and keys,
including a continuous floor spanning the bottom edge. Platforms cannot overlap
one another or the spawn, keys, or exit. The spawn and exit must stand on a
platform. Keys cannot overlap each other or the exit.

Physics, collision, collection, player size, exit position, drawing, and the air
bar use loaded data. Movement speed, gravity, jump strength, scoring, and the
fixed playfield remain game rules rather than level settings. Validation catches
structural errors, not every possible unwinnable jump arrangement: edited
platform levels still need playtesting. The shipped level retains its exact
previous layout, and its real-movement completion test remains in place.

Multiple-level selection, progression, an editor, variable maze dimensions, and
live mutation of an active level are deferred.

### Input profiles and remapping

Each game has its own gameplay bindings. Maze Chase exposes four movement
directions and pause; Star Invaders exposes horizontal movement, fire, and pause;
Key Cavern exposes horizontal movement, jump, and pause. Defaults preserve the
original controls. Binding Space to fire in one game and jump in another is valid.
The active top state chooses its input profile; menus, overlays, and result
screens use fixed navigation instead of gameplay bindings. HUD hints show the
current bindings, not hard-coded defaults.

Press F3 from game selection to edit the selected game's controls, or from pause
to edit the paused game's controls. Up/Down selects an action, Enter begins
capture, and the next supported key replaces all aliases for that action.
Escape cancels capture; when not capturing, Escape or F3 returns to the previous
screen. R restores and saves only that game's defaults. Closing controls opened
from pause returns to pause, preserving gameplay and the air timer.

Supported inputs are individual letters, digits, arrow keys, Space, Tab, Shift,
and Control keys; key chords and other devices are not yet supported. Escape,
Enter, M, F1, and F3 are reserved for navigation and cannot be assigned.
A key cannot be assigned to multiple actions in the same game. Conflicts and
unsupported keys produce a visible error and leave the current binding unchanged.
While capturing, navigation keys are interpreted as candidate bindings, except
Escape which always cancels. Fixed pause-menu controls remain Escape/P/Enter to
resume, even when a game's pause key is remapped.

Successful edits and resets are saved immediately to
`$XDG_CONFIG_HOME/arcade/controls.json`, or `~/.config/arcade/controls.json` if
`XDG_CONFIG_HOME` is unset. The file has a version and explicit action/key-name
bindings for each game; serialized key names do not depend on numeric key codes.
On first run with no file, defaults are used. Existing malformed, unsupported,
or unreadable files are reported in the terminal and on the startup error screen
instead of silently resetting or overwriting the user's bindings. To recover
from a malformed file, fix it or move it aside to start with defaults.

Saves create a temporary file, flush it, and rename it over the destination.
Save errors are reported on the controls screen and in the terminal; active
bindings change only after a successful save. A conflicting temporary file is
not overwritten. Controls survive process restarts; audio settings remain
session-only. Remapping does not change held/pressed action semantics or gameplay
rules.

### Audio and settings

All three games have effects for their applicable gameplay events, plus menu
navigation sounds. Effects are generated as short PCM WAV tones in memory, so
no external audio assets or downloaded recordings are required. Screens drain
game events into the state stack; the application plays them after updates.
Events are drained before state replacement and never replayed from an old
frame. Changes to volume and mute are applied before playing that frame's events.

Press F1 from game selection or pause to open the audio settings overlay.
Left/Right changes effects volume in 10-percent steps, Enter toggles mute, and
Escape or F1 returns. Closing settings opened from pause returns to pause, not
to running gameplay. Settings apply to all games and survive game restarts and
menu navigation within the current process; they are not yet saved to disk.
Mute stops currently playing effects as well as suppressing new effects.

This milestone implements effects only; music and a separate music volume
control are deferred. Sound-loading errors are reported in the terminal and
on an error screen. Macroquad's native audio-device initialization is managed
by its backend and may fail before the application's loading handler runs.

On Ubuntu/Linux the audio-enabled build also needs ALSA development files:

```sh
sudo apt install libasound2-dev
```

### Running and checking

From the project directory:

```sh
cargo run
cargo test
cargo build
```

Gameplay keys below are defaults; remapped values appear in each game's HUD.
Menu and overlay controls remain fixed.

| Screen | Action | Keys |
|--------|--------|------|
| Game selection | Select game (wraps) | Up/Down or W/S |
| Game selection | Start selected game | Enter |
| Game selection | Quit | Escape |
| Game selection / pause | Audio settings | F1 |
| Game selection / pause | Controls for selected/current game | F3 |
| Controls screen | Select action | Up/Down or W/S |
| Controls screen | Begin binding capture | Enter |
| Controls screen | Restore this game's defaults | R |
| Controls screen | Cancel capture / return | Escape |
| Controls screen | Return when not capturing | F3 |
| Audio settings | Effects volume | Left/Right |
| Audio settings | Toggle mute | Enter |
| Audio settings | Return to previous screen | Escape or F1 |
| Maze | Turn | Arrow keys or WASD |
| Maze | Pause | Escape or P |
| Star Invaders | Move | Left/Right or A/D |
| Star Invaders | Fire | Space (hold to repeat) |
| Star Invaders | Pause | Escape or P |
| Key Cavern | Move | Left/Right or A/D |
| Key Cavern | Jump | Space, Up, or W (press) |
| Key Cavern | Pause | Escape or P |
| Pause overlay | Resume | Escape, P, or Enter |
| Pause overlay | Return to game selection | M |
| Win/loss screen | Play again | Enter |
| Win/loss screen | Return to game selection | M or Escape |

Tests cover transition flow, update blocking under an overlay, drawing
visibility, fixed-step accumulation, input press semantics, maze connectivity,
buffered turns, wall blocking, pellet scoring, enemy navigation, collisions,
respawn, and game outcomes. Shooter tests cover ship bounds, firing cooldown,
formation descent, nearest-hit swept collision, enemy firing, life loss,
projectile cleanup, and win/loss. Platform tests cover solid collisions, gravity,
grounded jumps, key collection, exit locking, air expiry, and completion of the
whole level using actual movement within the time budget. Screen tests verify
that pausing freezes air and that holding jump does not repeat it. Selection and
navigation tests cover all three games.
Audio tests cover PCM generation, volume bounds, mute, event draining,
settings lifetime, and returning from settings to pause without resuming.
Asset tests validate every shipped mask and reject malformed dimensions/pixels;
drawing tests verify all three games and shared overlays through the renderer.
Input tests cover profile isolation, held/pressed mapping, navigation safety,
conflict rejection, capture/cancel/reset, saved round trips, malformed files,
failed saves, and pause preservation while editing controls.
Level tests validate shipped layouts, reject malformed/unsafe data, confirm
that disk edits affect new games without changing active games, and cover
missing-file recovery and the level-error navigation screen.
