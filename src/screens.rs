use crate::input::{Action, Input};
use crate::controls::Game as GameChoice;
use crate::assets::Sprite;
use crate::geometry::Rect;
use crate::invaders::{self, Invaders};
use crate::maze::{Direction, Maze, Outcome, Tile, HEIGHT, WIDTH};
use crate::platform::{self, PlatformGame};
use crate::state::{State, Transition};
use crate::renderer::*;

impl GameChoice {
    fn start(self) -> Box<dyn State> {
        let result = (|| -> Result<Box<dyn State>, String> {
            match self {
                Self::Maze => {
                    let path = crate::levels::directory()?.join("maze.json");
                    let level = crate::levels::load_maze(&path)?;
                    Ok(Box::new(MazeScreen { game: Maze::new(level) }))
                }
                Self::Invaders => Ok(Box::new(InvadersScreen::default())),
                Self::Platform => {
                    let path = crate::levels::directory()?.join("platform.json");
                    let level = crate::levels::load_platform(&path)?;
                    Ok(Box::new(PlatformScreen { game: PlatformGame::new(level), jump_requested: false }))
                }
            }
        })();
        match result {
            Ok(state) => state,
            Err(error) => {
                eprintln!("{error}");
                Box::new(LevelErrorScreen { game: self, message: error })
            }
        }
    }
}

struct LevelErrorScreen {
    game: GameChoice,
    message: String,
}

impl State for LevelErrorScreen {
    fn handle_input(&mut self, input: &Input) -> Transition {
        if input.pressed(Action::Back) || input.pressed(Action::MainMenu) {
            Transition::Replace(Box::new(MainMenu { selected: self.game }))
        } else if input.pressed(Action::Confirm) {
            Transition::Replace(self.game.start())
        } else {
            Transition::None
        }
    }
    fn update(&mut self, _: f32, _: &Input) {}
    fn draw(&self, renderer: &mut dyn Renderer) {
        renderer.draw_text("Level could not be loaded", 50.0, 150.0, 36.0, RED);
        let characters: Vec<_> = self.message.chars().collect();
        for (line, text) in characters.chunks(80).enumerate().take(10) {
            renderer.draw_text(&text.iter().collect::<String>(), 50.0, 205.0 + line as f32 * 24.0, 18.0, YELLOW);
        }
        renderer.draw_text("Fix the file, then Enter: retry    Escape / M: selection",
            50.0, 530.0, 22.0, WHITE);
    }
}

#[derive(Default)]
pub struct MainMenu {
    selected: GameChoice,
}

impl State for MainMenu {
    fn controls_target(&self) -> Option<GameChoice> { Some(self.selected) }
    fn allows_settings(&self) -> bool { true }

    fn handle_input(&mut self, input: &Input) -> Transition {
        if input.pressed(Action::Back) {
            return Transition::Pop;
        }
        if input.pressed(Action::MoveUp) {
            self.selected = match self.selected {
                GameChoice::Maze => GameChoice::Platform,
                GameChoice::Invaders => GameChoice::Maze,
                GameChoice::Platform => GameChoice::Invaders,
            };
        } else if input.pressed(Action::MoveDown) {
            self.selected = match self.selected {
                GameChoice::Maze => GameChoice::Invaders,
                GameChoice::Invaders => GameChoice::Platform,
                GameChoice::Platform => GameChoice::Maze,
            };
        }
        if input.pressed(Action::Confirm) {
            return Transition::Replace(self.selected.start());
        }
        Transition::None
    }

    fn update(&mut self, _: f32, _: &Input) {}

    fn draw(&self, renderer: &mut dyn Renderer) {
        renderer.draw_text("Arcade", 80.0, 160.0, 64.0, WHITE);
        renderer.draw_text("Choose a game", 80.0, 210.0, 28.0, LIGHTGRAY);
        for (choice, title, y) in [
            (GameChoice::Maze, "Maze Chase", 280.0),
            (GameChoice::Invaders, "Star Invaders", 340.0),
            (GameChoice::Platform, "Key Cavern", 400.0),
        ] {
            let selected = choice == self.selected;
            if selected {
                renderer.draw_text(">", 80.0, y, 32.0, GREEN);
            }
            renderer.draw_text(title, 120.0, y, 32.0, if selected { GREEN } else { LIGHTGRAY });
        }
        let description = match self.selected {
            GameChoice::Maze => "Collect every pellet. Avoid the pursuers.",
            GameChoice::Invaders => "Clear the formation before it reaches your ship.",
            GameChoice::Platform => "Collect all keys and reach the exit before air runs out.",
        };
        renderer.draw_text(description, 80.0, 450.0, 23.0, LIGHTGRAY);
        renderer.draw_text("Up / Down / W / S: select    Enter: play", 80.0, 500.0, 24.0, WHITE);
        renderer.draw_text("Escape: quit    F1: audio    F3: controls", 80.0, 545.0, 24.0, LIGHTGRAY);
    }
}

#[cfg_attr(test, derive(Default))]
struct MazeScreen {
    game: Maze,
}

impl State for MazeScreen {
    fn input_profile(&self) -> Option<GameChoice> {
        (self.game.outcome == Outcome::Playing).then_some(GameChoice::Maze)
    }
    fn drain_events(&mut self) -> Vec<crate::events::GameEvent> {
        std::mem::take(&mut self.game.events)
    }

