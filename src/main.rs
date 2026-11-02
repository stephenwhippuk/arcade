mod input;
mod maze;
mod screens;
mod state;
mod time;

use input::Input;
use macroquad::prelude::*;
use screens::MainMenu;
use state::StateStack;
use time::{FixedClock, STEP_SECONDS};

fn window_conf() -> Conf {
    Conf {
        window_title: "Arcade".to_owned(),
        window_width: 800,
        window_height: 600,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut states = StateStack::new(Box::new(MainMenu));
    let mut clock = FixedClock::default();

    while !states.is_empty() {
        let input = Input::keyboard();
        if states.handle_input(&input) {
            // Do not carry time from the previous screen into a new or resumed state.
            clock.reset();
        } else {
            for _ in 0..clock.advance(get_frame_time()) {
                states.update(STEP_SECONDS, &input);
            }
        }
        if states.is_empty() {
            break;
        }

        clear_background(Color::from_rgba(18, 22, 32, 255));
        states.draw();
        next_frame().await;
    }
}
