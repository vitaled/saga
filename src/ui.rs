//! The presentation layer: turns engine state into draw calls and maps clicks
//! back to engine input.
//!
//! Everything here works in virtual pixels, so a game authored for a 640x360
//! virtual resolution looks identical in any window size.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::definition::loader::LoadedGame;
use crate::definition::Verb;
use crate::engine::{Engine, MouseButton};
use crate::geometry::{Point, Rect};
use crate::render::{Color, Renderer, TextureId};

const PANEL_HEIGHT: f32 = 78.0;
const VERB_WIDTH: f32 = 74.0;
const VERB_HEIGHT: f32 = 13.0;
const SLOT_SIZE: f32 = 30.0;
const SLOT_GAP: f32 = 4.0;
const LINE_HEIGHT: f32 = 12.0;
const TEXT_SCALE: f32 = 1.0;

const PANEL_COLOR: Color = Color::rgba(0.05, 0.05, 0.09, 0.92);
const BUTTON_COLOR: Color = Color::rgba(0.16, 0.16, 0.24, 1.0);
const BUTTON_ACTIVE: Color = Color::rgba(0.34, 0.28, 0.12, 1.0);
const SLOT_COLOR: Color = Color::rgba(0.12, 0.12, 0.18, 1.0);
const SLOT_SELECTED: Color = Color::rgba(0.45, 0.36, 0.14, 1.0);
const TEXT_COLOR: Color = Color::rgba(0.92, 0.9, 0.82, 1.0);
const HIGHLIGHT_COLOR: Color = Color::rgba(1.0, 0.85, 0.4, 1.0);
const MISSING_ART: Color = Color::rgba(0.22, 0.24, 0.3, 1.0);
const ACTOR_PLACEHOLDER: Color = Color::rgba(0.75, 0.42, 0.3, 1.0);
const PLAYER_PLACEHOLDER: Color = Color::rgba(0.4, 0.6, 0.85, 1.0);

/// Draws a game and translates window clicks into engine input.
pub struct GameUi {
    game: LoadedGame,
    textures: HashMap<String, Option<TextureId>>,
    hover: Point,
}

impl GameUi {
    pub fn new(game: LoadedGame) -> Self {
        Self {
            game,
            textures: HashMap::new(),
            hover: Point::new(-1.0, -1.0),
        }
    }

    /// Remembers the cursor position, used for the hover label.
    pub fn set_cursor(&mut self, position: Point) {
        self.hover = position;
    }

    /// Routes a click either to the UI panel or to the scene.
    pub fn click(&mut self, engine: &mut Engine, point: Point, button: MouseButton) {
        let (width, height) = self.virtual_size();
        let panel_top = height - PANEL_HEIGHT;
        if point.y < panel_top {
            engine.click(point, button);
            return;
        }

        if !engine.choices().is_empty() {
            let index = ((point.y - panel_top - 6.0) / LINE_HEIGHT).floor();
            if index >= 0.0 && (index as usize) < engine.choices().len() {
                engine.choose_dialogue(index as usize);
            }
            return;
        }

        for (index, verb) in Verb::PRIORITY.iter().enumerate() {
            if self.verb_rect(index, panel_top).contains(point) {
                let active = engine.state().forced_verb == Some(*verb);
                engine.set_verb(if active { None } else { Some(*verb) });
                return;
            }
        }

        let slots = self.slot_rects(panel_top, width);
        for (index, rect) in slots.iter().enumerate() {
            if rect.contains(point) {
                match button {
                    MouseButton::Left => engine.click_inventory(index),
                    MouseButton::Right => engine.inspect_inventory(index),
                }
                return;
            }
        }
    }

    /// Draws one frame.
    pub fn draw(&mut self, engine: &Engine, renderer: &mut Renderer) {
        renderer.begin_frame();
        self.draw_scene(engine, renderer);
        self.draw_panel(engine, renderer);
        self.draw_caption(engine, renderer);
    }

    fn virtual_size(&self) -> (f32, f32) {
        let window = &self.game.definition.window;
        (window.virtual_width as f32, window.virtual_height as f32)
    }

