use crate::input::Action;
use macroquad::prelude::KeyCode;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Game {
    #[default]
    Maze,
    Invaders,
    Platform,
}

impl Game {
    pub fn title(self) -> &'static str {
        match self {
            Self::Maze => "Maze Chase",
            Self::Invaders => "Star Invaders",
            Self::Platform => "Key Cavern",
        }
    }

    pub fn actions(self) -> &'static [Action] {
        match self {
            Self::Maze => &[Action::MoveLeft, Action::MoveRight, Action::MoveUp, Action::MoveDown, Action::Pause],
            Self::Invaders => &[Action::MoveLeft, Action::MoveRight, Action::Fire, Action::Pause],
            Self::Platform => &[Action::MoveLeft, Action::MoveRight, Action::Jump, Action::Pause],
        }
    }
}

pub const KEYS: &[(&str, KeyCode)] = &[
    ("Left", KeyCode::Left), ("Right", KeyCode::Right),
    ("Up", KeyCode::Up), ("Down", KeyCode::Down), ("Space", KeyCode::Space),
    ("A", KeyCode::A), ("B", KeyCode::B), ("C", KeyCode::C),
    ("D", KeyCode::D), ("E", KeyCode::E), ("F", KeyCode::F),
    ("G", KeyCode::G), ("H", KeyCode::H), ("I", KeyCode::I),
    ("J", KeyCode::J), ("K", KeyCode::K), ("L", KeyCode::L),
    ("M", KeyCode::M), ("N", KeyCode::N), ("O", KeyCode::O),
    ("P", KeyCode::P), ("Q", KeyCode::Q), ("R", KeyCode::R),
    ("S", KeyCode::S), ("T", KeyCode::T), ("U", KeyCode::U),
    ("V", KeyCode::V), ("W", KeyCode::W), ("X", KeyCode::X),
    ("Y", KeyCode::Y), ("Z", KeyCode::Z),
    ("0", KeyCode::Key0), ("1", KeyCode::Key1), ("2", KeyCode::Key2),
    ("3", KeyCode::Key3), ("4", KeyCode::Key4), ("5", KeyCode::Key5),
    ("6", KeyCode::Key6), ("7", KeyCode::Key7), ("8", KeyCode::Key8),
    ("9", KeyCode::Key9), ("Tab", KeyCode::Tab),
    ("LeftShift", KeyCode::LeftShift), ("RightShift", KeyCode::RightShift),
    ("LeftControl", KeyCode::LeftControl), ("RightControl", KeyCode::RightControl),
    ("Enter", KeyCode::Enter), ("Escape", KeyCode::Escape),
    ("F1", KeyCode::F1), ("F3", KeyCode::F3),
];

pub fn key_name(key: KeyCode) -> Option<&'static str> {
    KEYS.iter().find(|(_, code)| *code == key).map(|(name, _)| *name)
}

