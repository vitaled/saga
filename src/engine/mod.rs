//! The SAGA runtime: turns a [`GameDefinition`] into a playable game.
//!
//! The engine is completely decoupled from rendering and windowing: it is fed
//! input events and a delta time and exposes the state a renderer needs. That
//! makes the whole game logic testable without a GPU.

pub mod animation;
pub mod inventory;
pub mod script;
pub mod state;

use std::sync::Arc;

use crate::definition::{
    Action, ActorDefinition, GameDefinition, HotspotDefinition, SceneDefinition, Value, Verb,
};
use crate::error::Result;
use crate::geometry::{clamp_to_polygon, Point, Rect};

use script::{caption_duration, ScriptRunner, ScriptStatus, ScriptStep};
pub use state::{ActorState, Caption, DialogueState, GameState, PLAYER_ID};

/// Mouse buttons understood by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Runs the default verb of the clicked object.
    Left,
    /// Looks at the clicked object.
    Right,
}

/// Something the player can click on inside a scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Hotspot(String),
    Actor(String),
}

/// Size used for actors without a sprite sheet, so they stay clickable.
const DEFAULT_ACTOR_SIZE: (f32, f32) = (24.0, 48.0);

/// Maximum number of script steps executed in a single frame; protects against
/// game definitions that loop forever (e.g. two scenes entering each other).
const MAX_STEPS_PER_FRAME: usize = 256;

/// A queued interaction that runs once the player reached the object.
#[derive(Debug, Clone)]
struct PendingInteraction {
    actions: Vec<Action>,
}

/// The playable game.
pub struct Engine {
    definition: Arc<GameDefinition>,
    state: GameState,
    script: ScriptRunner,
    pending: Option<PendingInteraction>,
}

impl Engine {
    /// Creates an engine for `definition` and enters the start scene.
    pub fn new(definition: GameDefinition) -> Result<Self> {
        crate::definition::validate::validate(&definition)?;
        let definition = Arc::new(definition);
        let state = GameState::new(&definition);
        let mut engine = Self {
            definition: definition.clone(),
            state,
            script: ScriptRunner::default(),
            pending: None,
        };
        let entry = definition.start_entry.clone();
        engine.enter_scene(definition.start_scene.clone(), entry);
        Ok(engine)
    }

    pub fn definition(&self) -> &GameDefinition {
        &self.definition
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    /// The scene the player is currently in.
    pub fn current_scene(&self) -> &SceneDefinition {
        self.definition
            .scene(&self.state.current_scene)
            .expect("the current scene always exists")
    }

    /// `true` while a script, a caption or a conversation is running.
    pub fn is_busy(&self) -> bool {
        self.script.is_busy() || self.pending.is_some() || self.state.caption.is_some()
    }

    /// `true` once the game ended.
    pub fn is_finished(&self) -> bool {
        !self.state.running
    }

    // ----------------------------------------------------------------- update

    /// Advances the game by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        if !self.state.running {
            return;
        }
        self.update_movement(dt);
        self.update_caption(dt);
        self.update_animations(dt);

        if let Some(pending) = self.pending.take() {
            if self.state.player.is_walking() {
                self.pending = Some(pending);
            } else {
                self.script.push_actions(&pending.actions);
            }
        }

        if let ScriptStatus::Waiting(remaining) = self.script.status {
            let remaining = remaining - dt;
            self.script.status = if remaining <= 0.0 {
                ScriptStatus::Idle
            } else {
                ScriptStatus::Waiting(remaining)
            };
        }
        if let ScriptStatus::Actor(id) = self.script.status.clone() {
            let still_walking = self
                .state
                .actor(&id)
                .map(ActorState::is_walking)
                .unwrap_or(false);
            if !still_walking {
                self.script.status = ScriptStatus::Idle;
            }
        }

