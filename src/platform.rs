use crate::events::GameEvent;
use crate::geometry::Rect;
use crate::levels::PlatformLevel;

pub const LEFT: f32 = 40.0;
pub const RIGHT: f32 = 760.0;
pub const TOP: f32 = 136.0;
pub const BOTTOM: f32 = 570.0;
#[cfg(test)]
pub const AIR_SECONDS: f64 = 60.0;
const SPEED: f32 = 200.0;
const GRAVITY: f32 = 1100.0;
const JUMP_SPEED: f32 = 480.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Won,
    Lost,
}

pub struct PlatformGame {
    pub level: PlatformLevel,
    pub events: Vec<GameEvent>,
    pub player: Rect,
    pub keys: Vec<Rect>,
    pub air: f64,
    pub score: u32,
    pub outcome: Outcome,
    velocity_y: f32,
    grounded: bool,
}

#[cfg(test)]
impl Default for PlatformGame {
    fn default() -> Self {
        Self::new(crate::levels::platform_fixture())
    }
}

impl PlatformGame {
    pub fn is_grounded(&self) -> bool { self.grounded }

    pub fn new(level: PlatformLevel) -> Self {
        Self {
            events: Vec::new(),
            player: level.player_start,
            keys: level.keys.clone(),
            air: level.air_seconds,
            level,
            score: 0,
            outcome: Outcome::Playing,
            velocity_y: 0.0,
            grounded: true,
        }
    }
}

