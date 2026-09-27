//! The YAML schema describing a SAGA game.
//!
//! A game is a single YAML document (optionally split across several files
//! merged by [`crate::definition::loader`]) that declares scenes, hotspots,
//! actors, items, dialogues and sprite sheets. Nothing in this module executes
//! game logic: it is a pure, serde friendly description of the game.

pub mod loader;
pub mod validate;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::geometry::{Point, Shape};

/// A value that can be stored in a game variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Bool(value) => write!(f, "{value}"),
            Value::Int(value) => write!(f, "{value}"),
            Value::Float(value) => write!(f, "{value}"),
            Value::Text(value) => write!(f, "{value}"),
        }
    }
}

/// Root of a game definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameDefinition {
    pub title: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub window: WindowConfig,
    /// Identifier of the scene the game starts in.
    pub start_scene: String,
    /// Optional entry point inside `start_scene`.
    #[serde(default)]
    pub start_entry: Option<String>,
    #[serde(default)]
    pub player: PlayerDefinition,
    #[serde(default)]
    pub variables: BTreeMap<String, Value>,
    #[serde(default)]
    pub items: BTreeMap<String, ItemDefinition>,
    #[serde(default)]
    pub dialogues: BTreeMap<String, DialogueDefinition>,
    #[serde(default)]
    pub sprites: BTreeMap<String, SpriteSheetDefinition>,
    pub scenes: BTreeMap<String, SceneDefinition>,
}

impl GameDefinition {
    pub fn scene(&self, id: &str) -> Option<&SceneDefinition> {
        self.scenes.get(id)
    }

    pub fn item(&self, id: &str) -> Option<&ItemDefinition> {
        self.items.get(id)
    }

    pub fn sprite(&self, id: &str) -> Option<&SpriteSheetDefinition> {
        self.sprites.get(id)
    }

    pub fn dialogue(&self, id: &str) -> Option<&DialogueDefinition> {
        self.dialogues.get(id)
    }
}

/// Window and virtual resolution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowConfig {
    #[serde(default = "default_window_width")]
    pub width: u32,
    #[serde(default = "default_window_height")]
    pub height: u32,
    /// Virtual resolution every coordinate in the YAML refers to. The engine
    /// letterboxes this resolution into the physical window.
    #[serde(default = "default_virtual_width")]
    pub virtual_width: u32,
    #[serde(default = "default_virtual_height")]
    pub virtual_height: u32,
    #[serde(default)]
    pub fullscreen: bool,
}

fn default_window_width() -> u32 {
    1280
}
fn default_window_height() -> u32 {
    720
}
fn default_virtual_width() -> u32 {
    640
}
fn default_virtual_height() -> u32 {
    360
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: default_window_width(),
            height: default_window_height(),
            virtual_width: default_virtual_width(),
            virtual_height: default_virtual_height(),
            fullscreen: false,
        }
    }
}

/// The player character.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerDefinition {
    #[serde(default = "default_player_name")]
    pub name: String,
    #[serde(default)]
    pub sprite: Option<String>,
    #[serde(default = "default_walk_speed")]
    pub walk_speed: f32,
    #[serde(default = "default_scale")]
    pub scale: f32,
}

fn default_player_name() -> String {
    "Player".to_string()
}
fn default_walk_speed() -> f32 {
    140.0
}
fn default_scale() -> f32 {
    1.0
}

impl Default for PlayerDefinition {
    fn default() -> Self {
        Self {
            name: default_player_name(),
            sprite: None,
            walk_speed: default_walk_speed(),
            scale: default_scale(),
        }
    }
}

/// A single room/scene of the game.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneDefinition {
    pub name: String,
    /// Path (relative to the game file) of the background image.
    #[serde(default)]
    pub background: Option<String>,
    /// Polygon the player character may walk on.
    #[serde(default)]
    pub walkable_area: Vec<Point>,
    /// Named positions used when entering the scene.
    #[serde(default)]
    pub entry_points: BTreeMap<String, Point>,
    #[serde(default)]
    pub hotspots: Vec<HotspotDefinition>,
    #[serde(default)]
    pub actors: Vec<ActorDefinition>,
    #[serde(default)]
    pub on_enter: Vec<Action>,
    #[serde(default)]
    pub on_exit: Vec<Action>,
}

