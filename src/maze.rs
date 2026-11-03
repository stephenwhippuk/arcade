use crate::events::GameEvent;
use crate::levels::MazeLevel;
use std::collections::VecDeque;

pub const WIDTH: usize = 19;
pub const HEIGHT: usize = 15;
const MOVE_SECONDS: f32 = 0.14;
const RESPAWN_SECONDS: f32 = 1.0;
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tile {
    pub x: usize,
    pub y: usize,
}

impl Tile {
    fn index(self) -> usize {
        self.y * WIDTH + self.x
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn neighbor(self, tile: Tile) -> Option<Tile> {
        match self {
            Self::Left if tile.x > 0 => Some(Tile { x: tile.x - 1, ..tile }),
            Self::Right if tile.x + 1 < WIDTH => Some(Tile { x: tile.x + 1, ..tile }),
            Self::Up if tile.y > 0 => Some(Tile { y: tile.y - 1, ..tile }),
            Self::Down if tile.y + 1 < HEIGHT => Some(Tile { y: tile.y + 1, ..tile }),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Won,
    Lost,
}

#[derive(Clone, Copy)]
pub struct Actor {
    pub tile: Tile,
    previous: Tile,
    pub facing: crate::geometry::Facing,
}

impl Actor {
    fn new(tile: Tile) -> Self {
        Self { tile, previous: tile, facing: crate::geometry::Facing::Right }
    }

    fn move_to(&mut self, tile: Tile) {
        use crate::geometry::Facing;
        if tile.x < self.tile.x { self.facing = Facing::Left; }
        else if tile.x > self.tile.x { self.facing = Facing::Right; }
        else if tile.y < self.tile.y { self.facing = Facing::Up; }
        else if tile.y > self.tile.y { self.facing = Facing::Down; }
        self.previous = self.tile;
        self.tile = tile;
    }

    pub fn position(&self, fraction: f32) -> (f32, f32) {
        let x = self.previous.x as f32;
        let y = self.previous.y as f32;
        (
            x + (self.tile.x as f32 - x) * fraction,
            y + (self.tile.y as f32 - y) * fraction,
        )
    }

    pub fn is_moving(&self) -> bool {
        self.tile != self.previous
    }
}

pub struct Maze {
    pub level: MazeLevel,
    pub events: Vec<GameEvent>,
    pellets: [bool; WIDTH * HEIGHT],
    pub player: Actor,
    pub enemies: [Actor; 2],
    pub score: u32,
    pub lives: u8,
    pub remaining: usize,
    pub outcome: Outcome,
    direction: Option<Direction>,
    requested: Option<Direction>,
    accumulator: f32,
    enemy_turn: bool,
    respawn: f32,
}

#[cfg(test)]
impl Default for Maze {
    fn default() -> Self {
        Self::new(crate::levels::maze_fixture())
    }
}

impl Maze {
    pub fn new(level: MazeLevel) -> Self {
        let mut pellets = [false; WIDTH * HEIGHT];
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let tile = Tile { x, y };
                pellets[tile.index()] = level.rows[y].as_bytes()[x] == b'.'
                    && tile != level.player_start
                    && !level.enemy_starts.contains(&tile);
            }
        }
        Self {
            events: Vec::new(),
            remaining: pellets.iter().filter(|&&pellet| pellet).count(),
            pellets,
            player: Actor::new(level.player_start),
            enemies: level.enemy_starts.map(Actor::new),
            level,
            score: 0,
            lives: 3,
            outcome: Outcome::Playing,
            direction: None,
            requested: None,
            accumulator: 0.0,
            enemy_turn: false,
            respawn: 0.0,
        }
    }
}

impl Maze {
    #[cfg(test)]
    fn is_wall(tile: Tile) -> bool {
        crate::levels::maze_fixture().is_wall(tile)
    }

    pub fn has_pellet(&self, tile: Tile) -> bool {
        !self.level.is_wall(tile) && self.pellets[tile.index()]
    }

    pub fn request_direction(&mut self, direction: Direction) {
        self.requested = Some(direction);
    }

    pub fn animation_fraction(&self) -> f32 {
        if self.outcome != Outcome::Playing || self.respawn > 0.0 {
            1.0
        } else {
            self.accumulator / MOVE_SECONDS
        }
    }

    pub fn is_respawning(&self) -> bool {
        self.respawn > 0.0
    }

    pub fn update(&mut self, dt: f32) {
        if self.outcome != Outcome::Playing {
            return;
        }
        if self.respawn > 0.0 {
            self.respawn = (self.respawn - dt).max(0.0);
            return;
        }
        self.accumulator += dt;
        while self.accumulator >= MOVE_SECONDS {
            self.accumulator -= MOVE_SECONDS;
            self.step();
            if self.outcome != Outcome::Playing || self.respawn > 0.0 {
                self.accumulator = 0.0;
                break;
            }
        }
    }

