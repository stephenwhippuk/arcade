use crate::events::GameEvent;
use crate::geometry::Rect;

pub const LEFT: f32 = 40.0;
pub const RIGHT: f32 = 760.0;
pub const TOP: f32 = 130.0;
pub const BOTTOM: f32 = 570.0;
const PLAYER_Y: f32 = 530.0;
const SHOT_SECONDS: f32 = 0.3;
const ENEMY_SHOT_SECONDS: f32 = 0.9;
const COLUMNS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Won,
    Lost,
}

pub struct Alien {
    pub bounds: Rect,
    column: usize,
}

pub struct Invaders {
    pub events: Vec<GameEvent>,
    pub player: Rect,
    pub aliens: Vec<Alien>,
    pub player_shots: Vec<Rect>,
    pub enemy_shots: Vec<Rect>,
    pub score: u32,
    pub lives: u8,
    pub outcome: Outcome,
    direction: f32,
    shot_timer: f32,
    enemy_shot_timer: f32,
    firing_column: usize,
    respawn: f32,
}

impl Default for Invaders {
    fn default() -> Self {
        let mut aliens = Vec::new();
        for row in 0..3 {
            for column in 0..COLUMNS {
                aliens.push(Alien {
                    bounds: Rect {
                        x: 110.0 + column as f32 * 65.0,
                        y: 150.0 + row as f32 * 44.0,
                        width: 32.0,
                        height: 24.0,
                    },
                    column,
                });
            }
        }
        Self {
            events: Vec::new(),
            player: Self::player_start(),
            aliens,
            player_shots: Vec::new(),
            enemy_shots: Vec::new(),
            score: 0,
            lives: 3,
            outcome: Outcome::Playing,
            direction: 1.0,
            shot_timer: 0.0,
            enemy_shot_timer: ENEMY_SHOT_SECONDS,
            firing_column: 0,
            respawn: 0.0,
        }
    }
}

impl Invaders {
    fn player_start() -> Rect {
        Rect { x: 382.0, y: PLAYER_Y, width: 36.0, height: 20.0 }
    }

    pub fn is_respawning(&self) -> bool {
        self.respawn > 0.0
    }

    pub fn update(&mut self, dt: f32, left: bool, right: bool, fire: bool) {
        if self.outcome != Outcome::Playing {
            return;
        }
        if self.respawn > 0.0 {
            self.respawn = (self.respawn - dt).max(0.0);
            return;
        }
        let movement = match (left, right) {
            (true, false) => -1.0,
            (false, true) => 1.0,
            _ => 0.0,
        };
        self.player.x = (self.player.x + movement * 300.0 * dt)
            .clamp(LEFT, RIGHT - self.player.width);
        self.shot_timer = (self.shot_timer - dt).max(0.0);
        if fire && self.shot_timer == 0.0 {
            self.events.push(GameEvent::Shoot);
            self.player_shots.push(Rect {
                x: self.player.x + self.player.width / 2.0 - 2.0,
                y: self.player.y - 10.0,
                width: 4.0,
                height: 10.0,
            });
            self.shot_timer = SHOT_SECONDS;
        }

        let speed = 35.0 + (24 - self.aliens.len()) as f32 * 3.0;
        let distance = self.direction * speed * dt;
        let edge = self.aliens.iter().any(|alien| {
            alien.bounds.x + distance < LEFT
                || alien.bounds.x + alien.bounds.width + distance > RIGHT
        });
        if edge {
            self.direction = -self.direction;
            for alien in &mut self.aliens {
                alien.bounds.y += 24.0;
            }
        } else {
            for alien in &mut self.aliens {
                alien.bounds.x += distance;
            }
        }
        if self.aliens.iter().any(|alien| alien.bounds.y + alien.bounds.height >= PLAYER_Y) {
            self.outcome = Outcome::Lost;
            self.events.push(GameEvent::Lose);
            return;
        }

        self.enemy_shot_timer -= dt;
        if self.enemy_shot_timer <= 0.0 {
            self.fire_enemy_shot();
            self.enemy_shot_timer = ENEMY_SHOT_SECONDS;
        }

        let mut surviving_shots = Vec::new();
        for mut shot in self.player_shots.drain(..) {
            let previous_y = shot.y;
            shot.y -= 480.0 * dt;
            let swept = Rect { height: previous_y + shot.height - shot.y, ..shot };
            // Hit the first alien along an upward shot, not whichever appears first in storage.
            let target = self.aliens.iter().enumerate()
                .filter(|(_, alien)| swept.overlaps(alien.bounds))
                .max_by(|(_, a), (_, b)| a.bounds.y.total_cmp(&b.bounds.y))
                .map(|(index, _)| index);
            if let Some(index) = target {
                self.aliens.remove(index);
                self.score += 100;
                self.events.push(GameEvent::Collect);
            } else if shot.y + shot.height >= TOP {
                surviving_shots.push(shot);
            }
        }
        self.player_shots = surviving_shots;

        let mut hit = false;
        self.enemy_shots.retain_mut(|shot| {
            let previous_y = shot.y;
            shot.y += 240.0 * dt;
            let swept = Rect {
                y: previous_y,
                height: shot.y + shot.height - previous_y,
                ..*shot
            };
            if swept.overlaps(self.player) {
                hit = true;
                false
            } else {
                shot.y < BOTTOM
            }
        });
        if hit {
            self.lives -= 1;
            self.events.push(GameEvent::Hit);
            self.player_shots.clear();
            self.enemy_shots.clear();
            if self.lives == 0 {
                self.outcome = Outcome::Lost;
                self.events.push(GameEvent::Lose);
                return;
            }
            self.player = Self::player_start();
            self.shot_timer = 0.0;
            self.enemy_shot_timer = ENEMY_SHOT_SECONDS;
            self.respawn = 1.0;
        }
        if self.aliens.is_empty() {
            self.outcome = Outcome::Won;
            self.events.push(GameEvent::Win);
        }
    }

