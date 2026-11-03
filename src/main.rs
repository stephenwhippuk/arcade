mod audio;
mod assets;
mod controls;
mod events;
mod geometry;
mod input;
mod invaders;
mod levels;
mod maze;
mod platform;
mod renderer;
mod screens;
mod state;
mod time;

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
    let controls = match controls::config_path().and_then(controls::ControlStore::load) {
        Ok(controls) => controls,
        Err(error) => {
            show_startup_error(&error).await;
            return;
        }
    };
    let assets = match assets::Assets::load() {
        Ok(assets) => assets,
        Err(error) => {
            show_startup_error(&error.to_string()).await;
            return;
        }
    };
    let mut renderer = renderer::MacroquadRenderer::new(assets);
    let mut audio = match audio::Audio::load().await {
        Ok(audio) => audio,
        Err(error) => {
            show_startup_error(&format!("Failed to load Arcade sound effects: {error}")).await;
            return;
        }
    };
    let mut states = StateStack::with_controls(Box::new(MainMenu::default()), controls);
    let mut clock = FixedClock::default();

    while !states.is_empty() {
        let input = states.keyboard_input();
        if states.handle_input(&input) {
            // Do not carry time from the previous screen into a new or resumed state.
            clock.reset();
        } else {
            for _ in 0..clock.advance(get_frame_time()) {
                states.update(STEP_SECONDS, &input);
            }
        }
        audio.set_settings(states.audio_settings());
        for event in states.drain_events() {
            audio.play(event);
        }
        if states.is_empty() {
            break;
        }

        clear_background(Color::from_rgba(18, 22, 32, 255));
        states.draw(&mut renderer);
        next_frame().await;
    }

    async fn show_startup_error(message: &str) {
        eprintln!("{message}");
        loop {
            clear_background(BLACK);
            draw_text("Startup failed. See terminal for details.", 25.0, 250.0, 24.0, RED);
            draw_text("Escape: exit", 25.0, 300.0, 24.0, WHITE);
            if is_key_pressed(KeyCode::Escape) {
                return;
            }
            next_frame().await;
        }
    }
}
