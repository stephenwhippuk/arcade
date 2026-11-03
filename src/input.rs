use crate::controls::{Controls, Game, KEYS};
use macroquad::prelude::{get_keys_down, get_keys_pressed, KeyCode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Fire,
    Jump,
    Confirm,
    Back,
    Pause,
    MainMenu,
    Settings,
    Controls,
    ResetControls,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::MoveLeft => "Left",
            Self::MoveRight => "Right",
            Self::MoveUp => "Up",
            Self::MoveDown => "Down",
            Self::Fire => "Fire",
            Self::Jump => "Jump",
            Self::Pause => "Pause",
            _ => "Menu",
        }
    }
}

#[derive(Default)]
pub struct Input {
    held: Vec<Action>,
    pressed: Vec<Action>,
    pub raw_pressed: Vec<KeyCode>,
}

impl Input {
    pub fn keyboard(controls: &Controls, game: Option<Game>) -> Self {
        Self::from_keys(controls, game, &get_keys_down().into_iter().collect::<Vec<_>>(),
            &get_keys_pressed().into_iter().collect::<Vec<_>>())
    }

    pub fn from_keys(controls: &Controls, game: Option<Game>, held: &[KeyCode], pressed: &[KeyCode]) -> Self {
        let bindings: &[(Action, &[KeyCode])] = &[
            (Action::MoveLeft, &[KeyCode::Left, KeyCode::A]),
            (Action::MoveRight, &[KeyCode::Right, KeyCode::D]),
            (Action::MoveUp, &[KeyCode::Up, KeyCode::W]),
            (Action::MoveDown, &[KeyCode::Down, KeyCode::S]),
            (Action::Confirm, &[KeyCode::Enter]),
            (Action::Back, &[KeyCode::Escape]),
            (Action::Pause, &[KeyCode::P]),
            (Action::MainMenu, &[KeyCode::M]),
            (Action::Settings, &[KeyCode::F1]),
            (Action::Controls, &[KeyCode::F3]),
            (Action::ResetControls, &[KeyCode::R]),
        ];
        let mut raw_pressed = pressed.to_vec();
        raw_pressed.sort_by_key(|key| *key as u32);
        let mut input = Self { raw_pressed, ..Self::default() };
        for &(action, keys) in bindings {
            let movement = matches!(action, Action::MoveLeft | Action::MoveRight | Action::MoveUp | Action::MoveDown);
            if game.is_some() && (movement || action == Action::Pause) {
                continue;
            }
            if keys.iter().any(|key| held.contains(key)) {
                input.held.push(action);
            }
            if keys.iter().any(|key| pressed.contains(key)) {
                input.pressed.push(action);
            }
            if let Some(game) = game {
                for binding in controls.bindings(game) {
                    let keys: Vec<_> = binding.keys.iter().map(|name| {
                        KEYS.iter().find(|(key, _)| *key == name)
                            .expect("Controls are validated before use").1
                    }).collect();
                    if keys.iter().any(|key| held.contains(key)) {
                        input.held.push(binding.action);
                    }
                    if keys.iter().any(|key| pressed.contains(key)) {
                        input.pressed.push(binding.action);
                    }
                }
            }
        }
        input
    }

    pub fn held(&self, action: Action) -> bool {
        self.held.contains(&action)
    }

    pub fn pressed(&self, action: Action) -> bool {
        self.pressed.contains(&action)
    }

    #[cfg(test)]
    pub fn for_test(held: &[Action], pressed: &[Action]) -> Self {
        Self {
            held: held.to_vec(),
            pressed: pressed.to_vec(),
            raw_pressed: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holding_an_action_does_not_repeat_its_press() {
        let input = Input::for_test(&[Action::Pause], &[]);
        assert!(input.held(Action::Pause));
        assert!(!input.pressed(Action::Pause));
    }

    #[test]
    fn active_profile_maps_only_its_actions_and_keeps_navigation_fixed() {
        let mut controls = Controls::default();
        controls.rebind(Game::Maze, Action::MoveLeft, "J").unwrap();
        let input = Input::from_keys(&controls, Some(Game::Maze), &[KeyCode::J],
            &[KeyCode::J, KeyCode::Escape, KeyCode::F3]);
        assert!(input.held(Action::MoveLeft));
        assert!(input.pressed(Action::MoveLeft));
        assert!(input.pressed(Action::Back));
        assert!(input.pressed(Action::Controls));
        let old = Input::from_keys(&controls, Some(Game::Maze), &[KeyCode::A], &[KeyCode::A]);
        assert!(!old.held(Action::MoveLeft));
        let other = Input::from_keys(&controls, Some(Game::Invaders), &[KeyCode::A], &[KeyCode::A]);
        assert!(other.held(Action::MoveLeft));
        let menu = Input::from_keys(&controls, None, &[], &[KeyCode::Down, KeyCode::Enter]);
        assert!(menu.pressed(Action::MoveDown));
        assert!(menu.pressed(Action::Confirm));
    }

    #[test]
    fn fire_and_jump_are_per_game_and_held_keys_do_not_create_presses() {
        let controls = Controls::default();
        let shooter = Input::from_keys(&controls, Some(Game::Invaders), &[KeyCode::Space], &[]);
        assert!(shooter.held(Action::Fire));
        assert!(!shooter.pressed(Action::Fire));
        assert!(!shooter.held(Action::Jump));
        let platform = Input::from_keys(&controls, Some(Game::Platform), &[], &[KeyCode::Space]);
        assert!(platform.pressed(Action::Jump));
        assert!(!platform.pressed(Action::Fire));
    }

    #[test]
    fn remapped_pause_is_active_in_game_but_overlay_resume_stays_fixed() {
        let mut controls = Controls::default();
        controls.rebind(Game::Platform, Action::Pause, "Q").unwrap();
        let old = Input::from_keys(&controls, Some(Game::Platform), &[], &[KeyCode::P]);
        assert!(!old.pressed(Action::Pause));
        let new = Input::from_keys(&controls, Some(Game::Platform), &[], &[KeyCode::Q]);
        assert!(new.pressed(Action::Pause));
        let overlay = Input::from_keys(&controls, None, &[], &[KeyCode::P]);
        assert!(overlay.pressed(Action::Pause));
    }
}