    fn fire_enemy_shot(&mut self) {
        for offset in 0..COLUMNS {
            let column = (self.firing_column + offset) % COLUMNS;
            let shooter = self.aliens.iter()
                .filter(|alien| alien.column == column)
                .max_by(|a, b| a.bounds.y.total_cmp(&b.bounds.y));
            if let Some(alien) = shooter {
                self.enemy_shots.push(Rect {
                    x: alien.bounds.x + alien.bounds.width / 2.0 - 3.0,
                    y: alien.bounds.y + alien.bounds.height,
                    width: 6.0,
                    height: 12.0,
                });
                self.firing_column = (column + 1) % COLUMNS;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn movement_is_bounded_and_opposite_inputs_cancel() {
        let mut game = Invaders::default();
        game.update(DT, true, true, false);
        assert_eq!(game.player.x, 382.0);
        game.player.x = LEFT;
        game.update(DT, true, false, false);
        assert_eq!(game.player.x, LEFT);
        game.player.x = RIGHT - game.player.width;
        game.update(DT, false, true, false);
        assert_eq!(game.player.x + game.player.width, RIGHT);
    }

    #[test]
    fn holding_fire_is_rate_limited() {
        let mut game = Invaders::default();
        game.update(DT, false, false, true);
        assert_eq!(game.player_shots.len(), 1);
        for _ in 0..10 {
            game.update(DT, false, false, true);
        }
        assert_eq!(game.player_shots.len(), 1);
        for _ in 0..10 {
            game.update(DT, false, false, true);
        }
        assert_eq!(game.player_shots.len(), 2);
    }

    #[test]
    fn formation_reverses_and_descends_at_the_edge() {
        let mut game = Invaders::default();
        game.aliens.retain(|alien| alien.column == 0);
        for alien in &mut game.aliens {
            alien.bounds.x = RIGHT - alien.bounds.width;
        }
        let y = game.aliens[0].bounds.y;
        game.update(DT, false, false, false);
        assert_eq!(game.direction, -1.0);
        assert_eq!(game.aliens[0].bounds.y, y + 24.0);
        game.update(DT, false, false, false);
        assert!(game.aliens[0].bounds.x + game.aliens[0].bounds.width < RIGHT);
    }

    #[test]
    fn swept_shot_hits_nearest_alien_once() {
        let mut game = Invaders::default();
        game.aliens.retain(|alien| alien.column == 0);
        game.player_shots.push(Rect { x: 145.0, y: 320.0, width: 4.0, height: 10.0 });
        game.update(0.2, false, false, false);
        assert_eq!(game.aliens.len(), 2);
        assert!(game.aliens.iter().all(|alien| alien.bounds.y < 238.0));
        assert_eq!(game.score, 100);
        assert!(game.player_shots.is_empty());
    }

    #[test]
    fn enemy_shoots_from_lowest_surviving_alien_and_skips_empty_columns() {
        let mut game = Invaders::default();
        game.aliens.retain(|alien| alien.column == 3);
        game.fire_enemy_shot();
        assert_eq!(game.enemy_shots.len(), 1);
        assert_eq!(game.enemy_shots[0].y, 262.0);
        assert_eq!(game.firing_column, 4);
    }

    #[test]
    fn hits_cost_one_life_per_tick_and_respawn_preserves_progress() {
        let mut game = Invaders::default();
        let shot = Rect { x: 395.0, y: 510.0, width: 6.0, height: 12.0 };
        game.enemy_shots = vec![shot, shot];
        game.score = 100;
        game.update(0.1, false, false, false);
        assert_eq!(game.lives, 2);
        assert_eq!(game.score, 100);
        assert!(game.is_respawning());
        assert!(game.enemy_shots.is_empty());
        let x = game.aliens[0].bounds.x;
        game.update(1.0, true, false, true);
        assert_eq!(game.aliens[0].bounds.x, x);
        assert!(game.player_shots.is_empty());
        game.lives = 1;
        game.enemy_shots.push(shot);
        game.update(0.1, false, false, false);
        assert_eq!(game.outcome, Outcome::Lost);
        assert_eq!(game.events.last(), Some(&GameEvent::Lose));
        game.update(1.0, false, false, true);
        assert_eq!(game.lives, 0);
    }

    #[test]
    fn last_alien_wins_and_landing_ends_game() {
        let mut game = Invaders::default();
        game.aliens.truncate(1);
        game.player_shots.push(Rect { x: 120.0, y: 180.0, width: 4.0, height: 10.0 });
        game.update(0.05, false, false, false);
        assert_eq!(game.outcome, Outcome::Won);
        assert_eq!(game.score, 100);
        let x = game.player.x;
        game.update(1.0, true, false, true);
        assert_eq!(game.player.x, x);
        let mut game = Invaders::default();
        game.aliens[0].bounds.y = PLAYER_Y - 24.0;
        game.update(DT, false, false, false);
        assert_eq!(game.outcome, Outcome::Lost);
    }

    #[test]
    fn missed_projectiles_are_removed_outside_the_playfield() {
        let mut game = Invaders::default();
        game.player_shots.push(Rect { x: LEFT, y: TOP - 20.0, width: 4.0, height: 10.0 });
        game.enemy_shots.push(Rect { x: LEFT, y: BOTTOM, width: 6.0, height: 12.0 });
        game.update(DT, false, false, false);
        assert!(game.player_shots.is_empty());
        assert!(game.enemy_shots.is_empty());
    }

    #[test]
    fn last_life_loss_takes_priority_over_destroying_last_alien() {
        let mut game = Invaders::default();
        game.lives = 1;
        game.aliens.truncate(1);
        game.player_shots.push(Rect { x: 120.0, y: 180.0, width: 4.0, height: 10.0 });
        game.enemy_shots.push(Rect { x: 395.0, y: 525.0, width: 6.0, height: 12.0 });
        game.update(0.05, false, false, false);
        assert!(game.aliens.is_empty());
        assert_eq!(game.score, 100);
        assert_eq!(game.outcome, Outcome::Lost);
    }

    #[test]
    fn sustained_play_keeps_formation_and_projectiles_bounded() {
        let mut game = Invaders::default();
        for _ in 0..6000 {
            game.update(DT, false, false, true);
            assert!(game.player_shots.len() <= 4);
            assert!(game.enemy_shots.len() <= 3);
            assert!(game.aliens.iter().all(|alien| {
                alien.bounds.x >= LEFT && alien.bounds.x + alien.bounds.width <= RIGHT
            }));
            if game.outcome != Outcome::Playing {
                break;
            }
        }
        assert_ne!(game.outcome, Outcome::Playing);
    }
}
