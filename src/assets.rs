use crate::controls::Game;
use crate::renderer::Color;
pub use crate::geometry::Facing;
use macroquad::prelude::Texture2D;
#[cfg(not(test))]
use macroquad::prelude::FilterMode;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sprite {
    Orb,
    Pursuer,
    Ship,
    Alien,
    Miner,
    Key,
    Exit,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Sequence {
    frames: Vec<String>,
    frame_seconds: f64,
    #[serde(default)]
    directions: BTreeMap<Facing, Vec<String>>,
    #[serde(default)]
    colors: Vec<[f32; 4]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    sequences: BTreeMap<String, Sequence>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    sprites: BTreeMap<Sprite, Definition>,
}

pub struct Frame {
    #[cfg(test)]
    pub name: String,
    pixels: Vec<u8>,
    texture: Option<Texture2D>,
}

impl Frame {
    pub fn texture(&self) -> &Texture2D {
        self.texture.as_ref().expect("Game assets must be uploaded before GPU drawing")
    }
}

pub struct Sample<'a> {
    pub frame: &'a Frame,
    pub rotation: f32,
    pub flip_x: bool,
    pub tint: Color,
}

pub struct Animations {
    manifest: Manifest,
    frames: BTreeMap<String, Frame>,
    pub elapsed: f64,
}

fn required(game: Game) -> &'static [(Sprite, &'static [&'static str])] {
    match game {
        Game::Maze => &[(Sprite::Orb, &["idle", "move"]), (Sprite::Pursuer, &["idle", "move"])],
        Game::Invaders => &[(Sprite::Ship, &["idle", "move"]), (Sprite::Alien, &["idle"])],
        Game::Platform => &[
            (Sprite::Miner, &["idle", "move", "jump"]),
            (Sprite::Key, &["idle"]), (Sprite::Exit, &["idle"]),
        ],
    }
}

pub fn directory() -> Result<PathBuf, String> {
    match std::env::var_os("ARCADE_SPRITE_DIR") {
        Some(value) => {
            let path = PathBuf::from(value);
            if !path.is_absolute() { return Err("ARCADE_SPRITE_DIR must be absolute".to_owned()); }
            Ok(path)
        }
        _ => Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/sprites")),
    }
}

fn read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    if bytes.len() as u64 > limit { return Err(format!("{} exceeds {limit} bytes", path.display())); }
    Ok(bytes)
}

