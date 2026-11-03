use crate::geometry::Rect;
use crate::maze::{Tile, HEIGHT, WIDTH};
use crate::platform::{BOTTOM, LEFT, RIGHT, TOP};
use serde::Deserialize;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MazeLevel {
    version: u32,
    pub rows: Vec<String>,
    pub player_start: Tile,
    pub enemy_starts: [Tile; 2],
}

impl MazeLevel {
    pub fn is_wall(&self, tile: Tile) -> bool {
        tile.x >= WIDTH || tile.y >= HEIGHT || self.rows[tile.y].as_bytes()[tile.x] == b'#'
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.rows.len() != HEIGHT
            || self.rows.iter().any(|row| row.len() != WIDTH || !row.bytes().all(|p| matches!(p, b'#' | b'.' | b' ')))
        {
            return Err("Maze requires version 1 and 15 rows of 19 ASCII '#', '.' or space cells".to_owned());
        }
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                if (x == 0 || y == 0 || x == WIDTH - 1 || y == HEIGHT - 1)
                    && !self.is_wall(Tile { x, y })
                {
                    return Err("Maze boundary must be enclosed by walls".to_owned());
                }
            }
        }
        let spawns = [self.player_start, self.enemy_starts[0], self.enemy_starts[1]];
        for (index, tile) in spawns.iter().enumerate() {
            if self.is_wall(*tile) || spawns[..index].contains(tile) {
                return Err("Maze spawns must be distinct open cells inside the map".to_owned());
            }
        }
        let mut visited = [false; WIDTH * HEIGHT];
        let mut queue = VecDeque::from([self.player_start]);
        visited[self.player_start.y * WIDTH + self.player_start.x] = true;
        while let Some(tile) = queue.pop_front() {
            for (x, y) in [(tile.x - 1, tile.y), (tile.x + 1, tile.y), (tile.x, tile.y - 1), (tile.x, tile.y + 1)] {
                let next = Tile { x, y };
                if !self.is_wall(next) && !visited[y * WIDTH + x] {
                    visited[y * WIDTH + x] = true;
                    queue.push_back(next);
                }
            }
        }
        let mut pellets = 0;
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let tile = Tile { x, y };
                if !self.is_wall(tile) && !visited[y * WIDTH + x] {
                    return Err(format!("Unreachable maze cell at ({x}, {y})"));
                }
                if self.rows[y].as_bytes()[x] == b'.' && !spawns.contains(&tile) {
                    pellets += 1;
                }
            }
        }
        if pellets == 0 { return Err("Maze must contain at least one collectible pellet".to_owned()); }
        Ok(())
    }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformLevel {
    version: u32,
    pub air_seconds: f64,
    pub player_start: Rect,
    pub platforms: Vec<Rect>,
    pub keys: Vec<Rect>,
    pub exit: Rect,
}

impl PlatformLevel {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || !self.air_seconds.is_finite() || self.air_seconds <= 0.0
            || self.air_seconds > 3600.0 || self.platforms.is_empty()
            || self.platforms.len() > 128 || self.keys.is_empty() || self.keys.len() > 128
        {
            return Err("Platform level requires version 1, 0 < air <= 3600 seconds, and 1..128 platforms and keys".to_owned());
        }
        for rect in self.platforms.iter().chain(&self.keys).chain([&self.player_start, &self.exit]) {
            if ![rect.x, rect.y, rect.width, rect.height].iter().all(|v| v.is_finite())
                || rect.width <= 0.0 || rect.height <= 0.0 || rect.x < LEFT || rect.y < TOP
                || rect.x + rect.width > RIGHT || rect.y + rect.height > BOTTOM
            {
                return Err("All platform rectangles must have positive sizes and fit inside the playfield".to_owned());
            }
        }
        if !self.platforms.iter().any(|p| p.x == LEFT && p.width == RIGHT - LEFT && p.y + p.height == BOTTOM) {
            return Err("Platform level needs a continuous floor spanning the bottom of the playfield".to_owned());
        }
        for (index, p) in self.platforms.iter().enumerate() {
            if self.platforms[..index].iter().any(|other| p.overlaps(*other)) {
                return Err("Solid platforms must not overlap".to_owned());
            }
            if p.overlaps(self.player_start) || p.overlaps(self.exit)
                || self.keys.iter().any(|key| p.overlaps(*key))
            {
                return Err("Player, keys and exit must not intersect solid platforms".to_owned());
            }
        }
        let supported = |rect: Rect| self.platforms.iter().any(|p| {
            (rect.y + rect.height - p.y).abs() < 0.001
                && rect.x < p.x + p.width && rect.x + rect.width > p.x
        });
        if !supported(self.player_start) || !supported(self.exit) {
            return Err("Player spawn and exit must stand on a platform".to_owned());
        }
        for (index, key) in self.keys.iter().enumerate() {
            if self.keys[..index].iter().any(|other| key.overlaps(*other)) || key.overlaps(self.exit) {
                return Err("Keys must not overlap one another or the exit".to_owned());
            }
        }
        Ok(())
    }
}