    fn handle_input(&mut self, input: &Input) -> Transition {
        if self.game.outcome != Outcome::Playing {
            if input.pressed(Action::MainMenu) || input.pressed(Action::Back) {
                return Transition::Reset(Box::new(MainMenu::default()));
            }
            if input.pressed(Action::Confirm) {
                return Transition::Replace(GameChoice::Maze.start());
            }
            return Transition::None;
        }
        if input.pressed(Action::Back) || input.pressed(Action::Pause) {
            return Transition::Push(Box::new(PauseMenu { game: GameChoice::Maze }));
        }
        let directions = [
            (Action::MoveLeft, Direction::Left),
            (Action::MoveRight, Direction::Right),
            (Action::MoveUp, Direction::Up),
            (Action::MoveDown, Direction::Down),
        ];
        // Store turns during input handling even when this frame has no simulation tick.
        let requested = directions.iter().find(|&&(action, _)| input.pressed(action))
            .or_else(|| directions.iter().find(|&&(action, _)| input.held(action)));
        if let Some(&(_, direction)) = requested {
            self.game.request_direction(direction);
        }
        Transition::None
    }

    fn update(&mut self, dt: f32, _: &Input) {
        self.game.update(dt);
    }

    fn draw(&self, renderer: &mut dyn Renderer) {
        const CELL: f32 = 28.0;
        const LEFT: f32 = (800.0 - WIDTH as f32 * CELL) / 2.0;
        const TOP: f32 = 136.0;
        renderer.draw_text("Maze Chase", 32.0, 40.0, 32.0, WHITE);
        renderer.draw_text(
            &format!(
                "Score: {}    Lives: {}    Pellets: {}",
                self.game.score, self.game.lives, self.game.remaining
            ),
            32.0,
            72.0,
            24.0,
            LIGHTGRAY,
        );
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let tile = Tile { x, y };
                let left = LEFT + x as f32 * CELL;
                let top = TOP + y as f32 * CELL;
                if self.game.level.is_wall(tile) {
                    renderer.draw_rectangle(left + 1.0, top + 1.0, CELL - 2.0, CELL - 2.0, DARKBLUE);
                } else if self.game.has_pellet(tile) {
                    renderer.draw_circle(left + CELL / 2.0, top + CELL / 2.0, 3.0, WHITE);
                }
            }
        }
        let fraction = self.game.animation_fraction();
        let (x, y) = self.game.player.position(fraction);
        renderer.draw_sprite(Sprite::Orb, Rect {
            x: LEFT + (x + 0.5) * CELL - 10.0,
            y: TOP + (y + 0.5) * CELL - 10.0,
            width: 20.0, height: 20.0,
        }, YELLOW);
        for (enemy, color) in self.game.enemies.iter().zip([RED, SKYBLUE]) {
            let (x, y) = enemy.position(fraction);
            renderer.draw_sprite(Sprite::Pursuer, Rect {
                x: LEFT + (x + 0.5) * CELL - 9.0,
                y: TOP + (y + 0.5) * CELL - 9.0,
                width: 18.0, height: 18.0,
            }, color);
        }
        if self.game.is_respawning() {
            renderer.draw_text("Caught! Get ready...", 270.0, 590.0, 24.0, YELLOW);
        }
        if self.game.outcome != Outcome::Playing {
            let title = if self.game.outcome == Outcome::Won { "Maze cleared!" } else { "Game over" };
            draw_result(renderer, title, self.game.score);
        }
    }
}

#[derive(Default)]
struct InvadersScreen {
    game: Invaders,
    fire_requested: bool,
}

impl State for InvadersScreen {
    fn input_profile(&self) -> Option<GameChoice> {
        (self.game.outcome == invaders::Outcome::Playing).then_some(GameChoice::Invaders)
    }
    fn drain_events(&mut self) -> Vec<crate::events::GameEvent> {
        std::mem::take(&mut self.game.events)
    }

    fn handle_input(&mut self, input: &Input) -> Transition {
        if self.game.outcome != invaders::Outcome::Playing {
            if input.pressed(Action::MainMenu) || input.pressed(Action::Back) {
                return Transition::Reset(Box::new(MainMenu { selected: GameChoice::Invaders }));
            }
            if input.pressed(Action::Confirm) {
                return Transition::Replace(Box::new(Self::default()));
            }
            return Transition::None;
        }
        if input.pressed(Action::Back) || input.pressed(Action::Pause) {
            return Transition::Push(Box::new(PauseMenu { game: GameChoice::Invaders }));
        }
        self.fire_requested |= input.pressed(Action::Fire);
        Transition::None
    }

    fn update(&mut self, dt: f32, input: &Input) {
        self.game.update(
            dt,
            input.held(Action::MoveLeft),
            input.held(Action::MoveRight),
            self.fire_requested || input.held(Action::Fire),
        );
        self.fire_requested = false;
    }