fn reserved(name: &str) -> bool {
    matches!(name, "Escape" | "Enter" | "M" | "F1" | "F3")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub action: Action,
    pub keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Controls {
    version: u32,
    maze: Vec<Binding>,
    invaders: Vec<Binding>,
    platform: Vec<Binding>,
}

impl Default for Controls {
    fn default() -> Self {
        let binding = |action, keys: &[&str]| Binding {
            action, keys: keys.iter().map(|key| (*key).to_owned()).collect(),
        };
        let horizontal = || vec![
            binding(Action::MoveLeft, &["Left", "A"]),
            binding(Action::MoveRight, &["Right", "D"]),
        ];
        let mut maze = horizontal();
        maze.extend([
            binding(Action::MoveUp, &["Up", "W"]),
            binding(Action::MoveDown, &["Down", "S"]),
            binding(Action::Pause, &["P"]),
        ]);
        let mut invaders = horizontal();
        invaders.extend([binding(Action::Fire, &["Space"]), binding(Action::Pause, &["P"])]);
        let mut platform = horizontal();
        platform.extend([
            binding(Action::Jump, &["Space", "Up", "W"]),
            binding(Action::Pause, &["P"]),
        ]);
        Self { version: 1, maze, invaders, platform }
    }
}

impl Controls {
    pub fn bindings(&self, game: Game) -> &[Binding] {
        match game {
            Game::Maze => &self.maze,
            Game::Invaders => &self.invaders,
            Game::Platform => &self.platform,
        }
    }

    fn bindings_mut(&mut self, game: Game) -> &mut Vec<Binding> {
        match game {
            Game::Maze => &mut self.maze,
            Game::Invaders => &mut self.invaders,
            Game::Platform => &mut self.platform,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(format!("Unsupported controls version: {}", self.version));
        }
        for game in [Game::Maze, Game::Invaders, Game::Platform] {
            let bindings = self.bindings(game);
            if bindings.len() != game.actions().len() {
                return Err(format!("{} must define every gameplay action exactly once", game.title()));
            }
            let mut used = Vec::new();
            for &action in game.actions() {
                let matches: Vec<_> = bindings.iter().filter(|binding| binding.action == action).collect();
                if matches.len() != 1 || matches[0].keys.is_empty() {
                    return Err(format!("{}: missing or duplicate {action:?}", game.title()));
                }
                for key in &matches[0].keys {
                    if !KEYS.iter().any(|(name, _)| *name == key) || reserved(key) {
                        return Err(format!("{}: unsupported or reserved key {key}", game.title()));
                    }
                    if used.contains(key) {
                        return Err(format!("{}: duplicate key {key}", game.title()));
                    }
                    used.push(key.clone());
                }
            }
        }
        Ok(())
    }

    pub fn rebind(&mut self, game: Game, action: Action, key: &str) -> Result<(), String> {
        let mut candidate = self.clone();
        let binding = candidate.bindings_mut(game).iter_mut()
            .find(|binding| binding.action == action)
            .ok_or_else(|| format!("Action {action:?} is not used by {}", game.title()))?;
        binding.keys = vec![key.to_owned()];
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    pub fn reset(&mut self, game: Game) {
        *self.bindings_mut(game) = Self::default().bindings(game).to_vec();
    }

    pub fn hint(&self, game: Game) -> String {
        self.bindings(game).iter().map(|binding| {
            format!("{}: {}", binding.action.label(), binding.keys.join("/"))
        }).collect::<Vec<_>>().join("   ")
    }
}

pub struct ControlStore {
    pub controls: Controls,
    path: Option<PathBuf>,
}

impl ControlStore {
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let controls = match std::fs::read_to_string(&path) {
            Ok(text) => {
                let controls: Controls = serde_json::from_str(&text)
                    .map_err(|error| format!("Cannot parse {}: {error}", path.display()))?;
                controls.validate().map_err(|error| format!("{}: {error}", path.display()))?;
                controls
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Controls::default(),
            Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
        };
        Ok(Self { controls, path: Some(path) })
    }

    pub fn save(&mut self, candidate: Controls) -> Result<(), String> {
        candidate.validate()?;
        let path = self.path.as_ref().ok_or_else(|| "No persistent controls path configured".to_owned())?;
        save_file(path, &candidate)?;
        self.controls = candidate;
        Ok(())
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self { controls: Controls::default(), path: None }
    }
}

fn save_file(path: &Path, controls: &Controls) -> Result<(), String> {
    let directory = path.parent().ok_or_else(|| "Controls path has no parent directory".to_owned())?;
    std::fs::create_dir_all(directory).map_err(|error| format!("Cannot create {}: {error}", directory.display()))?;
    let text = serde_json::to_vec_pretty(controls).map_err(|error| format!("Cannot encode controls: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    let result = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        let written = (|| {
            file.write_all(&text)?;
            file.sync_all()?;
            std::fs::rename(&temporary, path)
        })();
        if written.is_err() {
            std::fs::remove_file(&temporary)?;
        }
        written
    })();
    result.map_err(|error| format!("Cannot save {}: {error}", path.display()))
}

