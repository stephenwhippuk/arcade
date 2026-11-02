use crate::input::{Action, Input};
use crate::maze::{Direction, Maze, Outcome, Tile, HEIGHT, WIDTH};
use crate::state::{State, Transition};
use macroquad::prelude::*;

pub struct MainMenu;

impl State for MainMenu {
    fn handle_input(&mut self, input: &Input) -> Transition {
        if input.pressed(Action::Back) {
            Transition::Pop
        } else if input.pressed(Action::Confirm) {
            Transition::Replace(Box::new(MazeScreen::default()))
        } else {
            Transition::None
        }
    }

    fn update(&mut self, _: f32, _: &Input) {}

    fn draw(&self) {
        draw_text("Arcade", 80.0, 160.0, 64.0, WHITE);
        draw_text("Maze Chase", 80.0, 210.0, 28.0, LIGHTGRAY);
        draw_text("Collect every pellet. Avoid the pursuers.", 80.0, 255.0, 24.0, LIGHTGRAY);
        draw_text("Enter: play", 80.0, 310.0, 28.0, GREEN);
        draw_text("Escape: quit", 80.0, 360.0, 24.0, LIGHTGRAY);
    }
}

#[derive(Default)]
struct MazeScreen {
    game: Maze,
}

impl State for MazeScreen {
    fn handle_input(&mut self, input: &Input) -> Transition {
        if self.game.outcome != Outcome::Playing {
            if input.pressed(Action::MainMenu) || input.pressed(Action::Back) {
                return Transition::Reset(Box::new(MainMenu));
            }
            if input.pressed(Action::Confirm) {
                return Transition::Replace(Box::new(Self::default()));
            }
            return Transition::None;
        }
        if input.pressed(Action::Back) || input.pressed(Action::Pause) {
            return Transition::Push(Box::new(PauseMenu));
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

    fn draw(&self) {
        const CELL: f32 = 28.0;
        const LEFT: f32 = (800.0 - WIDTH as f32 * CELL) / 2.0;
        const TOP: f32 = 136.0;
        draw_text("Maze Chase", 32.0, 40.0, 32.0, WHITE);
        draw_text(
            &format!(
                "Score: {}    Lives: {}    Pellets: {}",
                self.game.score, self.game.lives, self.game.remaining
            ),
            32.0,
            72.0,
            24.0,
            LIGHTGRAY,
        );
        draw_text(
            "Arrows / WASD: turn    Escape / P: pause",
            32.0,
            104.0,
            20.0,
            LIGHTGRAY,
        );
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let tile = Tile { x, y };
                let left = LEFT + x as f32 * CELL;
                let top = TOP + y as f32 * CELL;
                if Maze::is_wall(tile) {
                    draw_rectangle(left + 1.0, top + 1.0, CELL - 2.0, CELL - 2.0, DARKBLUE);
                } else if self.game.has_pellet(tile) {
                    draw_circle(left + CELL / 2.0, top + CELL / 2.0, 3.0, WHITE);
                }
            }
        }
        let fraction = self.game.animation_fraction();
        let (x, y) = self.game.player.position(fraction);
        draw_circle(LEFT + (x + 0.5) * CELL, TOP + (y + 0.5) * CELL, 10.0, YELLOW);
        for (enemy, color) in self.game.enemies.iter().zip([RED, SKYBLUE]) {
            let (x, y) = enemy.position(fraction);
            draw_rectangle(
                LEFT + (x + 0.5) * CELL - 9.0,
                TOP + (y + 0.5) * CELL - 9.0,
                18.0,
                18.0,
                color,
            );
        }
        if self.game.is_respawning() {
            draw_text("Caught! Get ready...", 270.0, 590.0, 24.0, YELLOW);
        }
        if self.game.outcome != Outcome::Playing {
            draw_rectangle(0.0, 0.0, 800.0, 600.0, Color::new(0.0, 0.0, 0.0, 0.8));
            let title = if self.game.outcome == Outcome::Won { "Maze cleared!" } else { "Game over" };
            draw_text(title, 230.0, 235.0, 48.0, WHITE);
            draw_text(&format!("Final score: {}", self.game.score), 230.0, 290.0, 28.0, YELLOW);
            draw_text("Enter: play again", 230.0, 350.0, 26.0, GREEN);
            draw_text("M / Escape: main menu", 230.0, 395.0, 26.0, LIGHTGRAY);
        }
    }
}

struct PauseMenu;

impl State for PauseMenu {
    fn handle_input(&mut self, input: &Input) -> Transition {
        if input.pressed(Action::MainMenu) {
            Transition::Reset(Box::new(MainMenu))
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

    fn draw(&self) {
        draw_rectangle(0.0, 0.0, 800.0, 600.0, Color::new(0.0, 0.0, 0.0, 0.75));
        draw_text("Paused", 240.0, 220.0, 48.0, WHITE);
        draw_text("Escape / P / Enter: resume", 240.0, 285.0, 26.0, GREEN);
        draw_text("M: main menu", 240.0, 375.0, 26.0, LIGHTGRAY);
    }

    fn is_transparent(&self) -> bool {
        true
    }
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
        let mut stack = StateStack::new(Box::new(MainMenu));
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
}