    fn draw(&self, renderer: &mut dyn Renderer) {
        renderer.draw_text("Star Invaders", 32.0, 40.0, 32.0, WHITE);
        renderer.draw_text(
            &format!("Score: {}    Lives: {}    Invaders: {}",
                self.game.score, self.game.lives, self.game.aliens.len()),
            32.0, 72.0, 24.0, LIGHTGRAY,
        );
        renderer.draw_rectangle_lines(Rect {
            x: invaders::LEFT, y: invaders::TOP,
            width: invaders::RIGHT - invaders::LEFT,
            height: invaders::BOTTOM - invaders::TOP,
        }, 2.0, DARKGRAY);
        let player = self.game.player;
        renderer.draw_sprite(Sprite::Ship, player, GREEN);
        for alien in &self.game.aliens {
            let rect = alien.bounds;
            renderer.draw_sprite(Sprite::Alien, rect, SKYBLUE);
        }
        for (shots, color) in [
            (&self.game.player_shots, YELLOW),
            (&self.game.enemy_shots, RED),
        ] {
            for shot in shots {
                renderer.draw_rectangle(shot.x, shot.y, shot.width, shot.height, color);
            }
        }
        if self.game.is_respawning() {
            renderer.draw_text("Ship hit! Get ready...", 270.0, 595.0, 24.0, YELLOW);
        }
        if self.game.outcome != invaders::Outcome::Playing {
            let title = if self.game.outcome == invaders::Outcome::Won {
                "Formation cleared!"
            } else {
                "Game over"
            };
            draw_result(renderer, title, self.game.score);
        }
    }
}

#[cfg_attr(test, derive(Default))]
struct PlatformScreen {
    game: PlatformGame,
    jump_requested: bool,
}

impl State for PlatformScreen {
    fn input_profile(&self) -> Option<GameChoice> {
        (self.game.outcome == platform::Outcome::Playing).then_some(GameChoice::Platform)
    }
    fn drain_events(&mut self) -> Vec<crate::events::GameEvent> {
        std::mem::take(&mut self.game.events)
    }

    fn handle_input(&mut self, input: &Input) -> Transition {
        if self.game.outcome != platform::Outcome::Playing {
            if input.pressed(Action::MainMenu) || input.pressed(Action::Back) {
                return Transition::Reset(Box::new(MainMenu { selected: GameChoice::Platform }));
            }
            if input.pressed(Action::Confirm) {
                return Transition::Replace(GameChoice::Platform.start());
            }
            return Transition::None;
        }
        if input.pressed(Action::Back) || input.pressed(Action::Pause) {
            return Transition::Push(Box::new(PauseMenu { game: GameChoice::Platform }));
        }
        self.jump_requested |= input.pressed(Action::Jump);
        Transition::None
    }

    fn update(&mut self, dt: f32, input: &Input) {
        self.game.update(
            dt,
            input.held(Action::MoveLeft),
            input.held(Action::MoveRight),
            self.jump_requested,
        );
        self.jump_requested = false;
    }

    fn draw(&self, renderer: &mut dyn Renderer) {
        renderer.draw_text("Key Cavern", 32.0, 40.0, 32.0, WHITE);
        renderer.draw_text(
            &format!("Score: {}    Keys left: {}    Air: {:.0}s",
                self.game.score, self.game.keys.len(), self.game.air.ceil()),
            32.0, 72.0, 24.0, LIGHTGRAY,
        );
        let air_color = if self.game.air <= 10.0 { RED } else { SKYBLUE };
        renderer.draw_rectangle(40.0, 117.0, 720.0, 8.0, DARKGRAY);
        renderer.draw_rectangle(40.0, 117.0, 720.0 * (self.game.air / self.game.level.air_seconds) as f32,
            8.0, air_color);
        renderer.draw_rectangle_lines(Rect {
            x: platform::LEFT, y: platform::TOP,
            width: platform::RIGHT - platform::LEFT,
            height: platform::BOTTOM - platform::TOP,
        }, 2.0, DARKGRAY);
        for surface in &self.game.level.platforms {
            renderer.draw_rectangle(surface.x, surface.y, surface.width, surface.height, DARKBLUE);
            renderer.draw_rectangle(surface.x, surface.y, surface.width, 3.0, SKYBLUE);
        }
        for key in &self.game.keys {
            renderer.draw_sprite(Sprite::Key, *key, GOLD);
        }
        let exit = self.game.level.exit;
        let exit_color = if self.game.exit_open() { GREEN } else { RED };
        renderer.draw_sprite(Sprite::Exit, exit, exit_color);
        renderer.draw_text(if self.game.exit_open() { "OPEN" } else { "LOCKED" },
            exit.x - 12.0, exit.y - 8.0, 16.0, exit_color);
        let player = self.game.player;
        renderer.draw_sprite(Sprite::Miner, player, YELLOW);
        renderer.draw_text("Collect every key, then enter the green exit.", 160.0, 595.0, 22.0, LIGHTGRAY);
        if self.game.outcome != platform::Outcome::Playing {
            draw_result(renderer,
                if self.game.outcome == platform::Outcome::Won { "Cavern escaped!" } else { "Out of air!" },
                self.game.score,
            );
        }
    }
}

fn draw_result(renderer: &mut dyn Renderer, title: &str, score: u32) {
    renderer.draw_rectangle(0.0, 0.0, 800.0, 600.0, Color::new(0.0, 0.0, 0.0, 0.8));
    renderer.draw_text(title, 180.0, 235.0, 48.0, WHITE);
    renderer.draw_text(&format!("Final score: {score}"), 230.0, 290.0, 28.0, YELLOW);
    renderer.draw_text("Enter: play again", 230.0, 350.0, 26.0, GREEN);
    renderer.draw_text("M / Escape: game selection", 230.0, 395.0, 26.0, LIGHTGRAY);
}

struct PauseMenu {
    game: GameChoice,
}

impl State for PauseMenu {
    fn controls_target(&self) -> Option<GameChoice> { Some(self.game) }
    fn allows_settings(&self) -> bool { true }

