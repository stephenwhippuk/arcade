use crate::input::Input;

pub enum Transition {
    None,
    Push(Box<dyn State>),
    Pop,
    Replace(Box<dyn State>),
    Reset(Box<dyn State>),
}

pub trait State {
    fn handle_input(&mut self, input: &Input) -> Transition;
    fn update(&mut self, dt: f32, input: &Input);
    fn draw(&self);

    fn is_transparent(&self) -> bool {
        false
    }
}

pub struct StateStack {
    states: Vec<Box<dyn State>>,
}

impl StateStack {
    pub fn new(initial: Box<dyn State>) -> Self {
        Self {
            states: vec![initial],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn handle_input(&mut self, input: &Input) -> bool {
        let Some(state) = self.states.last_mut() else {
            return false;
        };
        let transition = state.handle_input(input);
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
        }
    }

    pub fn draw(&self) {
        let first_visible = self.states.iter().rposition(|state| !state.is_transparent());
        for state in &self.states[first_visible.unwrap_or(0)..] {
            state.draw();
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

        fn draw(&self) {
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
        stack.draw();
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
        stack.draw();
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
        stack.draw();
    }
}
