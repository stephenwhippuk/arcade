use macroquad::prelude::{FilterMode, Texture2D};
use std::fmt;

const SIDE: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sprite {
    Orb,
    Pursuer,
    Ship,
    Alien,
    Miner,
    Key,
    Exit,
}

const SOURCES: [(Sprite, &str, &str); 7] = [
    (Sprite::Orb, "orb.sprite", include_str!("../assets/sprites/orb.sprite")),
    (Sprite::Pursuer, "pursuer.sprite", include_str!("../assets/sprites/pursuer.sprite")),
    (Sprite::Ship, "ship.sprite", include_str!("../assets/sprites/ship.sprite")),
    (Sprite::Alien, "alien.sprite", include_str!("../assets/sprites/alien.sprite")),
    (Sprite::Miner, "miner.sprite", include_str!("../assets/sprites/miner.sprite")),
    (Sprite::Key, "key.sprite", include_str!("../assets/sprites/key.sprite")),
    (Sprite::Exit, "exit.sprite", include_str!("../assets/sprites/exit.sprite")),
];

#[derive(Debug)]
pub struct AssetError {
    name: &'static str,
    reason: &'static str,
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid sprite {}: {}", self.name, self.reason)
    }
}

impl std::error::Error for AssetError {}

fn decode(name: &'static str, source: &str) -> Result<Vec<u8>, AssetError> {
    let rows: Vec<_> = source.lines().collect();
    if rows.len() != SIDE || rows.iter().any(|row| row.len() != SIDE) {
        return Err(AssetError { name, reason: "expected exactly eight rows of eight pixels" });
    }
    let mut pixels = Vec::with_capacity(SIDE * SIDE * 4);
    for row in rows {
        for pixel in row.bytes() {
            match pixel {
                b'#' => pixels.extend_from_slice(&[255, 255, 255, 255]),
                b'.' => pixels.extend_from_slice(&[0, 0, 0, 0]),
                _ => return Err(AssetError { name, reason: "pixels must be '#' or '.'" }),
            }
        }
    }
    Ok(pixels)
}

pub struct Assets {
    textures: Vec<(Sprite, Texture2D)>,
}

impl Assets {
    pub fn load() -> Result<Self, AssetError> {
        // Validate every source before creating any GPU resources.
        let decoded = SOURCES.iter().map(|&(sprite, name, source)| {
            decode(name, source).map(|pixels| (sprite, pixels))
        }).collect::<Result<Vec<_>, _>>()?;
        let textures = decoded.into_iter().map(|(sprite, pixels)| {
            let texture = Texture2D::from_rgba8(SIDE as u16, SIDE as u16, &pixels);
            texture.set_filter(FilterMode::Nearest);
            (sprite, texture)
        }).collect();
        Ok(Self { textures })
    }

    pub fn texture(&self, sprite: Sprite) -> &Texture2D {
        &self.textures.iter().find(|(kind, _)| *kind == sprite)
            .expect("Every sprite identifier must have an embedded asset").1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_shipped_sprites_decode_with_visible_and_transparent_pixels() {
        for (_, name, source) in SOURCES {
            let pixels = decode(name, source).unwrap();
            assert_eq!(pixels.len(), SIDE * SIDE * 4);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] == 255), "{name}");
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] == 0), "{name}");
        }
    }

    #[test]
    fn invalid_dimensions_and_pixels_report_the_asset_name() {
        for source in ["#", "xxxxxxxx\nxxxxxxxx\nxxxxxxxx\nxxxxxxxx\nxxxxxxxx\nxxxxxxxx\nxxxxxxxx\nxxxxxxxx"] {
            let error = decode("broken.sprite", source).unwrap_err();
            assert!(error.to_string().contains("broken.sprite"));
        }
    }

    #[test]
    fn sprite_identifiers_are_unique() {
        for (index, (sprite, _, _)) in SOURCES.iter().enumerate() {
            assert!(!SOURCES[..index].iter().any(|(other, _, _)| other == sprite));
        }
    }
}