    fn draw_scene(&mut self, engine: &Engine, renderer: &mut Renderer) {
        let (width, height) = self.virtual_size();
        let scene = engine.current_scene();
        let background = scene
            .background
            .clone()
            .and_then(|path| self.texture(renderer, &path));
        match background {
            Some(texture) => renderer.draw_sprite(
                texture,
                Rect::new(0.0, 0.0, width, height),
                None,
                Color::WHITE,
            ),
            None => renderer.draw_rect(Rect::new(0.0, 0.0, width, height), MISSING_ART),
        }

        // Actors are sorted by their feet so characters overlap correctly.
        let mut actors: Vec<&crate::engine::ActorState> = engine
            .state()
            .actors
            .iter()
            .filter(|actor| actor.visible)
            .collect();
        actors.push(&engine.state().player);
        actors.sort_by(|a, b| {
            a.position
                .y
                .partial_cmp(&b.position.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for actor in actors {
            let is_player = actor.id == crate::engine::PLAYER_ID;
            let bounds = engine.actor_bounds(actor);
            let sheet = actor
                .sprite
                .as_ref()
                .and_then(|sprite| engine.definition().sprite(sprite));
            let texture = sheet.and_then(|sheet| {
                let image = sheet.image.clone();
                self.texture(renderer, &image)
            });
            match (sheet, texture) {
                (Some(sheet), Some(texture)) => {
                    let animation = sheet.animations.get(&actor.animation.name);
                    let frame = actor.animation.sheet_frame(animation);
                    let (image_width, _) = renderer.texture_size(texture);
                    let columns = (image_width / sheet.frame_width.max(1)).max(1);
                    let mut source = Rect::new(
                        (frame % columns * sheet.frame_width) as f32,
                        (frame / columns * sheet.frame_height) as f32,
                        sheet.frame_width as f32,
                        sheet.frame_height as f32,
                    );
                    if actor.facing < 0.0 {
                        source.x += source.width;
                        source.width = -source.width;
                    }
                    renderer.draw_sprite(texture, bounds, Some(source), Color::WHITE);
                }
                _ => renderer.draw_rect(
                    bounds,
                    if is_player {
                        PLAYER_PLACEHOLDER
                    } else {
                        ACTOR_PLACEHOLDER
                    },
                ),
            }
        }
    }

    fn draw_panel(&mut self, engine: &Engine, renderer: &mut Renderer) {
        let (width, height) = self.virtual_size();
        let panel_top = height - PANEL_HEIGHT;
        renderer.draw_rect(
            Rect::new(0.0, panel_top, width, PANEL_HEIGHT),
            PANEL_COLOR,
        );

        let choices = engine.choices();
        if !choices.is_empty() {
            for (index, text) in &choices {
                let y = panel_top + 6.0 + *index as f32 * LINE_HEIGHT;
                renderer.draw_text(
                    &format!("{}. {text}", index + 1),
                    Point::new(10.0, y),
                    TEXT_SCALE,
                    HIGHLIGHT_COLOR,
                );
            }
            return;
        }

        for (index, verb) in Verb::PRIORITY.iter().enumerate() {
            let rect = self.verb_rect(index, panel_top);
            let active = engine.state().forced_verb == Some(*verb);
            renderer.draw_rect(
                rect,
                if active { BUTTON_ACTIVE } else { BUTTON_COLOR },
            );
            renderer.draw_text(
                verb.label(),
                Point::new(rect.x + 4.0, rect.y + 3.0),
                TEXT_SCALE,
                if active { HIGHLIGHT_COLOR } else { TEXT_COLOR },
            );
        }

        let slots = self.slot_rects(panel_top, width);
        let inventory = engine.state().inventory.items().to_vec();
        for (index, rect) in slots.iter().enumerate() {
            let Some(item_id) = inventory.get(index) else {
                renderer.draw_rect(*rect, SLOT_COLOR);
                continue;
            };
            let selected = engine.state().selected_item.as_ref() == Some(item_id);
            renderer.draw_rect(*rect, if selected { SLOT_SELECTED } else { SLOT_COLOR });

            let item = engine.definition().item(item_id);
            let icon = item.and_then(|item| {
                let sheet = engine.definition().sprite(item.icon.as_deref()?)?;
                let image = sheet.image.clone();
                let texture = self.texture(renderer, &image)?;
                let (image_width, _) = renderer.texture_size(texture);
                let columns = (image_width / sheet.frame_width.max(1)).max(1);
                Some((
                    texture,
                    Rect::new(
                        (item.icon_frame % columns * sheet.frame_width) as f32,
                        (item.icon_frame / columns * sheet.frame_height) as f32,
                        sheet.frame_width as f32,
                        sheet.frame_height as f32,
                    ),
                ))
            });
            match icon {
                Some((texture, source)) => renderer.draw_sprite(
                    texture,
                    Rect::new(rect.x + 3.0, rect.y + 3.0, rect.width - 6.0, rect.height - 6.0),
                    Some(source),
                    Color::WHITE,
                ),
                None => {
                    let label = item
                        .map(|item| item.name.clone())
                        .unwrap_or_else(|| item_id.clone());
                    let label: String = label.chars().take(4).collect();
                    renderer.draw_text(
                        &label,
                        Point::new(rect.x + 3.0, rect.y + rect.height / 2.0 - 3.0),
                        TEXT_SCALE,
                        TEXT_COLOR,
                    );
                }
            }
        }

        if let Some(label) = engine.hover_label(self.hover) {
            let text_width = Renderer::text_width(&label, TEXT_SCALE);
            renderer.draw_text(
                &label,
                Point::new((width - text_width) / 2.0, panel_top - 12.0),
                TEXT_SCALE,
                HIGHLIGHT_COLOR,
            );
        }
    }

    fn draw_caption(&mut self, engine: &Engine, renderer: &mut Renderer) {
        let (width, height) = self.virtual_size();
        let Some(caption) = engine.state().caption.as_ref() else {
            return;
        };
        let text = match &caption.speaker {
            Some(speaker) => format!("{speaker}: {}", caption.text),
            None => caption.text.clone(),
        };
        let max_characters = ((width - 40.0) / 6.0) as usize;
        let lines = wrap(&text, max_characters.max(8));
        let block_height = lines.len() as f32 * LINE_HEIGHT;
        let top = height - PANEL_HEIGHT - 24.0 - block_height;
        for (index, line) in lines.iter().enumerate() {
            let line_width = Renderer::text_width(line, TEXT_SCALE);
            renderer.draw_text(
                line,
                Point::new(
                    (width - line_width) / 2.0,
                    top + index as f32 * LINE_HEIGHT,
                ),
                TEXT_SCALE,
                TEXT_COLOR,
            );
        }
    }

    fn verb_rect(&self, index: usize, panel_top: f32) -> Rect {
        Rect::new(
            6.0,
            panel_top + 6.0 + index as f32 * VERB_HEIGHT,
            VERB_WIDTH,
            VERB_HEIGHT - 2.0,
        )
    }

    fn slot_rects(&self, panel_top: f32, width: f32) -> Vec<Rect> {
        let left = VERB_WIDTH + 16.0;
        let columns = (((width - left - 8.0) / (SLOT_SIZE + SLOT_GAP)).floor() as usize).max(1);
        let mut rects = Vec::new();
        for row in 0..2 {
            for column in 0..columns {
                rects.push(Rect::new(
                    left + column as f32 * (SLOT_SIZE + SLOT_GAP),
                    panel_top + 8.0 + row as f32 * (SLOT_SIZE + SLOT_GAP),
                    SLOT_SIZE,
                    SLOT_SIZE,
                ));
            }
        }
        rects
    }

    /// Loads (and caches) a texture declared in the game definition.
    fn texture(&mut self, renderer: &mut Renderer, relative: &str) -> Option<TextureId> {
        if let Some(texture) = self.textures.get(relative) {
            return *texture;
        }
        let texture = self
            .game
            .asset_path(relative)
            .and_then(|path: PathBuf| renderer.load_texture(&path))
            .map_err(|error| log::warn!("{error}"))
            .ok();
        self.textures.insert(relative.to_string(), texture);
        texture
    }
}

/// Breaks `text` into lines of at most `max_characters` characters.
pub fn wrap(text: &str, max_characters: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            line = word.to_string();
        } else if line.chars().count() + 1 + word.chars().count() <= max_characters {
            line.push(' ');
            line.push_str(word);
        } else {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn wrapping_keeps_words_intact() {
        let lines = wrap("the quick brown fox jumps", 10);
        assert_eq!(lines, vec!["the quick", "brown fox", "jumps"]);
    }

    #[test]
    fn wrapping_an_empty_string_yields_one_empty_line() {
        assert_eq!(wrap("", 10), vec![String::new()]);
    }
}