fn decode(path: &Path, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let source = std::str::from_utf8(bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let rows: Vec<_> = source.lines().collect();
    if rows.len() != 8 || rows.iter().any(|row| row.len() != 8) {
        return Err(format!("{}: expected eight rows of eight pixels", path.display()));
    }
    let mut pixels = Vec::with_capacity(256);
    for pixel in rows.iter().flat_map(|row| row.bytes()) {
        match pixel {
            b'#' => pixels.extend_from_slice(&[255, 255, 255, 255]),
            b'.' => pixels.extend_from_slice(&[0, 0, 0, 0]),
            _ => return Err(format!("{}: pixels must be '#' or '.'", path.display())),
        }
    }
    if !pixels.chunks_exact(4).any(|p| p[3] != 0) {
        return Err(format!("{}: frame must contain visible pixels", path.display()));
    }
    Ok(pixels)
}

impl Animations {
    pub fn load(directory: &Path, game: Game) -> Result<Self, String> {
        let name = match game { Game::Maze => "maze", Game::Invaders => "invaders", Game::Platform => "platform" };
        let path = directory.join(format!("{name}.animations.json"));
        let manifest: Manifest = serde_json::from_slice(&read(&path, 65536)?)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let result = Self::validate_and_load(directory, game, manifest);
        result.map_err(|e| format!("{}: {e}", path.display()))
    }

    fn validate_and_load(directory: &Path, game: Game, manifest: Manifest) -> Result<Self, String> {
        if manifest.version != 1 || manifest.sprites.len() != required(game).len() {
            return Err("Expected version 1 and exactly this game's sprite definitions".to_owned());
        }
        let mut frames = BTreeMap::new();
        for &(sprite, names) in required(game) {
            let definition = manifest.sprites.get(&sprite).ok_or_else(|| format!("Missing sprite {sprite:?}"))?;
            if definition.sequences.len() > 16 || names.iter().any(|name| !definition.sequences.contains_key(*name)) {
                return Err(format!("{sprite:?}: missing required sequence or more than 16 sequences"));
            }
            for sequence in definition.sequences.values() {
                if !sequence.frame_seconds.is_finite() || !(0.01..=10.0).contains(&sequence.frame_seconds)
                    || sequence.colors.len() > 32 || sequence.colors.iter().flatten().any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                {
                    return Err("Frame durations must be 0.01..10 seconds and palettes contain at most 32 RGBA colors in 0..1".to_owned());
                }
                for list in std::iter::once(&sequence.frames).chain(sequence.directions.values()) {
                    if list.is_empty() || list.len() > 32 { return Err("Frame lists must contain 1..32 filenames".to_owned()); }
                    for name in list {
                        let mut components = Path::new(name).components();
                        if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some()
                            || !name.ends_with(".sprite")
                        {
                            return Err(format!("Frame {name:?} must be a local .sprite filename"));
                        }
                        if !frames.contains_key(name) {
                            if frames.len() >= 128 { return Err("At most 128 unique frames per game".to_owned()); }
                            let path = directory.join(name);
                            let pixels = decode(&path, &read(&path, 1024)?)?;
                            frames.insert(name.clone(), Frame {
                                #[cfg(test)]
                                name: name.clone(),
                                pixels, texture: None,
                            });
                        }
                    }
                }
            }
        }
        Ok(Self { manifest, frames, elapsed: 0.0 })
    }

    #[cfg(not(test))]
    pub fn upload(&mut self) {
        for frame in self.frames.values_mut() {
            let texture = Texture2D::from_rgba8(8, 8, &frame.pixels);
            texture.set_filter(FilterMode::Nearest);
            frame.texture = Some(texture);
        }
    }

    pub fn advance(&mut self, dt: f32) {
        self.elapsed += f64::from(dt);
    }

    pub fn sample(&self, sprite: Sprite, sequence: &str, facing: Facing, tint: Color) -> Sample<'_> {
        let sequence = &self.manifest.sprites[&sprite].sequences[sequence];
        let explicit = sequence.directions.get(&facing);
        let frames = explicit.unwrap_or(&sequence.frames);
        let index = (self.elapsed / sequence.frame_seconds).floor() as usize;
        let tint = if sequence.colors.is_empty() { tint } else {
            let [r, g, b, a] = sequence.colors[index % sequence.colors.len()];
            Color::new(r, g, b, a)
        };
        let (rotation, flip_x) = if explicit.is_some() { (0.0, false) } else {
            match facing {
                Facing::Right => (0.0, false),
                Facing::Left => (0.0, true),
                Facing::Up => (-std::f32::consts::FRAC_PI_2, false),
                Facing::Down => (std::f32::consts::FRAC_PI_2, false),
            }
        };
        Sample { frame: &self.frames[&frames[index % frames.len()]], rotation, flip_x, tint }
    }
}