pub fn directory() -> Result<PathBuf, String> {
    match std::env::var_os("ARCADE_LEVEL_DIR") {
        Some(path) if !path.is_empty() => {
            let path = PathBuf::from(path);
            if !path.is_absolute() { return Err("ARCADE_LEVEL_DIR must be absolute".to_owned()); }
            Ok(path)
        }
        _ => Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/levels")),
    }
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    if bytes.len() > 65536 {
        return Err(format!("{}: level files must be at most 64 KiB", path.display()));
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("Cannot parse {}: {e}", path.display()))
}

pub fn load_maze(path: &Path) -> Result<MazeLevel, String> {
    let level: MazeLevel = read(path)?;
    level.validate().map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(level)
}

pub fn load_platform(path: &Path) -> Result<PlatformLevel, String> {
    let level: PlatformLevel = read(path)?;
    level.validate().map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(level)
}

#[cfg(test)]
pub fn maze_fixture() -> MazeLevel {
    let level: MazeLevel = serde_json::from_str(include_str!("../assets/levels/maze.json")).unwrap();
    level.validate().unwrap();
    level
}

#[cfg(test)]
pub fn platform_fixture() -> PlatformLevel {
    let level: PlatformLevel = serde_json::from_str(include_str!("../assets/levels/platform.json")).unwrap();
    level.validate().unwrap();
    level
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controls::tests::TestDirectory;

    #[test]
    fn shipped_levels_validate_and_preserve_the_current_layouts() {
        let maze = maze_fixture();
        assert_eq!(maze.player_start, Tile { x: 1, y: 1 });
        assert_eq!(maze.enemy_starts, [Tile { x: 17, y: 13 }, Tile { x: 17, y: 1 }]);
        let platform = platform_fixture();
        assert_eq!(platform.platforms.len(), 5);
        assert_eq!(platform.keys.len(), 5);
        assert_eq!(platform.air_seconds, 60.0);
        assert_eq!((platform.player_start.x, platform.player_start.y), (70.0, 522.0));
        assert_eq!((platform.exit.x, platform.exit.y), (510.0, 172.0));
    }

    #[test]
    fn invalid_maze_dimensions_boundaries_spawns_and_disconnected_cells_are_rejected() {
        let original = maze_fixture();
        let mut level = original.clone();
        level.rows.pop();
        assert!(level.validate().is_err());
        level = original.clone();
        level.rows[1].replace_range(1..2, "x");
        assert!(level.validate().is_err());
        level = original.clone();
        level.rows[0].replace_range(1..2, ".");
        assert!(level.validate().unwrap_err().contains("boundary"));
        level = original.clone();
        level.player_start.x = usize::MAX;
        assert!(level.validate().is_err());
        level = original.clone();
        level.enemy_starts[0] = level.player_start;
        assert!(level.validate().is_err());
        level = original.clone();
        level.rows[1].replace_range(2..3, "#");
        level.rows[2].replace_range(1..2, "#");
        assert!(level.validate().unwrap_err().contains("Unreachable"));
        level = original.clone();
        for row in &mut level.rows { *row = row.replace('.', " "); }
        assert!(level.validate().unwrap_err().contains("pellet"));
        level = original;
        level.version = 99;
        assert!(level.validate().is_err());
    }

    #[test]
    fn platform_validation_rejects_unsafe_geometry_and_invalid_timer() {
        let original = platform_fixture();
        for air in [0.0, -1.0, f64::INFINITY, 3601.0] {
            let mut level = original.clone();
            level.air_seconds = air;
            assert!(level.validate().is_err());
        }
        let mut level = original.clone();
        level.player_start.width = -1.0;
        assert!(level.validate().is_err());
        level = original.clone();
        level.exit.x = RIGHT;
        assert!(level.validate().is_err());
        level = original.clone();
        level.platforms.remove(0);
        assert!(level.validate().unwrap_err().contains("floor"));
        level = original.clone();
        level.player_start.y -= 1.0;
        assert!(level.validate().unwrap_err().contains("stand"));
        level = original.clone();
        level.keys[0] = level.keys[1];
        assert!(level.validate().unwrap_err().contains("overlap"));
        level = original.clone();
        level.keys[0] = level.platforms[1];
        assert!(level.validate().unwrap_err().contains("solid"));
        level = original.clone();
        level.platforms.push(level.platforms[1]);
        assert!(level.validate().unwrap_err().contains("overlap"));
        level = original;
        level.keys.clear();
        assert!(level.validate().is_err());
    }

    #[test]
    fn disk_edits_are_loaded_again_without_rebuild_or_changing_an_active_game() {
        let directory = TestDirectory::new();
        let path = directory.0.join("platform.json");
        let mut document: serde_json::Value = serde_json::from_str(
            include_str!("../assets/levels/platform.json")).unwrap();
        std::fs::write(&path, document.to_string()).unwrap();
        let old_game = crate::platform::PlatformGame::new(load_platform(&path).unwrap());
        document["air_seconds"] = 12.0.into();
        document["player_start"]["x"] = 80.0.into();
        document["keys"].as_array_mut().unwrap().pop();
        std::fs::write(&path, document.to_string()).unwrap();
        let new_game = crate::platform::PlatformGame::new(load_platform(&path).unwrap());
        assert_eq!(old_game.air, 60.0);
        assert_eq!(old_game.keys.len(), 5);
        assert_eq!(new_game.air, 12.0);
        assert_eq!(new_game.keys.len(), 4);
        assert_eq!(new_game.player.x, 80.0);
    }

    #[test]
    fn maze_layout_and_respawn_use_loaded_data_not_old_constants() {
        let mut level = maze_fixture();
        level.player_start = Tile { x: 3, y: 1 };
        level.rows[1].replace_range(4..5, " ");
        level.validate().unwrap();
        let mut game = crate::maze::Maze::new(level);
        assert_eq!(game.player.tile, Tile { x: 3, y: 1 });
        assert!(!game.has_pellet(Tile { x: 4, y: 1 }));
        assert!(!game.has_pellet(game.player.tile));
        game.request_direction(crate::maze::Direction::Right);
        game.update(0.14);
        assert_eq!(game.player.tile, Tile { x: 4, y: 1 });
        assert_eq!(game.score, 0);
        game.enemies[0].tile = Tile { x: 5, y: 1 };
        game.request_direction(crate::maze::Direction::Right);
        game.update(0.14);
        assert_eq!(game.lives, 2);
        assert_eq!(game.player.tile, Tile { x: 3, y: 1 });
        assert_eq!(game.enemies[0].tile, game.level.enemy_starts[0]);
    }

    #[test]
    fn errors_include_path_and_never_substitute_default_levels() {
        let directory = TestDirectory::new();
        let path = directory.0.join("maze.json");
        assert!(load_maze(&path).err().unwrap().contains("maze.json"));
        std::fs::write(&path, "{invalid").unwrap();
        assert!(load_maze(&path).err().unwrap().contains("Cannot parse"));
        std::fs::write(&path, " ".repeat(65537)).unwrap();
        assert!(load_maze(&path).err().unwrap().contains("64 KiB"));
        std::fs::write(&path, include_str!("../assets/levels/maze.json")).unwrap();
        assert!(load_maze(&path).is_ok());
    }
}