pub fn config_path() -> Result<PathBuf, String> {
    if let Some(directory) = std::env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        let directory = PathBuf::from(directory);
        if !directory.is_absolute() {
            return Err("XDG_CONFIG_HOME must be an absolute path".to_owned());
        }
        return Ok(directory.join("arcade/controls.json"));
    }
    std::env::var_os("HOME").filter(|value| !value.is_empty())
        .map(|home| PathBuf::from(home).join(".config/arcade/controls.json"))
        .ok_or_else(|| "Cannot locate controls configuration: set HOME or XDG_CONFIG_HOME".to_owned())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub struct TestDirectory(pub PathBuf);

    impl TestDirectory {
        pub fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!("arcade-controls-{}-{}",
                std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn defaults_are_valid_and_cross_game_keys_do_not_conflict() {
        let controls = Controls::default();
        controls.validate().unwrap();
        assert!(controls.hint(Game::Invaders).contains("Fire: Space"));
        assert!(controls.hint(Game::Platform).contains("Jump: Space/Up/W"));
    }

    #[test]
    fn remapping_replaces_aliases_only_in_selected_game() {
        let mut controls = Controls::default();
        let original = controls.clone();
        controls.rebind(Game::Maze, Action::MoveLeft, "J").unwrap();
        assert_eq!(controls.maze[0].keys, vec!["J"]);
        assert_eq!(controls.invaders, original.invaders);
        assert_eq!(controls.platform, original.platform);
        controls.reset(Game::Maze);
        assert_eq!(controls, original);
    }

    #[test]
    fn conflicts_reserved_keys_and_unknown_names_do_not_change_controls() {
        let mut controls = Controls::default();
        let original = controls.clone();
        for key in ["Right", "D", "Escape", "Enter", "M", "F1", "F3", "NotAKey"] {
            assert!(controls.rebind(Game::Maze, Action::MoveLeft, key).is_err(), "{key}");
            assert_eq!(controls, original);
        }
    }

    #[test]
    fn invalid_profile_shapes_and_versions_are_rejected() {
        let mut controls = Controls::default();
        controls.version = 2;
        assert!(controls.validate().is_err());
        controls.version = 1;
        controls.maze[0].keys.clear();
        assert!(controls.validate().is_err());
        controls = Controls::default();
        controls.maze[0].action = Action::MoveRight;
        assert!(controls.validate().is_err());
        assert!(serde_json::from_str::<Controls>(r#"{"version":1,"unexpected":true}"#).is_err());
    }

    #[test]
    fn missing_file_defaults_then_save_and_reload_restore_each_profile() {
        let directory = TestDirectory::new();
        let path = directory.0.join("nested/controls.json");
        let mut store = ControlStore::load(path.clone()).unwrap();
        assert_eq!(store.controls, Controls::default());
        assert!(!path.exists());
        let mut candidate = store.controls.clone();
        candidate.rebind(Game::Maze, Action::MoveLeft, "J").unwrap();
        candidate.rebind(Game::Invaders, Action::Fire, "K").unwrap();
        candidate.rebind(Game::Platform, Action::Jump, "L").unwrap();
        store.save(candidate.clone()).unwrap();
        let restored = ControlStore::load(path.clone()).unwrap();
        assert_eq!(restored.controls, candidate);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn malformed_existing_file_is_reported_and_not_overwritten() {
        let directory = TestDirectory::new();
        let path = directory.0.join("controls.json");
        std::fs::write(&path, "{broken").unwrap();
        let error = ControlStore::load(path.clone()).err().unwrap();
        assert!(error.contains("Cannot parse"));
        assert!(error.contains("controls.json"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{broken");
    }

    #[test]
    fn syntactically_valid_but_invalid_saved_bindings_are_reported() {
        let directory = TestDirectory::new();
        let path = directory.0.join("controls.json");
        let mut controls = Controls::default();
        controls.maze[0].keys = vec!["Escape".to_owned()];
        std::fs::write(&path, serde_json::to_string(&controls).unwrap()).unwrap();
        let error = ControlStore::load(path).err().unwrap();
        assert!(error.contains("reserved key Escape"));
    }

    #[test]
    fn failed_atomic_save_preserves_existing_file_and_active_bindings() {
        let directory = TestDirectory::new();
        let path = directory.0.join("controls.json");
        let mut store = ControlStore::load(path.clone()).unwrap();
        store.save(Controls::default()).unwrap();
        let previous = std::fs::read(&path).unwrap();
        // A competing writer's temporary file must not be overwritten or deleted.
        std::fs::write(path.with_extension("json.tmp"), "other writer").unwrap();
        let mut candidate = store.controls.clone();
        candidate.rebind(Game::Platform, Action::Jump, "J").unwrap();
        assert!(store.save(candidate).unwrap_err().contains("Cannot save"));
        assert_eq!(store.controls, Controls::default());
        assert_eq!(std::fs::read(&path).unwrap(), previous);
        assert_eq!(std::fs::read_to_string(path.with_extension("json.tmp")).unwrap(), "other writer");
    }
}