impl PlatformGame {
    pub fn exit_open(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn update(&mut self, dt: f32, left: bool, right: bool, jump: bool) {
        if self.outcome != Outcome::Playing {
            return;
        }
        self.air = (self.air - f64::from(dt)).max(0.0);
        if self.air == 0.0 {
            self.outcome = Outcome::Lost;
            self.events.push(GameEvent::Lose);
            return;
        }
        if jump && self.grounded {
            self.events.push(GameEvent::Jump);
            self.velocity_y = -JUMP_SPEED;
            self.grounded = false;
        }
        let movement = match (left, right) {
            (true, false) => -SPEED * dt,
            (false, true) => SPEED * dt,
            _ => 0.0,
        };
        self.move_horizontal(movement);
        self.move_vertical(dt);
        self.keys.retain(|key| {
            if self.player.overlaps(*key) {
                self.score += 100;
                self.events.push(GameEvent::Collect);
                false
            } else {
                true
            }
        });
        if self.exit_open() && self.player.overlaps(self.level.exit) {
            self.outcome = Outcome::Won;
            self.events.push(GameEvent::Win);
        }
    }

    fn move_horizontal(&mut self, movement: f32) {
        let old_x = self.player.x;
        let mut next_x = (old_x + movement).clamp(LEFT, RIGHT - self.player.width);
        for platform in &self.level.platforms {
            let vertical_overlap = self.player.y < platform.y + platform.height
                && self.player.y + self.player.height > platform.y;
            if !vertical_overlap {
                continue;
            }
            if movement > 0.0
                && old_x + self.player.width <= platform.x
                && next_x + self.player.width >= platform.x
            {
                next_x = next_x.min(platform.x - self.player.width);
            } else if movement < 0.0
                && old_x >= platform.x + platform.width
                && next_x <= platform.x + platform.width
            {
                next_x = next_x.max(platform.x + platform.width);
            }
        }
        self.player.x = next_x;
    }

    fn move_vertical(&mut self, dt: f32) {
        self.velocity_y += GRAVITY * dt;
        let old_y = self.player.y;
        let mut next_y = old_y + self.velocity_y * dt;
        self.grounded = false;
        let falling = self.velocity_y >= 0.0;
        let mut collided = false;
        for platform in &self.level.platforms {
            let horizontal_overlap = self.player.x < platform.x + platform.width
                && self.player.x + self.player.width > platform.x;
            if !horizontal_overlap {
                continue;
            }
            // Resolve crossed surfaces, rather than relying on overlap at the end of a tick.
            if falling
                && old_y + self.player.height <= platform.y
                && next_y + self.player.height >= platform.y
            {
                next_y = next_y.min(platform.y - self.player.height);
                self.grounded = true;
                collided = true;
            } else if !falling
                && old_y >= platform.y + platform.height
                && next_y <= platform.y + platform.height
            {
                next_y = next_y.max(platform.y + platform.height);
                collided = true;
            }
        }
        if next_y < TOP {
            next_y = TOP;
            collided = true;
        }
        if collided {
            self.velocity_y = 0.0;
        }
        self.player.y = next_y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DT: f32 = 1.0 / 60.0;

    fn tick(game: &mut PlatformGame, left: bool, right: bool, jump: bool) {
        game.update(DT, left, right, jump);
    }

    #[test]
    fn floor_and_horizontal_bounds_hold_and_opposite_inputs_cancel() {
        let mut game = PlatformGame::default();
        tick(&mut game, true, true, false);
        assert_eq!(game.player.x, 70.0);
        assert_eq!(game.player.y + game.player.height, 550.0);
        game.player.x = LEFT;
        tick(&mut game, true, false, false);
        assert_eq!(game.player.x, LEFT);
        game.player.x = RIGHT - game.player.width;
        tick(&mut game, false, true, false);
        assert_eq!(game.player.x + game.player.width, RIGHT);
    }

    #[test]
    fn jumping_requires_ground_and_gravity_returns_player_to_floor() {
        let mut game = PlatformGame::default();
        tick(&mut game, false, false, true);
        assert!(game.player.y < 522.0);
        assert!(!game.grounded);
        let velocity = game.velocity_y;
        tick(&mut game, false, false, true);
        assert!(game.velocity_y > velocity);
        for _ in 0..60 {
            tick(&mut game, false, false, false);
        }
        assert!(game.grounded);
        assert_eq!(game.player.y, 522.0);
    }

    #[test]
    fn solid_platforms_block_heads_sides_and_catch_fast_falls() {
        let mut game = PlatformGame::default();
        game.player.x = 200.0;
        tick(&mut game, false, false, true);
        for _ in 0..10 {
            tick(&mut game, false, false, false);
            assert!(game.player.y >= game.level.platforms[1].y + game.level.platforms[1].height);
        }
        game.player.x = 120.0;
        game.player.y = 455.0;
        game.move_horizontal(100.0);
        assert_eq!(game.player.x + game.player.width, 150.0);
        game.player.x = 200.0;
        game.player.y = 390.0;
        game.velocity_y = 1000.0;
        game.move_vertical(0.1);
        assert_eq!(game.player.y + game.player.height, 465.0);
        assert!(game.grounded);
    }

    #[test]
    fn walking_off_a_platform_starts_falling() {
        let mut game = PlatformGame::default();
        game.player.x = 319.0;
        game.player.y = 437.0;
        tick(&mut game, false, true, false);
        assert!(!game.grounded);
        assert!(game.player.y > 437.0);
    }

    #[test]
    fn keys_score_once_and_exit_requires_all_keys() {
        let mut game = PlatformGame::default();
        game.player.x = 100.0;
        tick(&mut game, false, false, false);
        assert_eq!(game.keys.len(), 4);
        assert_eq!(game.score, 100);
        tick(&mut game, false, false, false);
        assert_eq!(game.score, 100);
        game.player.x = game.level.exit.x;
        game.player.y = game.level.exit.y;
        tick(&mut game, false, false, false);
        assert_eq!(game.outcome, Outcome::Playing);
        game.keys.clear();
        tick(&mut game, false, false, false);
        assert_eq!(game.outcome, Outcome::Won);
        let air = game.air;
        tick(&mut game, true, false, true);
        assert_eq!(game.air, air);
    }

    #[test]
    fn air_expires_at_sixty_seconds_and_has_priority_over_exit() {
        let mut game = PlatformGame::default();
        for _ in 0..3599 {
            tick(&mut game, false, false, false);
        }
        assert_eq!(game.outcome, Outcome::Playing);
        assert!((game.air - f64::from(DT)).abs() < 0.00001);
        game.keys.clear();
        game.player.x = game.level.exit.x;
        game.player.y = game.level.exit.y;
        tick(&mut game, false, false, false);
        assert_eq!(game.air, 0.0);
        assert_eq!(game.outcome, Outcome::Lost);
        assert_eq!(game.events, vec![GameEvent::Lose]);
        let x = game.player.x;
        tick(&mut game, true, false, true);
        assert_eq!(game.player.x, x);
    }

    fn walk_to(game: &mut PlatformGame, x: f32) {
        for _ in 0..240 {
            let delta = x - game.player.x;
            if delta.abs() <= SPEED * DT || game.outcome == Outcome::Won {
                return;
            }
            tick(game, delta < 0.0, delta > 0.0, false);
        }
        panic!("Could not walk to {x}; at {:?}", game.player);
    }

    fn jump_to(game: &mut PlatformGame, x: f32, surface_y: f32) {
        tick(game, x < game.player.x, x > game.player.x, true);
        for _ in 0..90 {
            let delta = x - game.player.x;
            tick(game, delta < -SPEED * DT, delta > SPEED * DT, false);
            if game.grounded {
                assert_eq!(game.player.y + game.player.height, surface_y);
                return;
            }
        }
        panic!("Did not land at {surface_y}; at {:?}", game.player);
    }

    #[test]
    fn entire_level_can_be_completed_with_real_movement_before_air_expires() {
        let mut game = PlatformGame::default();
        walk_to(&mut game, 105.0);
        walk_to(&mut game, 90.0);
        jump_to(&mut game, 195.0, 465.0);
        walk_to(&mut game, 290.0);
        jump_to(&mut game, 390.0, 380.0);
        walk_to(&mut game, 430.0);
        walk_to(&mut game, 345.0);
        jump_to(&mut game, 245.0, 295.0);
        walk_to(&mut game, 235.0);
        walk_to(&mut game, 290.0);
        jump_to(&mut game, 390.0, 210.0);
        walk_to(&mut game, 520.0);
        assert!(game.keys.is_empty(), "Uncollected keys: {:?}", game.keys);
        assert_eq!(game.score, 500);
        assert_eq!(game.outcome, Outcome::Won);
        assert!(game.air > 40.0, "Route used too much air: {}", game.air);
    }
}
