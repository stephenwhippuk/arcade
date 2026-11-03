use crate::audio::AudioSettings;
use crate::controls::{ControlStore, Game};
use crate::events::GameEvent;
use crate::input::{Action, Input};
use crate::screens::{AudioSettingsScreen, ControlsScreen};
use crate::renderer::Renderer;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub enum Transition {
    None,
    Push(Box<dyn State>),
    Pop,
    Replace(Box<dyn State>),
    Reset(Box<dyn State>),
}

pub trait State {
    fn input_profile(&self) -> Option<Game> { None }
    fn controls_target(&self) -> Option<Game> { None }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        Vec::new()
    }

    fn allows_settings(&self) -> bool {
        false
    }

    fn handle_input(&mut self, input: &Input) -> Transition;
    fn update(&mut self, dt: f32, input: &Input);
    fn draw(&self, renderer: &mut dyn Renderer);

    fn is_transparent(&self) -> bool {
        false
    }
}

pub struct StateStack {
    states: Vec<Box<dyn State>>,
    events: Vec<GameEvent>,
    audio_settings: Rc<Cell<AudioSettings>>,
    controls: Rc<RefCell<ControlStore>>,
}

impl StateStack {
    #[cfg(test)]
    pub fn new(initial: Box<dyn State>) -> Self {
        Self::with_controls(initial, ControlStore::in_memory())
    }

    pub fn with_controls(initial: Box<dyn State>, controls: ControlStore) -> Self {
        Self {
            states: vec![initial],
            events: Vec::new(),
            audio_settings: Rc::new(Cell::new(AudioSettings::default())),
            controls: Rc::new(RefCell::new(controls)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn handle_input(&mut self, input: &Input) -> bool {
        let Some(state) = self.states.last_mut() else {
            return false;
        };
        if input.pressed(Action::Controls) {
            if let Some(game) = state.controls_target() {
                let controls = Rc::clone(&self.controls);
                self.events.push(GameEvent::Menu);
                return self.apply(Transition::Push(Box::new(ControlsScreen::new(game, controls))));
            }
        }
        if input.pressed(Action::Settings) && state.allows_settings() {
            let settings = Rc::clone(&self.audio_settings);
            self.events.push(GameEvent::Menu);
            return self.apply(Transition::Push(Box::new(AudioSettingsScreen::new(settings))));
        }
        let transition = state.handle_input(input);
        self.events.extend(state.drain_events());
        if !matches!(transition, Transition::None)
            || (state.allows_settings()
                && (input.pressed(Action::MoveUp) || input.pressed(Action::MoveDown)))
        {
            self.events.push(GameEvent::Menu);
        }
        self.apply(transition)
    }

    fn apply(&mut self, transition: Transition) -> bool {
        match transition {
            Transition::None => return false,
            Transition::Push(state) => self.states.push(state),
            Transition::Pop => {
                self.states.pop();
            }
            Transition::Replace(state) => {
                self.states.pop();
                self.states.push(state);
            }
            Transition::Reset(state) => {
                self.states.clear();
                self.states.push(state);
            }
        }
        true
    }

    pub fn update(&mut self, dt: f32, input: &Input) {
        if let Some(state) = self.states.last_mut() {
            state.update(dt, input);
            self.events.extend(state.drain_events());
        }
    }

    pub fn audio_settings(&self) -> AudioSettings {
        self.audio_settings.get()
    }

    pub fn keyboard_input(&self) -> Input {
        let profile = self.states.last().and_then(|state| state.input_profile());
        Input::keyboard(&self.controls.borrow().controls, profile)
    }

    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn draw(&self, renderer: &mut dyn Renderer) {
        let first_visible = self.states.iter().rposition(|state| !state.is_transparent());
        for state in &self.states[first_visible.unwrap_or(0)..] {
            state.draw(renderer);
            if let Some(game) = state.input_profile() {
                let hint = format!("{}   Esc: pause", self.controls.borrow().controls.hint(game));
                renderer.draw_text(&hint, 32.0, 104.0, 16.0, crate::renderer::LIGHTGRAY);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Default)]
    struct Counts {
        updates: Cell<usize>,
        draws: Cell<usize>,
    }

    struct TestState {
        counts: Rc<Counts>,
        transparent: bool,
    }

    impl State for TestState {
        fn handle_input(&mut self, _: &Input) -> Transition {
            Transition::None
        }

        fn update(&mut self, _: f32, _: &Input) {
            self.counts.updates.set(self.counts.updates.get() + 1);
        }

        fn draw(&self, _: &mut dyn Renderer) {
            self.counts.draws.set(self.counts.draws.get() + 1);
        }

        fn is_transparent(&self) -> bool {
            self.transparent
        }
    }

    fn test_state(counts: &Rc<Counts>, transparent: bool) -> Box<dyn State> {
        Box::new(TestState {
            counts: Rc::clone(counts),
            transparent,
        })
    }

    #[test]
    fn overlay_draws_underlying_state_but_blocks_its_updates() {
        let game = Rc::new(Counts::default());
        let pause = Rc::new(Counts::default());
        let mut stack = StateStack::new(test_state(&game, false));
        stack.apply(Transition::Push(test_state(&pause, true)));
        stack.update(0.1, &Input::default());
        stack.draw(&mut crate::renderer::RecordingRenderer::default());
        assert_eq!(game.updates.get(), 0);
        assert_eq!(game.draws.get(), 1);
        assert_eq!(pause.updates.get(), 1);
        assert_eq!(pause.draws.get(), 1);

        stack.apply(Transition::Pop);
        stack.update(0.1, &Input::default());
        assert_eq!(game.updates.get(), 1);
    }

    #[test]
    fn opaque_state_hides_underlying_screens() {
        let menu = Rc::new(Counts::default());
        let game = Rc::new(Counts::default());
        let mut stack = StateStack::new(test_state(&menu, false));
        stack.apply(Transition::Push(test_state(&game, false)));
        stack.draw(&mut crate::renderer::RecordingRenderer::default());
        assert_eq!(menu.draws.get(), 0);
        assert_eq!(game.draws.get(), 1);
    }

    #[test]
    fn replace_preserves_lower_states_and_reset_discards_them() {
        let counts = Rc::new(Counts::default());
        let mut stack = StateStack::new(test_state(&counts, false));
        stack.apply(Transition::Push(test_state(&counts, false)));
        stack.apply(Transition::Replace(test_state(&counts, false)));
        assert_eq!(stack.states.len(), 2);
        stack.apply(Transition::Reset(test_state(&counts, false)));
        assert_eq!(stack.states.len(), 1);
        stack.apply(Transition::Pop);
        assert!(stack.is_empty());
        assert!(!stack.handle_input(&Input::default()));
        stack.update(0.1, &Input::default());
        stack.draw(&mut crate::renderer::RecordingRenderer::default());
    }
}
