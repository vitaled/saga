//! GPU rendering with wgpu.
//!
//! The renderer is a single instanced sprite pipeline: every visual element
//! (background, sprite frame, UI panel, glyph) is a textured, tinted quad in
//! virtual pixel coordinates. Quads are batched per texture and uploaded once
//! per frame, which keeps the draw call count at a handful even for busy
//! scenes.

pub mod font;
pub mod renderer;

pub use renderer::{Color, Renderer, TextureId};
