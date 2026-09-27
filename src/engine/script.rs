//! The scripting layer: a queue of steps plus condition evaluation.

use std::collections::VecDeque;

use crate::definition::{Action, Condition, GameDefinition};
use crate::engine::state::GameState;

/// A single unit of work the engine performs, one at a time.
#[derive(Debug, Clone)]
pub(crate) enum ScriptStep {
    /// An action authored in the YAML.
    Action(Action),
    /// Switches to another scene (queued after the `on_exit` actions).
    EnterScene {
        scene: String,
        entry: Option<String>,
    },
    /// Enters a dialogue node: runs its actions and shows its line.
    EnterDialogueNode { node: String },
    /// Decides how a dialogue node continues once its line has been shown.
    ResolveDialogueNode,
    /// Continues the conversation after a choice has been made.
    ResolveChoice { goto: Option<String>, end: bool },
    /// Records a scene as visited, once its `on_enter` actions have run.
    MarkVisited { scene: String },
}

/// What the script is currently waiting for.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) enum ScriptStatus {
    #[default]
    Idle,
    /// Waiting for a timer (`wait` action).
    Waiting(f32),
    /// Waiting for the current caption to be dismissed or to time out.
    Caption,
    /// Waiting for an actor to reach its walk target.
    Actor(String),
}

/// An ordered queue of script steps.
#[derive(Debug, Default)]
pub(crate) struct ScriptRunner {
    queue: VecDeque<ScriptStep>,
    pub(crate) status: ScriptStatus,
}

impl ScriptRunner {
    pub(crate) fn is_busy(&self) -> bool {
        self.status != ScriptStatus::Idle || !self.queue.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.queue.clear();
        self.status = ScriptStatus::Idle;
    }

    pub(crate) fn push_actions(&mut self, actions: &[Action]) {
        for action in actions {
            self.queue.push_back(ScriptStep::Action(action.clone()));
        }
    }

    pub(crate) fn push(&mut self, step: ScriptStep) {
        self.queue.push_back(step);
    }

    /// Inserts steps at the front, keeping their relative order.
    pub(crate) fn push_front_all(&mut self, steps: Vec<ScriptStep>) {
        for step in steps.into_iter().rev() {
            self.queue.push_front(step);
        }
    }

    pub(crate) fn next_step(&mut self) -> Option<ScriptStep> {
        self.queue.pop_front()
    }
}

/// Evaluates a condition against the current state.
pub fn evaluate(condition: &Condition, state: &GameState, definition: &GameDefinition) -> bool {
    match condition {
        Condition::HasItem { item } => state.inventory.contains(item),
        Condition::VariableEquals { name, value } => {
            state.variable(name).map(|current| current == value) == Some(true)
        }
        Condition::VisitedScene { scene } => state.visited_scenes.contains(scene),
        Condition::HotspotEnabled { hotspot, scene } => {
            let scene = scene.as_deref().unwrap_or(&state.current_scene);
            state.hotspot_enabled(definition, scene, hotspot)
        }
        Condition::Not { condition } => !evaluate(condition, state, definition),
        Condition::All { conditions } => conditions
            .iter()
            .all(|condition| evaluate(condition, state, definition)),
        Condition::Any { conditions } => conditions
            .iter()
            .any(|condition| evaluate(condition, state, definition)),
    }
}

/// How long a caption stays on screen when the author gave no duration.
pub fn caption_duration(text: &str) -> f32 {
    (1.2 + text.chars().count() as f32 * 0.05).min(8.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::loader::parse_game;
    use crate::definition::Value;

    const GAME: &str = r#"
title: Conditions
start_scene: room
items:
  key:
    name: Key
scenes:
  room:
    name: Room
"#;

    #[test]
    fn conditions_combine_with_logic_operators() {
        let definition = parse_game(GAME).unwrap();
        let mut state = GameState::new(&definition);
        state.inventory.add("key");
        state.variables.insert("doors".to_string(), Value::Int(2));

        let has_key = Condition::HasItem {
            item: "key".to_string(),
        };
        let two_doors = Condition::VariableEquals {
            name: "doors".to_string(),
            value: Value::Int(2),
        };
        let visited = Condition::VisitedScene {
            scene: "room".to_string(),
        };

        assert!(evaluate(&has_key, &state, &definition));
        assert!(evaluate(&two_doors, &state, &definition));
        assert!(!evaluate(&visited, &state, &definition));
        assert!(evaluate(
            &Condition::Not {
                condition: Box::new(visited.clone())
            },
            &state,
            &definition
        ));
        assert!(evaluate(
            &Condition::Any {
                conditions: vec![visited.clone(), has_key.clone()]
            },
            &state,
            &definition
        ));
        assert!(!evaluate(
            &Condition::All {
                conditions: vec![visited, has_key]
            },
            &state,
            &definition
        ));
    }

    #[test]
    fn captions_stay_longer_for_longer_lines() {
        assert!(caption_duration("Hi") < caption_duration("A much longer sentence indeed"));
        assert!(caption_duration(&"x".repeat(1000)) <= 8.0);
    }
}