#[cfg(test)]
impl Animations {
    pub fn fixture(game: Game) -> Self {
        Self::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/sprites"), game).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{GOLD, SKYBLUE, WHITE};

    #[test]
    fn all_shipped_frames_decode_visible_white_and_transparent_pixels() {
        for game in [Game::Maze, Game::Invaders, Game::Platform] {
            let assets = Animations::fixture(game);
            for frame in assets.frames.values() {
                assert_eq!(frame.pixels.len(), 256);
                assert!(frame.pixels.chunks_exact(4).any(|p| p == [255, 255, 255, 255]));
                assert!(frame.pixels.chunks_exact(4).any(|p| p == [0, 0, 0, 0]));
            }
        }
    }

    #[test]
    fn exact_frame_boundaries_wrap_and_draws_do_not_advance_time() {
        let mut assets = Animations::fixture(Game::Maze);
        for (time, name) in [(0.0, "orb.sprite"), (0.119, "orb.sprite"),
            (0.12, "orb-open.sprite"), (0.239, "orb-open.sprite"), (0.24, "orb.sprite")]
        {
            assets.elapsed = time;
            for _ in 0..3 {
                assert_eq!(assets.sample(Sprite::Orb, "move", Facing::Right, WHITE).frame.name, name);
                assert_eq!(assets.elapsed, time);
            }
        }
    }

    #[test]
    fn direction_overrides_replace_transforms_and_default_art_faces_right() {
        let assets = Animations::fixture(Game::Maze);
        let left = assets.sample(Sprite::Orb, "move", Facing::Left, WHITE);
        assert!(left.flip_x);
        assert_eq!(left.rotation, 0.0);
        assert_eq!(assets.sample(Sprite::Orb, "move", Facing::Up, WHITE).rotation,
            -std::f32::consts::FRAC_PI_2);
        assert_eq!(assets.sample(Sprite::Orb, "move", Facing::Down, WHITE).rotation,
            std::f32::consts::FRAC_PI_2);
        let up = assets.sample(Sprite::Pursuer, "move", Facing::Up, WHITE);
        assert!(!up.flip_x);
        assert_eq!(up.rotation, 0.0);
    }

    #[test]
    fn key_palette_cycles_independently_of_frame_count_and_other_tints_are_preserved() {
        let mut assets = Animations::fixture(Game::Platform);
        assert_eq!(assets.sample(Sprite::Key, "idle", Facing::Right, WHITE).tint, GOLD);
        assets.elapsed = 0.15;
        assert_eq!(assets.sample(Sprite::Key, "idle", Facing::Right, WHITE).tint, SKYBLUE);
        assets.elapsed = 0.6;
        assert_eq!(assets.sample(Sprite::Key, "idle", Facing::Right, WHITE).tint, GOLD);
        assert_eq!(assets.sample(Sprite::Exit, "idle", Facing::Right, WHITE).tint, WHITE);
    }

    #[test]
    fn invalid_masks_are_rejected() {
        let path = Path::new("bad.sprite");
        for bytes in [b".".as_slice(), b"........\n".repeat(8).as_slice(),
            b".......x\n".repeat(8).as_slice()]
        {
            assert!(decode(path, bytes).unwrap_err().contains("bad.sprite"));
        }
    }

    #[test]
    fn invalid_sequences_paths_versions_and_palettes_are_rejected() {
        let source = include_str!("../assets/sprites/maze.animations.json");
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/sprites");
        for changed in [
            source.replace("\"version\": 1", "\"version\": 2"),
            source.replace("0.12", "0"),
            source.replace("0.12", "11"),
            source.replace("[\"orb.sprite\"]", "[]"),
            source.replace("orb-open.sprite", "../orb.sprite"),
            source.replace("orb-open.sprite", "/tmp/orb.sprite"),
            source.replace("orb-open.sprite", "missing.sprite"),
            source.replace("\"idle\":", "\"unknown\":"),
            source.replace("\"frame_seconds\": 0.12", "\"frame_seconds\": 0.12, \"colors\": [[2, 0, 0, 1]]"),
        ] {
            let manifest = serde_json::from_str(&changed).unwrap();
            assert!(Animations::validate_and_load(&dir, Game::Maze, manifest).is_err());
        }
        assert!(serde_json::from_str::<Manifest>(&source.replace("\"frame_seconds\": 0.12",
            "\"frame_seconds\": 0.12, \"unexpected\": true")).is_err());
        assert!(serde_json::from_str::<Manifest>(&source.replace("\"directions\": {",
            "\"directions\": { \"diagonal\": [\"orb.sprite\"],")).is_err());
    }

    #[test]
    fn disk_edits_reload_without_mutating_active_snapshot_and_errors_can_be_retried() {
        let dir = std::env::temp_dir().join(format!("arcade-animation-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let source_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/sprites");
        for name in ["maze.animations.json", "orb.sprite", "orb-open.sprite", "pursuer.sprite", "pursuer-step.sprite"] {
            std::fs::copy(source_dir.join(name), dir.join(name)).unwrap();
        }
        let active = Animations::load(&dir, Game::Maze).unwrap();
        let changed = b"########\n".repeat(8);
        std::fs::write(dir.join("orb.sprite"), &changed).unwrap();
        let restarted = Animations::load(&dir, Game::Maze).unwrap();
        assert_ne!(active.frames["orb.sprite"].pixels, restarted.frames["orb.sprite"].pixels);
        let path = dir.join("maze.animations.json");
        std::fs::write(&path, "{").unwrap();
        assert!(Animations::load(&dir, Game::Maze).err().unwrap().contains("maze.animations.json"));
        std::fs::copy(source_dir.join("maze.animations.json"), &path).unwrap();
        assert!(Animations::load(&dir, Game::Maze).is_ok());
        std::fs::write(&path, vec![b' '; 65537]).unwrap();
        assert!(Animations::load(&dir, Game::Maze).err().unwrap().contains("exceeds"));
        for name in ["maze.animations.json", "orb.sprite", "orb-open.sprite", "pursuer.sprite", "pursuer-step.sprite"] {
            std::fs::remove_file(dir.join(name)).unwrap();
        }
        std::fs::remove_dir(dir).unwrap();
    }
}
