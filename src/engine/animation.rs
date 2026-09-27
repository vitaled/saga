//! Frame based sprite animation.

use crate::definition::AnimationDefinition;

/// Playback state of a single animation.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationState {
    /// Name of the animation inside the sprite sheet.
    pub name: String,
    /// Index inside [`AnimationDefinition::frames`].
    pub frame: usize,
    /// Seconds spent on the current frame.
    pub elapsed: f32,
    /// Overrides the looping flag of the definition when set.
    pub looping: Option<bool>,
    /// `true` once a non looping animation has shown its last frame.
    pub finished: bool,
}

impl AnimationState {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            frame: 0,
            elapsed: 0.0,
            looping: None,
            finished: false,
        }
    }

    /// Switches to another animation. Replaying the running animation is a no-op.
    pub fn play(&mut self, name: impl Into<String>, looping: Option<bool>) {
        let name = name.into();
        if self.name == name && self.looping == looping && !self.finished {
            return;
        }
        self.name = name;
        self.frame = 0;
        self.elapsed = 0.0;
        self.looping = looping;
        self.finished = false;
    }

    /// Advances the animation by `dt` seconds.
    pub fn update(&mut self, dt: f32, definition: Option<&AnimationDefinition>) {
        let Some(definition) = definition else {
            return;
        };
        if definition.frames.is_empty() || definition.fps <= 0.0 || self.finished {
            return;
        }
        let frame_duration = 1.0 / definition.fps;
        self.elapsed += dt;
        while self.elapsed >= frame_duration {
            self.elapsed -= frame_duration;
            if self.frame + 1 < definition.frames.len() {
                self.frame += 1;
            } else if self.looping.unwrap_or(definition.looping) {
                self.frame = 0;
            } else {
                self.finished = true;
                break;
            }
        }
    }

    /// Frame of the sprite sheet that must be drawn.
    pub fn sheet_frame(&self, definition: Option<&AnimationDefinition>) -> u32 {
        definition
            .and_then(|definition| definition.frames.get(self.frame).copied())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn animation(frames: Vec<u32>, looping: bool) -> AnimationDefinition {
        AnimationDefinition {
            frames,
            fps: 10.0,
            looping,
        }
    }

    #[test]
    fn looping_animations_wrap_around() {
        let definition = animation(vec![0, 1, 2], true);
        let mut state = AnimationState::new("walk");
        state.update(0.1, Some(&definition));
        assert_eq!(state.sheet_frame(Some(&definition)), 1);
        state.update(0.2, Some(&definition));
        assert_eq!(state.sheet_frame(Some(&definition)), 0);
        assert!(!state.finished);
    }

    #[test]
    fn non_looping_animations_stop_on_the_last_frame() {
        let definition = animation(vec![4, 5], false);
        let mut state = AnimationState::new("wave");
        state.update(1.0, Some(&definition));
        assert!(state.finished);
        assert_eq!(state.sheet_frame(Some(&definition)), 5);
    }

    #[test]
    fn playing_the_same_animation_does_not_restart_it() {
        let definition = animation(vec![0, 1, 2], true);
        let mut state = AnimationState::new("walk");
        state.update(0.1, Some(&definition));
        state.play("walk", None);
        assert_eq!(state.frame, 1);
        state.play("idle", None);
        assert_eq!(state.frame, 0);
    }
}
