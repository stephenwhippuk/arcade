pub const STEP_SECONDS: f32 = 1.0 / 60.0;
const MAX_FRAME_SECONDS: f32 = 0.25;

#[derive(Default)]
pub struct FixedClock {
    accumulator: f32,
}

impl FixedClock {
    pub fn advance(&mut self, frame_seconds: f32) -> usize {
        // Limit catch-up work after a stall or a suspended window.
        self.accumulator += frame_seconds.min(MAX_FRAME_SECONDS);
        let mut steps = 0;
        while self.accumulator >= STEP_SECONDS {
            self.accumulator -= STEP_SECONDS;
            steps += 1;
        }
        steps
    }

    pub fn reset(&mut self) {
        self.accumulator = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_frames_accumulate_to_one_update() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.advance(STEP_SECONDS / 2.0), 0);
        assert_eq!(clock.advance(STEP_SECONDS / 2.0), 1);
        assert_eq!(clock.advance(STEP_SECONDS * 3.0), 3);
    }

    #[test]
    fn catch_up_is_bounded_and_reset_discards_remainder() {
        let mut clock = FixedClock::default();
        let steps = clock.advance(10.0);
        assert!((14..=15).contains(&steps));
        assert!(clock.accumulator < STEP_SECONDS);
        clock.reset();
        assert_eq!(clock.advance(STEP_SECONDS / 2.0), 0);
    }
}
