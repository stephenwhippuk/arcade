use crate::assets::{Sample, Sprite};
use crate::geometry::Rect;
use macroquad::prelude as mq;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

pub const WHITE: Color = Color::new(1.0, 1.0, 1.0, 1.0);
pub const LIGHTGRAY: Color = Color::new(0.78, 0.78, 0.78, 1.0);
pub const DARKGRAY: Color = Color::new(0.31, 0.31, 0.31, 1.0);
pub const GREEN: Color = Color::new(0.0, 0.89, 0.19, 1.0);
pub const YELLOW: Color = Color::new(0.99, 0.98, 0.0, 1.0);
pub const GOLD: Color = Color::new(1.0, 0.8, 0.0, 1.0);
pub const RED: Color = Color::new(0.9, 0.16, 0.22, 1.0);
pub const SKYBLUE: Color = Color::new(0.4, 0.75, 1.0, 1.0);
pub const DARKBLUE: Color = Color::new(0.0, 0.32, 0.67, 1.0);

pub trait Renderer {
    fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, color: Color);
    fn draw_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32, color: Color);
    fn draw_rectangle_lines(&mut self, bounds: Rect, thickness: f32, color: Color);
    fn draw_circle(&mut self, x: f32, y: f32, radius: f32, color: Color);
    fn draw_sprite(&mut self, sprite: Sprite, sample: Sample<'_>, bounds: Rect);
}

pub struct MacroquadRenderer;

impl MacroquadRenderer {
    pub fn new() -> Self {
        Self
    }
}

fn color(color: Color) -> mq::Color {
    mq::Color::new(color.r, color.g, color.b, color.a)
}

impl Renderer for MacroquadRenderer {
    fn draw_text(&mut self, text: &str, x: f32, y: f32, size: f32, tint: Color) {
        mq::draw_text(text, x, y, size, color(tint));
    }

    fn draw_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32, tint: Color) {
        mq::draw_rectangle(x, y, width, height, color(tint));
    }

    fn draw_rectangle_lines(&mut self, bounds: Rect, thickness: f32, tint: Color) {
        mq::draw_rectangle_lines(bounds.x, bounds.y, bounds.width, bounds.height, thickness, color(tint));
    }

    fn draw_circle(&mut self, x: f32, y: f32, radius: f32, tint: Color) {
        mq::draw_circle(x, y, radius, color(tint));
    }

    fn draw_sprite(&mut self, _: Sprite, sample: Sample<'_>, bounds: Rect) {
        mq::draw_texture_ex(sample.frame.texture(), bounds.x, bounds.y, color(sample.tint),
            mq::DrawTextureParams {
                dest_size: Some(mq::vec2(bounds.width, bounds.height)),
                rotation: sample.rotation,
                flip_x: sample.flip_x,
                ..Default::default()
            });
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct RecordingRenderer {
    pub sprites: Vec<(Sprite, Rect, Color)>,
    pub texts: Vec<String>,
    pub frames: Vec<(String, f32, bool)>,
}

#[cfg(test)]
impl Renderer for RecordingRenderer {
    fn draw_text(&mut self, text: &str, _: f32, _: f32, _: f32, _: Color) {
        self.texts.push(text.to_owned());
    }
    fn draw_rectangle(&mut self, _: f32, _: f32, _: f32, _: f32, _: Color) {}
    fn draw_rectangle_lines(&mut self, _: Rect, _: f32, _: Color) {}
    fn draw_circle(&mut self, _: f32, _: f32, _: f32, _: Color) {}
    fn draw_sprite(&mut self, sprite: Sprite, sample: Sample<'_>, bounds: Rect) {
        self.sprites.push((sprite, bounds, sample.tint));
        self.frames.push((sample.frame.name.clone(), sample.rotation, sample.flip_x));
    }
}