    fn handle_input(&mut self, input: &Input) -> Transition {
        if input.pressed(Action::MainMenu) {
            Transition::Reset(Box::new(MainMenu { selected: self.game }))
        } else if input.pressed(Action::Confirm)
            || input.pressed(Action::Back)
            || input.pressed(Action::Pause)
        {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn update(&mut self, _: f32, _: &Input) {}

    fn draw(&self, renderer: &mut dyn Renderer) {
        renderer.draw_rectangle(0.0, 0.0, 800.0, 600.0, Color::new(0.0, 0.0, 0.0, 0.75));
        renderer.draw_text("Paused", 240.0, 220.0, 48.0, WHITE);
        renderer.draw_text("Escape / P / Enter: resume", 240.0, 285.0, 26.0, GREEN);
        renderer.draw_text("M: game selection", 240.0, 375.0, 26.0, LIGHTGRAY);
        renderer.draw_text("F1: audio    F3: controls", 240.0, 420.0, 26.0, LIGHTGRAY);
    }

    fn is_transparent(&self) -> bool {
        true
    }
}

pub struct AudioSettingsScreen {
    settings: std::rc::Rc<std::cell::Cell<crate::audio::AudioSettings>>,
    events: Vec<crate::events::GameEvent>,
}

impl AudioSettingsScreen {
    pub fn new(settings: std::rc::Rc<std::cell::Cell<crate::audio::AudioSettings>>) -> Self {
        Self { settings, events: Vec::new() }
    }
}

impl State for AudioSettingsScreen {
    fn drain_events(&mut self) -> Vec<crate::events::GameEvent> {
        std::mem::take(&mut self.events)
    }

    fn handle_input(&mut self, input: &Input) -> Transition {
        if input.pressed(Action::Back) || input.pressed(Action::Settings) {
            return Transition::Pop;
        }
        let mut settings = self.settings.get();
        if input.pressed(Action::MoveLeft) {
            settings.adjust(false);
        } else if input.pressed(Action::MoveRight) {
            settings.adjust(true);
        }
        if input.pressed(Action::Confirm) {
            settings.muted = !settings.muted;
        }
        if settings != self.settings.get() {
            self.events.push(crate::events::GameEvent::Menu);
        }
        self.settings.set(settings);
        Transition::None
    }

    fn update(&mut self, _: f32, _: &Input) {}

    fn draw(&self, renderer: &mut dyn Renderer) {
        let settings = self.settings.get();
        renderer.draw_rectangle(0.0, 0.0, 800.0, 600.0, Color::new(0.0, 0.0, 0.0, 0.9));
        renderer.draw_text("Audio settings", 190.0, 210.0, 44.0, WHITE);
        renderer.draw_text(&format!("Effects volume: {}%", settings.volume), 190.0, 275.0, 28.0, SKYBLUE);
        renderer.draw_text(if settings.muted { "Muted: yes" } else { "Muted: no" }, 190.0, 325.0, 28.0, YELLOW);
        renderer.draw_text("Left / Right: volume    Enter: mute", 190.0, 395.0, 24.0, LIGHTGRAY);
        renderer.draw_text("Escape / F1: return", 190.0, 445.0, 24.0, LIGHTGRAY);
    }

    fn is_transparent(&self) -> bool { true }
}

pub struct ControlsScreen {
    game: GameChoice,
    store: std::rc::Rc<std::cell::RefCell<crate::controls::ControlStore>>,
    selected: usize,
    waiting: bool,
    message: String,
}

impl ControlsScreen {
    pub fn new(game: GameChoice, store: std::rc::Rc<std::cell::RefCell<crate::controls::ControlStore>>) -> Self {
        Self { game, store, selected: 0, waiting: false, message: String::new() }
    }

    fn save(&mut self, candidate: crate::controls::Controls) {
        match self.store.borrow_mut().save(candidate) {
            Ok(()) => {
                self.waiting = false;
                self.message = "Saved. Gameplay controls updated.".to_owned();
            }
            Err(error) => {
                eprintln!("{error}");
                self.message = format!("Save failed: {error}");
            }
        }
    }
}

impl State for ControlsScreen {
    fn handle_input(&mut self, input: &Input) -> Transition {
        if input.pressed(Action::Back) {
            if self.waiting {
                self.waiting = false;
                self.message = "Binding cancelled.".to_owned();
                return Transition::None;
            }
            return Transition::Pop;
        }
        if self.waiting {
            if input.raw_pressed.len() > 1 {
                self.message = "Press one key at a time; combinations are not supported.".to_owned();
            } else if let Some(&key) = input.raw_pressed.first() {
                if let Some(name) = crate::controls::key_name(key) {
                    let mut candidate = self.store.borrow().controls.clone();
                    match candidate.rebind(self.game, self.game.actions()[self.selected], name) {
                        Ok(()) => self.save(candidate),
                        Err(error) => {
                            eprintln!("{error}");
                            self.message = error;
                        }
                    }
                } else {
                    self.message = "Unsupported key. Use letters, digits, arrows, Space, Tab, Shift or Ctrl.".to_owned();
                }
            }
            return Transition::None;
        }
        if input.pressed(Action::Controls) {
            return Transition::Pop;
        }
        if input.pressed(Action::MoveUp) {
            self.selected = (self.selected + self.game.actions().len() - 1) % self.game.actions().len();
        } else if input.pressed(Action::MoveDown) {
            self.selected = (self.selected + 1) % self.game.actions().len();
        } else if input.pressed(Action::Confirm) {
            self.waiting = true;
            self.message = "Press a new key (Escape cancels). This replaces all aliases for this action.".to_owned();
        } else if input.pressed(Action::ResetControls) {
            let mut candidate = self.store.borrow().controls.clone();
            candidate.reset(self.game);
            self.save(candidate);
        }
        Transition::None
    }

    fn update(&mut self, _: f32, _: &Input) {}

    fn draw(&self, renderer: &mut dyn Renderer) {
        renderer.draw_rectangle(0.0, 0.0, 800.0, 600.0, Color::new(0.0, 0.0, 0.0, 0.95));
        renderer.draw_text(&format!("Controls: {}", self.game.title()), 65.0, 90.0, 36.0, WHITE);
        let store = self.store.borrow();
        for (index, &action) in self.game.actions().iter().enumerate() {
            let binding = store.controls.bindings(self.game).iter()
                .find(|binding| binding.action == action).expect("Every game action has a binding");
            let tint = if self.selected == index { GREEN } else { LIGHTGRAY };
            renderer.draw_text(&format!("{} {}: {}", if index == self.selected { ">" } else { " " },
                action.label(), binding.keys.join(" / ")), 65.0, 155.0 + index as f32 * 48.0, 26.0, tint);
        }
        renderer.draw_text("Up/Down: select   Enter: rebind   R: restore this game's defaults", 65.0, 430.0, 20.0, WHITE);
        renderer.draw_text("Escape/F3: return   Escape while capturing: cancel", 65.0, 465.0, 20.0, LIGHTGRAY);
        renderer.draw_text("Reserved: Escape, Enter, M, F1, F3. Keys cannot conflict within a game.", 65.0, 495.0, 18.0, LIGHTGRAY);
        let characters: Vec<_> = self.message.chars().collect();
        for (line, text) in characters.chunks(85).enumerate().take(3) {
            renderer.draw_text(&text.iter().collect::<String>(), 65.0, 525.0 + line as f32 * 22.0, 18.0, YELLOW);
        }
    }

    fn is_transparent(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StateStack;
    use crate::time::STEP_SECONDS;

    #[test]
    fn turn_is_buffered_before_a_simulation_tick() {
        let mut screen = MazeScreen::default();
        screen.handle_input(&Input::for_test(&[], &[Action::MoveRight]));
        screen.update(0.14, &Input::default());
        assert_eq!(screen.game.player.tile, Tile { x: 2, y: 1 });
    }

    #[test]
    fn end_screen_supports_restart_and_menu() {
        let mut screen = MazeScreen::default();
        screen.game.outcome = Outcome::Won;
        assert!(matches!(
            screen.handle_input(&Input::for_test(&[], &[Action::Confirm])),
            Transition::Replace(_)
        ));
        screen.game.outcome = Outcome::Lost;
        assert!(matches!(
            screen.handle_input(&Input::for_test(&[], &[Action::Back])),
            Transition::Reset(_)
        ));
    }

    #[test]
    fn menu_game_pause_resume_menu_and_quit_flow() {
        let mut stack = StateStack::new(Box::new(MainMenu::default()));
        let press = |action| Input::for_test(&[], &[action]);
        assert!(stack.handle_input(&press(Action::Confirm)));
        stack.update(STEP_SECONDS, &Input::default());
        assert!(stack.handle_input(&press(Action::Pause)));
        assert!(!stack.handle_input(&Input::for_test(&[Action::Pause], &[])));
        assert!(stack.handle_input(&press(Action::Pause)));
        assert!(stack.handle_input(&press(Action::Back)));
        assert!(stack.handle_input(&press(Action::MainMenu)));
        assert!(stack.handle_input(&press(Action::Back)));
        assert!(stack.is_empty());
    }

    #[test]
    fn selection_wraps_in_both_directions_and_launches_all_games() {
        let mut menu = MainMenu::default();
        let press = |action| Input::for_test(&[], &[action]);
        assert_eq!(menu.selected, GameChoice::Maze);
        menu.handle_input(&press(Action::MoveUp));
        assert_eq!(menu.selected, GameChoice::Platform);
        menu.handle_input(&press(Action::MoveDown));
        assert_eq!(menu.selected, GameChoice::Maze);
        for choice in [GameChoice::Maze, GameChoice::Invaders, GameChoice::Platform] {
            assert_eq!(menu.selected, choice);
            assert!(matches!(menu.handle_input(&press(Action::Confirm)), Transition::Replace(_)));
            menu.handle_input(&Input::for_test(&[Action::MoveDown], &[]));
            assert_eq!(menu.selected, choice);
            menu.handle_input(&press(Action::MoveDown));
        }
        assert_eq!(menu.selected, GameChoice::Maze);
        menu.handle_input(&press(Action::MoveUp));
        menu.handle_input(&press(Action::MoveUp));
        assert_eq!(menu.selected, GameChoice::Invaders);
    }

    #[test]
    fn invaders_fire_press_survives_a_frame_without_an_update() {
        let mut screen = InvadersScreen::default();
        screen.handle_input(&Input::for_test(&[], &[Action::Fire]));
        screen.handle_input(&Input::default());
        screen.update(STEP_SECONDS, &Input::default());
        assert_eq!(screen.game.player_shots.len(), 1);
        assert!(!screen.fire_requested);
    }

    #[test]
    fn invaders_selection_pause_resume_return_and_quit_flow() {
        let mut stack = StateStack::new(Box::new(MainMenu::default()));
        let press = |action| Input::for_test(&[], &[action]);
        assert!(!stack.handle_input(&press(Action::MoveDown)));
        assert!(stack.handle_input(&press(Action::Confirm)));
        stack.update(STEP_SECONDS, &Input::for_test(&[Action::Fire], &[]));
        assert!(stack.handle_input(&press(Action::Pause)));
        assert!(stack.handle_input(&press(Action::Confirm)));
        assert!(stack.handle_input(&press(Action::Pause)));
        assert!(stack.handle_input(&press(Action::MainMenu)));
        assert!(stack.handle_input(&press(Action::Back)));
        assert!(stack.is_empty());
    }

    #[test]
    fn invaders_end_screen_restarts_or_returns_to_selection() {
        let mut screen = InvadersScreen::default();
        for outcome in [invaders::Outcome::Won, invaders::Outcome::Lost] {
            screen.game.outcome = outcome;
            assert!(matches!(
                screen.handle_input(&Input::for_test(&[], &[Action::Confirm])),
                Transition::Replace(_)
            ));
            assert!(matches!(
                screen.handle_input(&Input::for_test(&[], &[Action::MainMenu])),
                Transition::Reset(_)
            ));
        }
    }

    #[test]
    fn jump_press_is_preserved_without_a_tick_but_holding_does_not_repeat() {
        let mut screen = PlatformScreen::default();
        screen.handle_input(&Input::for_test(&[], &[Action::Jump]));
        screen.handle_input(&Input::default());
        screen.update(STEP_SECONDS, &Input::default());
        assert!(screen.game.player.y < 522.0);
        assert!(!screen.jump_requested);
        for _ in 0..90 {
            let held = Input::for_test(&[Action::Jump], &[]);
            screen.handle_input(&held);
            screen.update(STEP_SECONDS, &held);
        }
        assert_eq!(screen.game.player.y, 522.0);
    }

    #[test]
    fn platform_pause_freezes_air_and_resume_preserves_game() {
        use std::cell::RefCell;
        use std::rc::Rc;

        struct Observed(Rc<RefCell<PlatformScreen>>);
        impl State for Observed {
            fn handle_input(&mut self, input: &Input) -> Transition {
                self.0.borrow_mut().handle_input(input)
            }
            fn update(&mut self, dt: f32, input: &Input) {
                self.0.borrow_mut().update(dt, input);
            }
            fn draw(&self, _: &mut dyn Renderer) {}
        }

        let screen = Rc::new(RefCell::new(PlatformScreen::default()));
        let mut stack = StateStack::new(Box::new(Observed(Rc::clone(&screen))));
        stack.update(STEP_SECONDS, &Input::default());
        let air = screen.borrow().game.air;
        let press = |action| Input::for_test(&[], &[action]);
        assert!(stack.handle_input(&press(Action::Pause)));
        assert!(stack.handle_input(&press(Action::Settings)));
        for _ in 0..120 {
            stack.update(STEP_SECONDS, &Input::default());
        }
        assert_eq!(screen.borrow().game.air, air);
        assert!(stack.handle_input(&press(Action::Back)));
        stack.update(STEP_SECONDS, &Input::default());
        assert_eq!(screen.borrow().game.air, air);
        assert!(stack.handle_input(&press(Action::Confirm)));
        stack.update(STEP_SECONDS, &Input::default());
        assert!(screen.borrow().game.air < air);
        assert!(stack.handle_input(&press(Action::Pause)));
        assert!(stack.handle_input(&press(Action::MainMenu)));
        assert!(stack.handle_input(&press(Action::Confirm)));
        assert!(stack.handle_input(&press(Action::Pause)));
        assert!(stack.handle_input(&press(Action::MainMenu)));
        assert!(stack.handle_input(&press(Action::Back)));
        assert!(stack.is_empty());
    }

    #[test]
    fn platform_results_offer_fresh_restart_and_selection() {
        let mut screen = PlatformScreen::default();
        for outcome in [platform::Outcome::Won, platform::Outcome::Lost] {
            screen.game.outcome = outcome;
            assert!(matches!(
                screen.handle_input(&Input::for_test(&[], &[Action::Confirm])),
                Transition::Replace(_)
            ));
            assert!(matches!(
                screen.handle_input(&Input::for_test(&[], &[Action::Back])),
                Transition::Reset(_)
            ));
        }
        let fresh = PlatformScreen::default();
        assert_eq!(fresh.game.air, platform::AIR_SECONDS);
        assert_eq!(fresh.game.keys.len(), 5);
        assert_eq!(fresh.game.score, 0);
    }

    #[test]
    fn game_sounds_are_reported_once_and_do_not_repeat_on_idle_updates() {
        use crate::events::GameEvent;

        let mut maze = MazeScreen::default();
        maze.handle_input(&Input::for_test(&[], &[Action::MoveRight]));
        maze.update(0.14, &Input::default());
        assert_eq!(maze.drain_events(), vec![GameEvent::Collect]);
        assert!(maze.drain_events().is_empty());

        let mut shooter = InvadersScreen::default();
        shooter.update(STEP_SECONDS, &Input::for_test(&[Action::Fire], &[]));
        assert_eq!(shooter.drain_events(), vec![GameEvent::Shoot]);
        shooter.update(STEP_SECONDS, &Input::default());
        assert!(shooter.drain_events().is_empty());

        let mut platform = PlatformScreen::default();
        platform.handle_input(&Input::for_test(&[], &[Action::Jump]));
        platform.update(STEP_SECONDS, &Input::default());
        assert_eq!(platform.drain_events(), vec![GameEvent::Jump]);
        platform.update(STEP_SECONDS, &Input::default());
        assert!(platform.drain_events().is_empty());
    }

    #[test]
    fn settings_preserve_volume_and_return_to_pause_before_resuming() {
        let press = |action| Input::for_test(&[], &[action]);
        let mut stack = StateStack::new(Box::new(MainMenu::default()));
        assert!(stack.handle_input(&press(Action::Settings)));
        assert!(!stack.handle_input(&press(Action::MoveLeft)));
        assert_eq!(stack.audio_settings().volume, 50);
        assert!(!stack.handle_input(&press(Action::Confirm)));
        assert!(stack.audio_settings().muted);
        assert!(stack.handle_input(&press(Action::Back)));
        assert!(stack.handle_input(&press(Action::Confirm)));
        assert!(!stack.handle_input(&press(Action::Settings)));
        assert!(stack.handle_input(&press(Action::Pause)));
        assert!(stack.handle_input(&press(Action::Settings)));
        assert_eq!(stack.audio_settings().volume, 50);
        assert!(stack.audio_settings().muted);
        assert!(stack.handle_input(&press(Action::Back)));
        // Confirm resumes pause; a second pause press must now open it again.
        assert!(stack.handle_input(&press(Action::Confirm)));
        assert!(stack.handle_input(&press(Action::Pause)));
        assert!(stack.handle_input(&press(Action::MainMenu)));
        assert!(stack.audio_settings().muted);
    }

    #[test]
    fn maze_renders_interpolated_actors_with_their_existing_sizes_and_tints() {
        let screen = MazeScreen::default();
        let mut renderer = RecordingRenderer::default();
        screen.draw(&mut renderer);
        assert_eq!(renderer.sprites.len(), 3);
        let (sprite, bounds, tint) = renderer.sprites[0];
        assert_eq!(sprite, Sprite::Orb);
        assert_eq!((bounds.x, bounds.y, bounds.width, bounds.height), (166.0, 168.0, 20.0, 20.0));
        assert_eq!(tint, YELLOW);
        assert_eq!(renderer.sprites[1].0, Sprite::Pursuer);
        assert_eq!(renderer.sprites[1].2, RED);
        assert_eq!(renderer.sprites[2].2, SKYBLUE);
        assert!(renderer.texts.iter().any(|text| text.starts_with("Score:")));
    }

    #[test]
    fn shooter_sprite_bounds_match_the_actual_entities() {
        let screen = InvadersScreen::default();
        let mut renderer = RecordingRenderer::default();
        screen.draw(&mut renderer);
        assert_eq!(renderer.sprites.len(), 25);
        assert_eq!(renderer.sprites[0].0, Sprite::Ship);
        assert_eq!(renderer.sprites[0].2, GREEN);
        let bounds = renderer.sprites[0].1;
        let player = screen.game.player;
        assert_eq!((bounds.x, bounds.y, bounds.width, bounds.height),
            (player.x, player.y, player.width, player.height));
        for (drawn, alien) in renderer.sprites[1..].iter().zip(&screen.game.aliens) {
            assert_eq!(drawn.0, Sprite::Alien);
            assert_eq!((drawn.1.x, drawn.1.y, drawn.1.width, drawn.1.height),
                (alien.bounds.x, alien.bounds.y, alien.bounds.width, alien.bounds.height));
        }
    }

    #[test]
    fn platform_draws_remaining_keys_and_changes_exit_tint_when_unlocked() {
        let mut screen = PlatformScreen::default();
        let mut renderer = RecordingRenderer::default();
        screen.draw(&mut renderer);
        assert_eq!(renderer.sprites.len(), 7);
        assert_eq!(renderer.sprites.iter().filter(|draw| draw.0 == Sprite::Key).count(), 5);
        assert_eq!(renderer.sprites[5].0, Sprite::Exit);
        assert_eq!(renderer.sprites[5].2, RED);
        assert_eq!(renderer.sprites[6].0, Sprite::Miner);
        screen.game.keys.clear();
        let mut renderer = RecordingRenderer::default();
        screen.draw(&mut renderer);
        assert_eq!(renderer.sprites.len(), 2);
        assert_eq!(renderer.sprites[0].0, Sprite::Exit);
        assert_eq!(renderer.sprites[0].2, GREEN);
        assert!(renderer.texts.iter().any(|text| text == "OPEN"));
    }

    #[test]
    fn shared_overlays_render_through_the_same_interface() {
        let mut renderer = RecordingRenderer::default();
        draw_result(&mut renderer, "Game over", 500);
        PauseMenu { game: GameChoice::Maze }.draw(&mut renderer);
        AudioSettingsScreen::new(std::rc::Rc::new(std::cell::Cell::new(
            crate::audio::AudioSettings::default(),
        ))).draw(&mut renderer);
        assert!(renderer.texts.contains(&"Final score: 500".to_owned()));
        assert!(renderer.texts.contains(&"Paused".to_owned()));
        assert!(renderer.texts.contains(&"Audio settings".to_owned()));
        assert!(renderer.sprites.is_empty());
    }

    #[test]
    fn controls_capture_cancel_conflict_save_and_reset_are_transactional() {
        use crate::controls::{ControlStore, Controls};
        use macroquad::prelude::KeyCode;
        let directory = crate::controls::tests::TestDirectory::new();
        let path = directory.0.join("controls.json");
        let store = std::rc::Rc::new(std::cell::RefCell::new(ControlStore::load(path.clone()).unwrap()));
        let mut screen = ControlsScreen::new(GameChoice::Platform, std::rc::Rc::clone(&store));
        let keys = |pressed: &[KeyCode]| Input::from_keys(&Controls::default(), None, &[], pressed);
        screen.handle_input(&keys(&[KeyCode::Enter]));
        assert!(screen.waiting);
        screen.handle_input(&keys(&[KeyCode::Escape]));
        assert!(!screen.waiting);
        assert!(!path.exists());
        screen.handle_input(&keys(&[KeyCode::Enter]));
        screen.handle_input(&keys(&[KeyCode::D]));
        assert!(screen.waiting);
        assert!(screen.message.contains("duplicate key"));
        assert_eq!(store.borrow().controls, Controls::default());
        screen.handle_input(&keys(&[KeyCode::J, KeyCode::K]));
        assert!(screen.message.contains("one key"));
        screen.handle_input(&keys(&[KeyCode::J]));
        assert!(!screen.waiting);
        assert_eq!(store.borrow().controls.bindings(GameChoice::Platform)[0].keys, vec!["J"]);
        assert_eq!(ControlStore::load(path).unwrap().controls, store.borrow().controls);
        screen.handle_input(&keys(&[KeyCode::R]));
        assert_eq!(store.borrow().controls, Controls::default());
        assert!(matches!(screen.handle_input(&keys(&[KeyCode::F3])), Transition::Pop));
    }

    #[test]
    fn controls_screen_stays_open_and_reports_save_failures() {
        use crate::controls::{ControlStore, Controls};
        use macroquad::prelude::KeyCode;
        let store = std::rc::Rc::new(std::cell::RefCell::new(ControlStore::in_memory()));
        let mut screen = ControlsScreen::new(GameChoice::Maze, std::rc::Rc::clone(&store));
        screen.handle_input(&Input::for_test(&[], &[Action::Confirm]));
        let input = Input::from_keys(&Controls::default(), None, &[], &[KeyCode::J]);
        screen.handle_input(&input);
        assert!(screen.waiting);
        assert!(screen.message.contains("Save failed"));
        assert_eq!(store.borrow().controls, Controls::default());
    }

    #[test]
    fn controls_overlay_from_pause_does_not_consume_platform_air() {
        use std::cell::RefCell;
        use std::rc::Rc;
        struct Observed(Rc<RefCell<PlatformScreen>>);
        impl State for Observed {
            fn handle_input(&mut self, input: &Input) -> Transition {
                self.0.borrow_mut().handle_input(input)
            }
            fn update(&mut self, dt: f32, input: &Input) {
                self.0.borrow_mut().update(dt, input);
            }
            fn draw(&self, _: &mut dyn Renderer) {}
        }
        let screen = Rc::new(RefCell::new(PlatformScreen::default()));
        let mut stack = StateStack::new(Box::new(Observed(Rc::clone(&screen))));
        let press = |action| Input::for_test(&[], &[action]);
        assert!(stack.handle_input(&press(Action::Pause)));
        assert!(stack.handle_input(&press(Action::Controls)));
        stack.update(10.0, &Input::default());
        assert_eq!(screen.borrow().game.air, platform::AIR_SECONDS);
        assert!(stack.handle_input(&press(Action::Back)));
        stack.update(10.0, &Input::default());
        assert_eq!(screen.borrow().game.air, platform::AIR_SECONDS);
        assert!(stack.handle_input(&press(Action::Confirm)));
        stack.update(STEP_SECONDS, &Input::default());
        assert!(screen.borrow().game.air < platform::AIR_SECONDS);
    }

    #[test]
    fn gameplay_hud_shows_saved_bindings_instead_of_default_keys() {
        use crate::controls::ControlStore;
        let mut store = ControlStore::in_memory();
        store.controls.rebind(GameChoice::Platform, Action::Jump, "J").unwrap();
        let stack = StateStack::with_controls(Box::new(PlatformScreen::default()), store);
        let mut renderer = RecordingRenderer::default();
        stack.draw(&mut renderer);
        let hint = renderer.texts.iter().find(|text| text.contains("Jump:")).unwrap();
        assert!(hint.contains("Jump: J"));
        assert!(!hint.contains("Jump: Space"));
        assert!(hint.contains("Esc: pause"));
    }

    #[test]
    fn level_error_screen_supports_retry_and_return_to_selected_game() {
        let mut screen = LevelErrorScreen { game: GameChoice::Platform,
            message: "platform.json: invalid geometry".to_owned() };
        let mut renderer = RecordingRenderer::default();
        screen.draw(&mut renderer);
        assert!(renderer.texts.iter().any(|text| text.contains("invalid geometry")));
        assert!(matches!(screen.handle_input(&Input::for_test(&[], &[Action::Confirm])),
            Transition::Replace(_)));
        assert!(matches!(screen.handle_input(&Input::for_test(&[], &[Action::Back])),
            Transition::Replace(_)));
        assert!(screen.input_profile().is_none());
    }
}
