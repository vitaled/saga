//! Mutable runtime state of a running game.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::definition::{ActorDefinition, GameDefinition, Value, Verb};
use crate::engine::animation::AnimationState;
use crate::engine::inventory::Inventory;
use crate::geometry::Point;

/// Identifier of the player character inside actions such as `play_animation`.
pub const PLAYER_ID: &str = "player";

/// A character that can be drawn and moved around a scene.
#[derive(Debug, Clone)]
pub struct ActorState {
    pub id: String,
    pub name: String,
    pub sprite: Option<String>,
    pub position: Point,
    pub scale: f32,
    pub walk_speed: f32,
    pub walk_to: Option<Point>,
    pub visible: bool,
    /// Position the actor is currently walking to.
    pub target: Option<Point>,
    pub animation: AnimationState,
    /// -1 when facing left, 1 when facing right.
    pub facing: f32,
}

impl ActorState {
    pub fn from_definition(definition: &ActorDefinition) -> Self {
        Self {
            id: definition.id.clone(),
            name: definition.name.clone(),
            sprite: definition.sprite.clone(),
            position: definition.position,
            scale: definition.scale,
            walk_speed: definition.walk_speed,
            walk_to: definition.walk_to,
            visible: definition.visible,
            target: None,
            animation: AnimationState::new(definition.animation.clone()),
            facing: 1.0,
        }
    }

    pub fn player(definition: &GameDefinition) -> Self {
        Self {
            id: PLAYER_ID.to_string(),
            name: definition.player.name.clone(),
            sprite: definition.player.sprite.clone(),
            position: Point::new(0.0, 0.0),
            scale: definition.player.scale,
            walk_speed: definition.player.walk_speed,
            walk_to: None,
            visible: true,
            target: None,
            animation: AnimationState::new("idle"),
            facing: 1.0,
        }
    }

    pub fn is_walking(&self) -> bool {
        self.target.is_some()
    }
}

/// A line of text currently shown on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Caption {
    pub speaker: Option<String>,
    pub text: String,
    /// Seconds before the caption disappears on its own.
    pub remaining: f32,
}

/// State of a running conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct DialogueState {
    pub dialogue: String,
    pub node: String,
    /// Indices (into the node's choice list) currently offered to the player.
    pub choices: Vec<usize>,
    /// `true` while the engine waits for the player to pick a choice.
    pub awaiting_choice: bool,
}

/// Everything that changes while a game is played.
#[derive(Debug, Clone)]
pub struct GameState {
    pub current_scene: String,
    pub visited_scenes: BTreeSet<String>,
    pub variables: BTreeMap<String, Value>,
    pub inventory: Inventory,
    pub player: ActorState,
    pub actors: Vec<ActorState>,
    /// Hotspot enabled flags that differ from the definition, keyed by (scene, hotspot).
    pub hotspot_overrides: HashMap<(String, String), bool>,
    /// Actor visibility flags that differ from the definition, keyed by (scene, actor).
    pub actor_overrides: HashMap<(String, String), bool>,
    pub caption: Option<Caption>,
    pub dialogue: Option<DialogueState>,
    /// Dialogue choices already used, for choices flagged `once`.
    pub used_choices: BTreeSet<(String, String, usize)>,
    /// Inventory item selected for a "use item with ..." interaction.
    pub selected_item: Option<String>,
    /// Verb applied to the next click; `None` uses the engine's verb priority.
    pub forced_verb: Option<Verb>,
    /// Set to `false` by the `end_game` action.
    pub running: bool,
    /// Text shown after the game ended.
    pub ending_text: Option<String>,
}

impl GameState {
    pub fn new(definition: &GameDefinition) -> Self {
        Self {
            current_scene: definition.start_scene.clone(),
            visited_scenes: BTreeSet::new(),
            variables: definition.variables.clone(),
            inventory: Inventory::new(),
            player: ActorState::player(definition),
            actors: Vec::new(),
            hotspot_overrides: HashMap::new(),
            actor_overrides: HashMap::new(),
            caption: None,
            dialogue: None,
            used_choices: BTreeSet::new(),
            selected_item: None,
            forced_verb: None,
            running: true,
            ending_text: None,
        }
    }

    pub fn actor(&self, id: &str) -> Option<&ActorState> {
        if id == PLAYER_ID {
            return Some(&self.player);
        }
        self.actors.iter().find(|actor| actor.id == id)
    }

    pub fn actor_mut(&mut self, id: &str) -> Option<&mut ActorState> {
        if id == PLAYER_ID {
            return Some(&mut self.player);
        }
        self.actors.iter_mut().find(|actor| actor.id == id)
    }

    pub fn variable(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    /// Whether the hotspot is enabled, taking runtime overrides into account.
    pub fn hotspot_enabled(&self, definition: &GameDefinition, scene: &str, hotspot: &str) -> bool {
        if let Some(enabled) = self
            .hotspot_overrides
            .get(&(scene.to_string(), hotspot.to_string()))
        {
            return *enabled;
        }
        definition
            .scene(scene)
            .and_then(|scene| scene.hotspot(hotspot))
            .map(|hotspot| hotspot.enabled)
            .unwrap_or(false)
    }
}