    fn level_destination(level: &MazeLevel, tile: Tile, direction: Direction) -> Option<Tile> {
        direction.neighbor(tile).filter(|&next| !level.is_wall(next))
    }

    #[cfg(test)]
    fn destination(tile: Tile, direction: Direction) -> Option<Tile> {
        Self::level_destination(&crate::levels::maze_fixture(), tile, direction)
    }

    fn step(&mut self) {
        if let Some(requested) = self.requested {
            if Self::level_destination(&self.level, self.player.tile, requested).is_some() {
                self.direction = Some(requested);
                self.requested = None;
            }
        }
        let next = self.direction
            .and_then(|direction| Self::level_destination(&self.level, self.player.tile, direction))
            .unwrap_or(self.player.tile);
        self.player.move_to(next);
        self.enemy_turn = !self.enemy_turn;
        for enemy in &mut self.enemies {
            let next = if self.enemy_turn {
                Self::level_chase_step(&self.level, enemy.tile, self.player.previous)
            } else {
                enemy.tile
            };
            enemy.move_to(next);
        }

        // Catch both shared destinations and actors crossing the same corridor edge.
        let hit = self.enemies.iter().any(|enemy| {
            enemy.tile == self.player.tile
                || (enemy.tile == self.player.previous
                    && enemy.previous == self.player.tile)
        });
        if hit {
            self.lives -= 1;
            self.events.push(GameEvent::Hit);
            if self.lives == 0 {
                self.outcome = Outcome::Lost;
                self.events.push(GameEvent::Lose);
            } else {
                self.player = Actor::new(self.level.player_start);
                self.enemies = self.level.enemy_starts.map(Actor::new);
                self.direction = None;
                self.requested = None;
                self.enemy_turn = false;
                self.respawn = RESPAWN_SECONDS;
            }
            return;
        }
        if self.pellets[self.player.tile.index()] {
            self.pellets[self.player.tile.index()] = false;
            self.score += 10;
            self.events.push(GameEvent::Collect);
            self.remaining -= 1;
            if self.remaining == 0 {
                self.outcome = Outcome::Won;
                self.events.push(GameEvent::Win);
            }
        }
    }

    #[cfg(test)]
    fn chase_step(start: Tile, target: Tile) -> Tile {
        Self::level_chase_step(&crate::levels::maze_fixture(), start, target)
    }