impl SceneDefinition {
    pub fn hotspot(&self, id: &str) -> Option<&HotspotDefinition> {
        self.hotspots.iter().find(|hotspot| hotspot.id == id)
    }

    pub fn actor(&self, id: &str) -> Option<&ActorDefinition> {
        self.actors.iter().find(|actor| actor.id == id)
    }
}

/// A clickable region of a scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotspotDefinition {
    pub id: String,
    pub name: String,
    pub area: Shape,
    /// Where the player walks before the interaction runs.
    #[serde(default)]
    pub walk_to: Option<Point>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Actions per verb, e.g. `look`, `use`, `talk`, `take`.
    #[serde(default)]
    pub interactions: BTreeMap<Verb, Vec<Action>>,
    /// Actions run when an inventory item is used on this hotspot, keyed by item id.
    #[serde(default)]
    pub use_with: BTreeMap<String, Vec<Action>>,
}

fn default_true() -> bool {
    true
}

/// A character (NPC) standing in a scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub sprite: Option<String>,
    pub position: Point,
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default = "default_animation")]
    pub animation: String,
    #[serde(default = "default_walk_speed")]
    pub walk_speed: f32,
    /// Where the player stands when interacting with this actor.
    #[serde(default)]
    pub walk_to: Option<Point>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub interactions: BTreeMap<Verb, Vec<Action>>,
    #[serde(default)]
    pub use_with: BTreeMap<String, Vec<Action>>,
}

fn default_animation() -> String {
    "idle".to_string()
}

/// An inventory item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDefinition {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// Sprite sheet used to draw the inventory icon.
    #[serde(default)]
    pub icon: Option<String>,
    /// Frame of the icon sprite sheet.
    #[serde(default)]
    pub icon_frame: u32,
    /// Actions run when this item is combined with another inventory item.
    #[serde(default)]
    pub combine: BTreeMap<String, Vec<Action>>,
}

/// A sprite sheet: a single image split into equally sized frames.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpriteSheetDefinition {
    pub image: String,
    pub frame_width: u32,
    pub frame_height: u32,
    #[serde(default)]
    pub animations: BTreeMap<String, AnimationDefinition>,
}

/// A named animation inside a sprite sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnimationDefinition {
    pub frames: Vec<u32>,
    #[serde(default = "default_fps")]
    pub fps: f32,
    #[serde(default = "default_true")]
    pub looping: bool,
}

fn default_fps() -> f32 {
    8.0
}

/// The verbs a player can apply to a hotspot or an actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verb {
    /// Inspect something.
    Look,
    /// Operate, open, push, ...
    Use,
    /// Talk to a character.
    Talk,
    /// Pick something up.
    Take,
    /// Walk to the hotspot (used for doors and scene exits).
    Walk,
}

impl Verb {
    /// Verbs in the order the engine tries them for a plain left click.
    pub const PRIORITY: [Verb; 5] = [Verb::Walk, Verb::Talk, Verb::Take, Verb::Use, Verb::Look];

    pub fn label(self) -> &'static str {
        match self {
            Verb::Look => "Look at",
            Verb::Use => "Use",
            Verb::Talk => "Talk to",
            Verb::Take => "Take",
            Verb::Walk => "Walk to",
        }
    }
}

/// A conversation made of nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueDefinition {
    /// Node the conversation starts with.
    pub start: String,
    pub nodes: BTreeMap<String, DialogueNode>,
}

/// A single beat of a conversation: an optional line plus optional choices.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueNode {
    /// Name shown in front of `text`; defaults to the conversation partner.
    #[serde(default)]
    pub speaker: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    /// Actions run when the node is entered.
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub choices: Vec<DialogueChoice>,
    /// Node to continue with when there are no choices.
    #[serde(default)]
    pub goto: Option<String>,
    /// Ends the conversation after the node has been shown.
    #[serde(default)]
    pub end: bool,
}

/// A selectable answer inside a dialogue node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DialogueChoice {
    pub text: String,
    /// The choice is only offered when the condition holds.
    #[serde(default)]
    pub condition: Option<Condition>,
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub goto: Option<String>,
    /// Ends the conversation once picked.
    #[serde(default)]
    pub end: bool,
    /// The choice disappears after it has been picked once.
    #[serde(default)]
    pub once: bool,
}