        self.run_script();
    }

    fn update_movement(&mut self, dt: f32) {
        let mut actors: Vec<&mut ActorState> = vec![&mut self.state.player];
        actors.extend(self.state.actors.iter_mut());
        for actor in actors {
            let Some(target) = actor.target else {
                actor.animation.play("idle", None);
                continue;
            };
            if target.x < actor.position.x {
                actor.facing = -1.0;
            } else if target.x > actor.position.x {
                actor.facing = 1.0;
            }
            actor.animation.play("walk", None);
            if actor
                .position
                .step_towards(target, actor.walk_speed.max(0.0) * dt)
            {
                actor.target = None;
                actor.animation.play("idle", None);
            }
        }
    }

    fn update_caption(&mut self, dt: f32) {
        let Some(caption) = self.state.caption.as_mut() else {
            return;
        };
        caption.remaining -= dt;
        if caption.remaining <= 0.0 {
            self.state.caption = None;
            if self.script.status == ScriptStatus::Caption {
                self.script.status = ScriptStatus::Idle;
            }
        }
    }

    fn update_animations(&mut self, dt: f32) {
        let definition = self.definition.clone();
        let mut actors: Vec<&mut ActorState> = vec![&mut self.state.player];
        actors.extend(self.state.actors.iter_mut());
        for actor in actors {
            let animation = actor
                .sprite
                .as_deref()
                .and_then(|sprite| definition.sprite(sprite))
                .and_then(|sheet| sheet.animations.get(&actor.animation.name));
            actor.animation.update(dt, animation);
        }
    }

    fn run_script(&mut self) {
        let mut steps = 0;
        while self.state.running
            && self.script.status == ScriptStatus::Idle
            && !self.awaiting_choice()
            && steps < MAX_STEPS_PER_FRAME
        {
            let Some(step) = self.script.next_step() else {
                break;
            };
            self.execute(step);
            steps += 1;
        }
        if steps == MAX_STEPS_PER_FRAME {
            log::warn!("script did not settle after {MAX_STEPS_PER_FRAME} steps, pausing it");
            self.script.clear();
        }
    }

    // ------------------------------------------------------------------ input

    /// Handles a click inside the scene, in virtual pixel coordinates.
    pub fn click(&mut self, point: Point, button: MouseButton) {
        if !self.state.running || self.awaiting_choice() {
            return;
        }
        if self.state.caption.is_some() {
            self.advance();
            return;
        }
        if self.is_busy() {
            return;
        }

        let verb = match button {
            MouseButton::Right => Some(Verb::Look),
            MouseButton::Left => self.state.forced_verb,
        };
        match self.target_at(point) {
            Some(target) => self.interact(target, verb),
            None => {
                self.state.selected_item = None;
                let destination = self.walkable_point(point);
                self.state.player.target = Some(destination);
            }
        }
    }

    /// Dismisses the caption currently on screen.
    pub fn advance(&mut self) {
        if self.state.caption.take().is_some() && self.script.status == ScriptStatus::Caption {
            self.script.status = ScriptStatus::Idle;
        }
    }

    /// Forces the verb used by the next left click (`None` restores the default).
    pub fn set_verb(&mut self, verb: Option<Verb>) {
        self.state.forced_verb = verb;
    }

    /// Selects, deselects or combines the inventory item at `index`.
    pub fn click_inventory(&mut self, index: usize) {
        if !self.state.running || self.awaiting_choice() || self.script.is_busy() {
            return;
        }
        let Some(item) = self.state.inventory.get(index).cloned() else {
            return;
        };
        match self.state.selected_item.clone() {
            Some(selected) if selected == item => self.state.selected_item = None,
            Some(selected) => {
                self.state.selected_item = None;
                match self.combination(&selected, &item) {
                    Some(actions) => self.script.push_actions(&actions),
                    None => self.say_player("That does not work."),
                }
            }
            None => self.state.selected_item = Some(item),
        }
    }

    /// Describes the inventory item at `index` (right click in the inventory).
    pub fn inspect_inventory(&mut self, index: usize) {
        if !self.state.running || self.script.is_busy() {
            return;
        }
        let Some(item) = self
            .state
            .inventory
            .get(index)
            .and_then(|id| self.definition.item(id))
        else {
            return;
        };
        let text = item
            .description
            .clone()
            .unwrap_or_else(|| format!("A {}.", item.name));
        self.say_player(text);
    }

    /// Picks the visible dialogue choice at `index` (index into [`Engine::choices`]).
    pub fn choose_dialogue(&mut self, index: usize) {
        if !self.awaiting_choice() {
            return;
        }
        let Some(dialogue_state) = self.state.dialogue.clone() else {
            return;
        };
        let Some(choice_index) = dialogue_state.choices.get(index).copied() else {
            return;
        };
        let Some(choice) = self
            .definition
            .dialogue(&dialogue_state.dialogue)
            .and_then(|dialogue| dialogue.nodes.get(&dialogue_state.node))
            .and_then(|node| node.choices.get(choice_index))
            .cloned()
        else {
            return;
        };

        if choice.once {
            self.state.used_choices.insert((
                dialogue_state.dialogue.clone(),
                dialogue_state.node.clone(),
                choice_index,
            ));
        }
        if let Some(dialogue) = self.state.dialogue.as_mut() {
            dialogue.awaiting_choice = false;
            dialogue.choices.clear();
        }

        let mut steps = vec![ScriptStep::Action(Action::Say {
            actor: Some(PLAYER_ID.to_string()),
            text: choice.text.clone(),
            duration: None,
        })];
        steps.extend(choice.actions.iter().cloned().map(ScriptStep::Action));
        steps.push(ScriptStep::ResolveChoice {
            goto: choice.goto.clone(),
            end: choice.end,
        });
        self.script.push_front_all(steps);
    }

    /// The dialogue lines currently offered, as `(index, text)` pairs.
    pub fn choices(&self) -> Vec<(usize, &str)> {
        let Some(dialogue_state) = self.state.dialogue.as_ref() else {
            return Vec::new();
        };
        if !dialogue_state.awaiting_choice {
            return Vec::new();
        }
        let Some(node) = self
            .definition
            .dialogue(&dialogue_state.dialogue)
            .and_then(|dialogue| dialogue.nodes.get(&dialogue_state.node))
        else {
            return Vec::new();
        };
        dialogue_state
            .choices
            .iter()
            .enumerate()
            .filter_map(|(visible_index, choice_index)| {
                node.choices
                    .get(*choice_index)
                    .map(|choice| (visible_index, choice.text.as_str()))
            })
            .collect()
    }

    fn awaiting_choice(&self) -> bool {
        self.state
            .dialogue
            .as_ref()
            .map(|dialogue| dialogue.awaiting_choice)
            .unwrap_or(false)
    }

    // ------------------------------------------------------------ interaction

    /// The topmost object at `point`, actors before hotspots.
    pub fn target_at(&self, point: Point) -> Option<Target> {
        let scene = self.current_scene();
        for actor in self.state.actors.iter().rev() {
            if actor.visible && self.actor_bounds(actor).contains(point) {
                return Some(Target::Actor(actor.id.clone()));
            }
        }
        for hotspot in scene.hotspots.iter().rev() {
            let enabled = self.state.hotspot_enabled(
                &self.definition,
                &self.state.current_scene,
                &hotspot.id,
            );
            if enabled && hotspot.area.contains(point) {
                return Some(Target::Hotspot(hotspot.id.clone()));
            }
        }
        None
    }

    /// Text a UI can show while hovering `point`, e.g. `Use Rope with Door`.
    pub fn hover_label(&self, point: Point) -> Option<String> {
        let name = match self.target_at(point)? {
            Target::Actor(id) => self.state.actor(&id)?.name.clone(),
            Target::Hotspot(id) => self.hotspot(&id)?.name.clone(),
        };
        Some(match self.state.selected_item.as_ref() {
            Some(item) => format!(
                "Use {} with {name}",
                self.definition
                    .item(item)
                    .map(|item| item.name.clone())
                    .unwrap_or_else(|| item.clone())
            ),
            None => match self.state.forced_verb {
                Some(verb) => format!("{} {name}", verb.label()),
                None => name,
            },
        })
    }

    /// Bounding box of an actor, anchored at its feet.
    pub fn actor_bounds(&self, actor: &ActorState) -> Rect {
        let (width, height) = actor
            .sprite
            .as_deref()
            .and_then(|sprite| self.definition.sprite(sprite))
            .map(|sheet| (sheet.frame_width as f32, sheet.frame_height as f32))
            .unwrap_or(DEFAULT_ACTOR_SIZE);
        let width = width * actor.scale;
        let height = height * actor.scale;
        Rect::new(
            actor.position.x - width / 2.0,
            actor.position.y - height,
            width,
            height,
        )
    }

    fn hotspot(&self, id: &str) -> Option<&HotspotDefinition> {
        self.current_scene().hotspot(id)
    }

    fn actor_definition(&self, id: &str) -> Option<&ActorDefinition> {
        self.current_scene().actor(id)
    }

    /// Starts an interaction: walks to the object first when it defines `walk_to`.
    pub fn interact(&mut self, target: Target, verb: Option<Verb>) {
        let (actions, walk_to) = match &target {
            Target::Hotspot(id) => {
                let Some(hotspot) = self.hotspot(id) else {
                    return;
                };
                (
                    self.resolve_actions(
                        &hotspot.interactions,
                        &hotspot.use_with,
                        verb,
                        &hotspot.name,
                    ),
                    hotspot.walk_to,
                )
            }
            Target::Actor(id) => {
                let Some(actor) = self.actor_definition(id) else {
                    return;
                };
                (
                    self.resolve_actions(&actor.interactions, &actor.use_with, verb, &actor.name),
                    actor.walk_to,
                )
            }
        };
        self.state.selected_item = None;
        match walk_to {
            Some(destination) => {
                self.state.player.target = Some(self.walkable_point(destination));
                self.pending = Some(PendingInteraction { actions });
            }
            None => self.script.push_actions(&actions),
        }
    }

    fn resolve_actions(
        &self,
        interactions: &std::collections::BTreeMap<Verb, Vec<Action>>,
        use_with: &std::collections::BTreeMap<String, Vec<Action>>,
        verb: Option<Verb>,
        name: &str,
    ) -> Vec<Action> {
        if let Some(item) = self.state.selected_item.as_ref() {
            return use_with.get(item).cloned().unwrap_or_else(|| {
                vec![say_player(format!(
                    "I cannot use {} with the {name}.",
                    self.definition
                        .item(item)
                        .map(|item| item.name.clone())
                        .unwrap_or_else(|| item.clone())
                ))]
            });
        }
        let verb = verb.unwrap_or_else(|| {
            Verb::PRIORITY
                .iter()
                .copied()
                .find(|verb| interactions.contains_key(verb))
                .unwrap_or(Verb::Look)
        });
        interactions
            .get(&verb)
            .cloned()
            .unwrap_or_else(|| vec![say_player(default_response(verb, name))])
    }

    fn combination(&self, first: &str, second: &str) -> Option<Vec<Action>> {
        let forward = self
            .definition
            .item(first)
            .and_then(|item| item.combine.get(second));
        let backward = self
            .definition
            .item(second)
            .and_then(|item| item.combine.get(first));
        forward.or(backward).cloned()
    }

    fn say_player(&mut self, text: impl Into<String>) {
        self.script.push(ScriptStep::Action(say_player(text)));
    }

    /// Clamps a position into the walkable area of the current scene.
    fn walkable_point(&self, point: Point) -> Point {
        let walkable = &self.current_scene().walkable_area;
        if walkable.len() < 3 {
            point
        } else {
            clamp_to_polygon(point, walkable)
        }
    }

    // ----------------------------------------------------------------- script

    fn execute(&mut self, step: ScriptStep) {
        match step {
            ScriptStep::Action(action) => self.execute_action(action),
            ScriptStep::EnterScene { scene, entry } => self.enter_scene(scene, entry),
            ScriptStep::EnterDialogueNode { node } => self.enter_dialogue_node(node),
            ScriptStep::ResolveDialogueNode => self.resolve_dialogue_node(),
            ScriptStep::ResolveChoice { goto, end } => {
                if end {
                    self.state.dialogue = None;
                } else if let Some(node) = goto {
                    self.script
                        .push_front_all(vec![ScriptStep::EnterDialogueNode { node }]);
                } else {
                    self.script
                        .push_front_all(vec![ScriptStep::ResolveDialogueNode]);
                }
            }
            ScriptStep::MarkVisited { scene } => {
                self.state.visited_scenes.insert(scene);
            }
        }
    }

    fn execute_action(&mut self, action: Action) {
        let definition = self.definition.clone();
        match action {
            Action::Say {
                actor,
                text,
                duration,
            } => {
                let speaker = actor.map(|actor| self.display_name(&actor));
                self.state.caption = Some(Caption {
                    speaker,
                    remaining: duration.unwrap_or_else(|| caption_duration(&text)),
                    text,
                });
                self.script.status = ScriptStatus::Caption;
            }
            Action::GoToScene { scene, entry } => {
                let on_exit = self.current_scene().on_exit.clone();
                self.script.clear();
                self.pending = None;
                let mut steps: Vec<ScriptStep> =
                    on_exit.into_iter().map(ScriptStep::Action).collect();
                steps.push(ScriptStep::EnterScene { scene, entry });
                self.script.push_front_all(steps);
            }
            Action::StartDialogue { dialogue } => {
                let Some(definition) = definition.dialogue(&dialogue) else {
                    return;
                };
                let start = definition.start.clone();
                self.state.dialogue = Some(DialogueState {
                    dialogue,
                    node: start.clone(),
                    choices: Vec::new(),
                    awaiting_choice: false,
                });
                self.script
                    .push_front_all(vec![ScriptStep::EnterDialogueNode { node: start }]);
            }
            Action::EndDialogue => self.state.dialogue = None,
            Action::GiveItem { item } => {
                self.state.inventory.add(item);
            }
            Action::RemoveItem { item } => {
                self.state.inventory.remove(&item);
                if self.state.selected_item.as_deref() == Some(item.as_str()) {
                    self.state.selected_item = None;
                }
            }
            Action::SetVariable { name, value } => {
                self.state.variables.insert(name, value);
            }
            Action::SetHotspot {
                hotspot,
                scene,
                enabled,
            } => {
                let scene = scene.unwrap_or_else(|| self.state.current_scene.clone());
                self.state
                    .hotspot_overrides
                    .insert((scene, hotspot), enabled);
            }
            Action::SetActorVisible { actor, visible } => {
                let scene = self.state.current_scene.clone();
                self.state
                    .actor_overrides
                    .insert((scene, actor.clone()), visible);
                if let Some(actor) = self.state.actor_mut(&actor) {
                    actor.visible = visible;
                }
            }
            Action::PlayAnimation {
                actor,
                animation,
                looping,
            } => {
                if let Some(actor) = self.state.actor_mut(&actor) {
                    actor.animation.play(animation, looping);
                }
            }
            Action::MoveActor { actor, to } => {
                let destination = self.walkable_point(to);
                if let Some(actor_state) = self.state.actor_mut(&actor) {
                    actor_state.target = Some(destination);
                    self.script.status = ScriptStatus::Actor(actor);
                }
            }
            Action::Wait { seconds } => {
                if seconds > 0.0 {
                    self.script.status = ScriptStatus::Waiting(seconds);
                }
            }
            Action::If {
                condition,
                then,
                otherwise,
            } => {
                let branch = if script::evaluate(&condition, &self.state, &definition) {
                    then
                } else {
                    otherwise
                };
                self.script
                    .push_front_all(branch.into_iter().map(ScriptStep::Action).collect());
            }
            Action::EndGame { text } => {
                self.state.running = false;
                self.state.ending_text = text.clone();
                if let Some(text) = text {
                    self.state.caption = Some(Caption {
                        speaker: None,
                        remaining: caption_duration(&text),
                        text,
                    });
                }
                self.script.clear();
                self.pending = None;
            }
        }
    }

    fn enter_scene(&mut self, scene_id: String, entry: Option<String>) {
        let definition = self.definition.clone();
        let Some(scene) = definition.scene(&scene_id) else {
            log::error!("tried to enter unknown scene `{scene_id}`");
            return;
        };

        // The scene that is being left has certainly been visited; the new one only
        // counts as visited once its `on_enter` script has run, so that script can
        // still ask whether this is the first visit.
        let previous = self.state.current_scene.clone();
        if !previous.is_empty() && previous != scene_id {
            self.state.visited_scenes.insert(previous);
        }
        self.state.current_scene = scene_id.clone();
        self.state.dialogue = None;
        self.state.selected_item = None;
        self.pending = None;

        self.state.actors = scene
            .actors
            .iter()
            .map(|actor| {
                let mut state = ActorState::from_definition(actor);
                if let Some(visible) = self
                    .state
                    .actor_overrides
                    .get(&(scene_id.clone(), actor.id.clone()))
                {
                    state.visible = *visible;
                }
                state
            })
            .collect();

        let position = entry
            .as_ref()
            .and_then(|entry| scene.entry_points.get(entry).copied())
            .or_else(|| scene.entry_points.get("default").copied())
            .unwrap_or_else(|| {
                Point::new(
                    definition.window.virtual_width as f32 / 2.0,
                    definition.window.virtual_height as f32 * 0.85,
                )
            });
        self.state.player.position = self.walkable_point(position);
        self.state.player.target = None;
        self.state.player.animation.play("idle", None);

        if scene.on_enter.is_empty() {
            self.state.visited_scenes.insert(scene_id);
        } else {
            let mut steps: Vec<ScriptStep> = scene
                .on_enter
                .iter()
                .cloned()
                .map(ScriptStep::Action)
                .collect();
            steps.push(ScriptStep::MarkVisited { scene: scene_id });
            self.script.push_front_all(steps);
        }
    }

    fn enter_dialogue_node(&mut self, node_id: String) {
        let definition = self.definition.clone();
        let Some(dialogue_state) = self.state.dialogue.as_mut() else {
            return;
        };
        dialogue_state.node = node_id.clone();
        dialogue_state.choices.clear();
        dialogue_state.awaiting_choice = false;
        let dialogue_id = dialogue_state.dialogue.clone();

        let Some(node) = definition
            .dialogue(&dialogue_id)
            .and_then(|dialogue| dialogue.nodes.get(&node_id))
        else {
            self.state.dialogue = None;
            return;
        };

        let mut steps: Vec<ScriptStep> = node
            .actions
            .iter()
            .cloned()
            .map(ScriptStep::Action)
            .collect();
        if let Some(text) = node.text.clone() {
            steps.push(ScriptStep::Action(Action::Say {
                actor: node.speaker.clone(),
                text,
                duration: None,
            }));
        }
        steps.push(ScriptStep::ResolveDialogueNode);
        self.script.push_front_all(steps);
    }

    fn resolve_dialogue_node(&mut self) {
        let definition = self.definition.clone();
        let Some(dialogue_state) = self.state.dialogue.clone() else {
            return;
        };
        let Some(node) = definition
            .dialogue(&dialogue_state.dialogue)
            .and_then(|dialogue| dialogue.nodes.get(&dialogue_state.node))
        else {
            self.state.dialogue = None;
            return;
        };

        if node.end {
            self.state.dialogue = None;
            return;
        }

        let choices: Vec<usize> = node
            .choices
            .iter()
            .enumerate()
            .filter(|(index, choice)| {
                if choice.once
                    && self.state.used_choices.contains(&(
                        dialogue_state.dialogue.clone(),
                        dialogue_state.node.clone(),
                        *index,
                    ))
                {
                    return false;
                }
                choice
                    .condition
                    .as_ref()
                    .map(|condition| script::evaluate(condition, &self.state, &definition))
                    .unwrap_or(true)
            })
            .map(|(index, _)| index)
            .collect();

        if !choices.is_empty() {
            if let Some(dialogue) = self.state.dialogue.as_mut() {
                dialogue.choices = choices;
                dialogue.awaiting_choice = true;
            }
        } else if let Some(goto) = node.goto.clone() {
            self.script
                .push_front_all(vec![ScriptStep::EnterDialogueNode { node: goto }]);
        } else {
            self.state.dialogue = None;
        }
    }

    /// Human readable name of an actor id, falling back to the id itself.
    pub fn display_name(&self, id: &str) -> String {
        if id == PLAYER_ID {
            return self.definition.player.name.clone();
        }
        if let Some(actor) = self.state.actor(id) {
            return actor.name.clone();
        }
        if let Some(actor) = self.actor_definition(id) {
            return actor.name.clone();
        }
        id.to_string()
    }

    /// Sets a variable from host code (useful for tests and tooling).
    pub fn set_variable(&mut self, name: impl Into<String>, value: Value) {
        self.state.variables.insert(name.into(), value);
    }
}

fn say_player(text: impl Into<String>) -> Action {
    Action::Say {
        actor: Some(PLAYER_ID.to_string()),
        text: text.into(),
        duration: None,
    }
}

fn default_response(verb: Verb, name: &str) -> String {
    match verb {
        Verb::Look => format!("Nothing special about the {name}."),
        Verb::Use => format!("I cannot use the {name}."),
        Verb::Talk => format!("The {name} does not answer."),
        Verb::Take => format!("I cannot take the {name}."),
        Verb::Walk => format!("I cannot go to the {name}."),
    }
}
