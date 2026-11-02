use crate::input::{Action, Input};
use crate::state::{State, Transition};
use macroquad::prelude::*;

pub struct MainMenu;

impl State for MainMenu {
    fn handle_input(&mut self, input: &Input) -> Transition {
        if input.pressed(Action::Back) {
            Transition::Pop
        } else if input.pressed(Action::Confirm) {
            Transition::Replace(Box::new(Demo::default()))
        } else {
            Transition::None
        }
    }

    fn update(&mut self, _: f32, _: &Input) {}

    fn draw(&self) {
        draw_text("Arcade", 80.0, 160.0, 64.0, WHITE);
        draw_text("State-stack demo", 80.0, 210.0, 28.0, LIGHTGRAY);
        draw_text("Enter: start demo", 80.0, 310.0, 28.0, GREEN);
        draw_text("Escape: quit", 80.0, 360.0, 24.0, LIGHTGRAY);
    }
}

struct Demo {
    x: f32,
    y: f32,
    elapsed: f32,
}

impl Default for Demo {
    fn default() -> Self {
        Self {
            x: 400.0,
            y: 300.0,
            elapsed: 0.0,
        }
    }
}

impl State for Demo {
    fn handle_input(&mut self, input: &Input) -> Transition {
        if input.pressed(Action::Back) || input.pressed(Action::Pause) {
            Transition::Push(Box::new(PauseMenu))
        } else {
            Transition::None
        }
    }

    fn update(&mut self, dt: f32, input: &Input) {
        let mut direction = Vec2::ZERO;
        if input.held(Action::MoveLeft) {
            direction.x -= 1.0;
        }
        if input.held(Action::MoveRight) {
            direction.x += 1.0;
        }
        if input.held(Action::MoveUp) {
            direction.y -= 1.0;
        }
        if input.held(Action::MoveDown) {
            direction.y += 1.0;
        }
        let direction = direction.normalize_or_zero();
        self.x = (self.x + direction.x * 200.0 * dt).clamp(48.0, 752.0);
        self.y = (self.y + direction.y * 200.0 * dt).clamp(148.0, 552.0);
        self.elapsed += dt;
    }

    fn draw(&self) {
        draw_text("Demo", 32.0, 48.0, 32.0, WHITE);
        draw_text(
            "Arrows / WASD: move    Escape / P: pause",
            32.0,
            82.0,
            22.0,
            LIGHTGRAY,
        );
        draw_rectangle_lines(32.0, 132.0, 736.0, 436.0, 2.0, DARKGRAY);
        draw_circle(self.x, self.y, 16.0, SKYBLUE);
        let orbit_x = 400.0 + (self.elapsed * 2.0).cos() * 90.0;
        let orbit_y = 350.0 + (self.elapsed * 2.0).sin() * 90.0;
        draw_circle(orbit_x, orbit_y, 8.0, GOLD);
        draw_text(
            &format!("Simulation: {:.1}s", self.elapsed),
            32.0,
            116.0,
            20.0,
            LIGHTGRAY,
        );
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
    fn movement_is_bounded_and_diagonal_speed_is_normalized() {
        let mut demo = Demo::default();
        let start = vec2(demo.x, demo.y);
        let input = Input::for_test(&[Action::MoveRight, Action::MoveDown], &[]);
        demo.update(1.0, &input);
        assert!((vec2(demo.x, demo.y).distance(start) - 200.0).abs() < 0.001);
        demo.update(100.0, &input);
        assert_eq!((demo.x, demo.y), (752.0, 552.0));
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
