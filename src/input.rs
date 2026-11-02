use macroquad::prelude::{is_key_down, is_key_pressed, KeyCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Confirm,
    Back,
    Pause,
    MainMenu,
}

#[derive(Default)]
pub struct Input {
    held: Vec<Action>,
    pressed: Vec<Action>,
}

impl Input {
    pub fn keyboard() -> Self {
        let bindings: &[(Action, &[KeyCode])] = &[
            (Action::MoveLeft, &[KeyCode::Left, KeyCode::A]),
            (Action::MoveRight, &[KeyCode::Right, KeyCode::D]),
            (Action::MoveUp, &[KeyCode::Up, KeyCode::W]),
            (Action::MoveDown, &[KeyCode::Down, KeyCode::S]),
            (Action::Confirm, &[KeyCode::Enter]),
            (Action::Back, &[KeyCode::Escape]),
            (Action::Pause, &[KeyCode::P]),
            (Action::MainMenu, &[KeyCode::M]),
        ];
        let mut input = Self::default();
        for &(action, keys) in bindings {
            if keys.iter().any(|&key| is_key_down(key)) {
                input.held.push(action);
            }
            if keys.iter().any(|&key| is_key_pressed(key)) {
                input.pressed.push(action);
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
}
