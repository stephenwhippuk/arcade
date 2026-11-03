#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameEvent {
    Collect,
    Shoot,
    Jump,
    Hit,
    Win,
    Lose,
    Menu,
}