    fn level_chase_step(level: &MazeLevel, start: Tile, target: Tile) -> Tile {
        let mut visited = [false; WIDTH * HEIGHT];
        let mut queue = VecDeque::from([(start, start)]);
        visited[start.index()] = true;
        while let Some((tile, first_step)) = queue.pop_front() {
            if tile == target {
                return first_step;
            }
            for direction in [Direction::Up, Direction::Left, Direction::Down, Direction::Right] {
                if let Some(next) = Self::level_destination(level, tile, direction) {
                    if !visited[next.index()] {
                        visited[next.index()] = true;
                        queue.push_back((next, if tile == start { next } else { first_step }));
                    }
                }
            }
        }
        start
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PLAYER_START: Tile = Tile { x: 1, y: 1 };
    const ENEMY_STARTS: [Tile; 2] = [Tile { x: 17, y: 13 }, Tile { x: 17, y: 1 }];

    fn safe_game() -> Maze {
        Maze { enemy_turn: true, ..Maze::default() }
    }

    #[test]
    fn map_is_enclosed_and_every_pellet_and_spawn_is_reachable() {
        assert!(crate::levels::maze_fixture().rows.iter().all(|row| row.len() == WIDTH));
        for x in 0..WIDTH {
            assert!(Maze::is_wall(Tile { x, y: 0 }));
            assert!(Maze::is_wall(Tile { x, y: HEIGHT - 1 }));
        }
        let mut visited = [false; WIDTH * HEIGHT];
        let mut queue = VecDeque::from([PLAYER_START]);
        visited[PLAYER_START.index()] = true;
        while let Some(tile) = queue.pop_front() {
            for direction in [Direction::Up, Direction::Left, Direction::Down, Direction::Right] {
                if let Some(next) = Maze::destination(tile, direction) {
                    if !visited[next.index()] {
                        visited[next.index()] = true;
                        queue.push_back(next);
                    }
                }
            }
        }
        for y in 0..HEIGHT {
            assert!(Maze::is_wall(Tile { x: 0, y }));
            assert!(Maze::is_wall(Tile { x: WIDTH - 1, y }));
            for x in 0..WIDTH {
                let tile = Tile { x, y };
                assert!(Maze::is_wall(tile) || visited[tile.index()], "{tile:?} is unreachable");
            }
        }
        let game = Maze::default();
        assert!(!game.has_pellet(PLAYER_START));
        assert!(ENEMY_STARTS.iter().all(|&tile| !Maze::is_wall(tile)));
        assert_eq!(game.remaining, game.pellets.iter().filter(|&&p| p).count());
    }

    #[test]
    fn walls_block_movement_and_pellets_score_only_once() {
        let mut game = safe_game();
        game.request_direction(Direction::Up);
        game.update(MOVE_SECONDS);
        assert_eq!(game.player.tile, PLAYER_START);
        game.request_direction(Direction::Right);
        game.update(MOVE_SECONDS);
        assert_eq!(game.player.tile, Tile { x: 2, y: 1 });
        assert_eq!(game.score, 10);
        game.request_direction(Direction::Left);
        game.update(MOVE_SECONDS);
        game.request_direction(Direction::Right);
        game.update(MOVE_SECONDS);
        assert_eq!(game.score, 10);
    }

    #[test]
    fn requested_turn_waits_until_an_open_junction() {
        let mut game = safe_game();
        game.request_direction(Direction::Right);
        game.update(MOVE_SECONDS);
        game.request_direction(Direction::Down);
        for _ in 0..3 {
            game.update(MOVE_SECONDS);
        }
        assert_eq!(game.player.tile, Tile { x: 5, y: 1 });
        game.update(MOVE_SECONDS);
        assert_eq!(game.player.tile, Tile { x: 5, y: 2 });
    }

    #[test]
    fn enemies_follow_corridors_at_half_player_speed() {
        let mut game = Maze::default();
        let start = game.enemies[0].tile;
        game.update(MOVE_SECONDS);
        let next = game.enemies[0].tile;
        assert_ne!(next, start);
        assert!(!Maze::is_wall(next));
        assert_eq!(next.x.abs_diff(start.x) + next.y.abs_diff(start.y), 1);
        game.update(MOVE_SECONDS);
        assert_eq!(game.enemies[0].tile, next);
        assert_eq!(Maze::chase_step(PLAYER_START, Tile { x: 5, y: 1 }), Tile { x: 2, y: 1 });
    }

    #[test]
    fn collision_respawns_without_resetting_score_and_last_life_ends_game() {
        let mut game = safe_game();
        game.score = 50;
        game.enemies[0] = Actor::new(Tile { x: 2, y: 1 });
        game.request_direction(Direction::Right);
        game.update(MOVE_SECONDS);
        assert_eq!(game.lives, 2);
        assert_eq!(game.score, 50);
        assert_eq!(game.player.tile, PLAYER_START);
        assert!(game.is_respawning());
        game.update(MOVE_SECONDS);
        assert_eq!(game.lives, 2);
        game.update(RESPAWN_SECONDS);
        game.lives = 1;
        game.enemy_turn = true;
        game.enemies[0] = Actor::new(Tile { x: 2, y: 1 });
        game.request_direction(Direction::Right);
        game.update(MOVE_SECONDS);
        assert_eq!(game.outcome, Outcome::Lost);
        game.update(10.0);
        assert_eq!(game.lives, 0);
    }

    #[test]
    fn swapping_tiles_with_an_enemy_is_a_collision() {
        let mut game = Maze::default();
        game.enemies[0] = Actor::new(Tile { x: 2, y: 1 });
        game.request_direction(Direction::Right);
        game.update(MOVE_SECONDS);
        assert_eq!(game.lives, 2);
    }

    #[test]
    fn collecting_last_pellet_wins_and_freezes_the_game() {
        let mut game = safe_game();
        game.pellets.fill(false);
        game.pellets[Tile { x: 2, y: 1 }.index()] = true;
        game.remaining = 1;
        game.request_direction(Direction::Right);
        game.update(MOVE_SECONDS);
        assert_eq!(game.outcome, Outcome::Won);
        assert_eq!(game.events, vec![GameEvent::Collect, GameEvent::Win]);
        assert_eq!(game.score, 10);
        game.update(10.0);
        assert_eq!(game.events, vec![GameEvent::Collect, GameEvent::Win]);
        assert_eq!(game.player.tile, Tile { x: 2, y: 1 });
    }

    #[test]
    fn animation_stays_between_adjacent_tiles() {
        let mut game = safe_game();
        game.request_direction(Direction::Right);
        game.update(MOVE_SECONDS * 1.5);
        let (x, y) = game.player.position(game.animation_fraction());
        assert!((x - 1.5).abs() < 0.001);
        assert_eq!(y, 1.0);
    }
}