/// Something the game does in reaction to an interaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    /// Shows a line of text. Without `actor` the line is narration.
    Say {
        #[serde(default)]
        actor: Option<String>,
        text: String,
        #[serde(default)]
        duration: Option<f32>,
    },
    /// Changes the current scene.
    GoToScene {
        scene: String,
        #[serde(default)]
        entry: Option<String>,
    },
    /// Starts a conversation.
    StartDialogue { dialogue: String },
    /// Ends the running conversation.
    EndDialogue,
    /// Adds an item to the inventory.
    GiveItem { item: String },
    /// Removes an item from the inventory.
    RemoveItem { item: String },
    /// Sets a game variable.
    SetVariable { name: String, value: Value },
    /// Enables or disables a hotspot (in the current scene unless `scene` is given).
    SetHotspot {
        hotspot: String,
        #[serde(default)]
        scene: Option<String>,
        enabled: bool,
    },
    /// Shows or hides an actor of the current scene.
    SetActorVisible { actor: String, visible: bool },
    /// Plays an animation on the player (`player`) or on an actor of the scene.
    PlayAnimation {
        actor: String,
        animation: String,
        #[serde(default)]
        looping: Option<bool>,
    },
    /// Walks the player (or an actor) to a position; the script waits for arrival.
    MoveActor {
        #[serde(default = "player_actor_id")]
        actor: String,
        to: Point,
    },
    /// Pauses the script.
    Wait { seconds: f32 },
    /// Runs actions conditionally.
    If {
        condition: Condition,
        #[serde(default)]
        then: Vec<Action>,
        #[serde(default, rename = "else")]
        otherwise: Vec<Action>,
    },
    /// Ends the game.
    EndGame {
        #[serde(default)]
        text: Option<String>,
    },
}

fn player_actor_id() -> String {
    crate::engine::PLAYER_ID.to_string()
}

/// A boolean test over the game state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "check", rename_all = "snake_case", deny_unknown_fields)]
pub enum Condition {
    /// True when the inventory contains the item.
    HasItem { item: String },
    /// True when the variable equals the value (missing variables never match).
    VariableEquals { name: String, value: Value },
    /// True when the scene has been visited at least once.
    VisitedScene { scene: String },
    /// True when the hotspot is currently enabled.
    HotspotEnabled {
        hotspot: String,
        #[serde(default)]
        scene: Option<String>,
    },
    /// Negates a condition.
    Not { condition: Box<Condition> },
    /// True when every nested condition holds.
    All { conditions: Vec<Condition> },
    /// True when at least one nested condition holds.
    Any { conditions: Vec<Condition> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_use_a_readable_tagged_representation() {
        let yaml = r#"
- action: say
  actor: player
  text: "Hello!"
- action: give_item
  item: key
- action: if
  condition:
    check: has_item
    item: key
  then:
    - action: go_to_scene
      scene: hall
  else:
    - action: say
      text: "Locked."
"#;
        let actions: Vec<Action> = serde_yaml::from_str(yaml).expect("actions parse");
        assert_eq!(actions.len(), 3);
        assert!(matches!(actions[0], Action::Say { .. }));
        assert!(matches!(actions[1], Action::GiveItem { .. }));
        match &actions[2] {
            Action::If {
                then, otherwise, ..
            } => {
                assert_eq!(then.len(), 1);
                assert_eq!(otherwise.len(), 1);
            }
            other => panic!("unexpected action {other:?}"),
        }
    }

    #[test]
    fn hotspot_areas_accept_rectangles_and_polygons() {
        let yaml = r#"
id: door
name: Door
area: { x: 10, y: 20, width: 30, height: 40 }
"#;
        let hotspot: HotspotDefinition = serde_yaml::from_str(yaml).expect("rect hotspot");
        assert!(matches!(hotspot.area, Shape::Rect(_)));
        assert!(hotspot.enabled);

        let yaml = r#"
id: puddle
name: Puddle
area:
  polygon:
    - { x: 0, y: 0 }
    - { x: 10, y: 0 }
    - { x: 10, y: 10 }
"#;
        let hotspot: HotspotDefinition = serde_yaml::from_str(yaml).expect("polygon hotspot");
        assert!(matches!(hotspot.area, Shape::Polygon { .. }));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let yaml = r#"
id: door
name: Door
area: { x: 0, y: 0, width: 1, height: 1 }
typo_field: true
"#;
        assert!(serde_yaml::from_str::<HotspotDefinition>(yaml).is_err());
    }
}
