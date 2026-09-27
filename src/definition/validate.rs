//! Static validation of a game definition.
//!
//! Validation catches broken references (scenes, items, dialogues, sprites,
//! animations, dialogue nodes) before the game is started, so authors get a
//! precise list of problems instead of a runtime failure.

use std::collections::BTreeSet;

use super::{Action, Condition, GameDefinition, SceneDefinition};
use crate::error::{Result, ValidationErrors};

/// Validates every reference inside `game`.
pub fn validate(game: &GameDefinition) -> Result<()> {
    let mut errors = ValidationErrors::default();

    if game.scenes.is_empty() {
        errors.push("the game defines no scenes");
    }
    if !game.scenes.contains_key(&game.start_scene) {
        errors.push(format!(
            "start_scene `{}` does not exist",
            game.start_scene
        ));
    } else if let Some(entry) = &game.start_entry {
        let scene = &game.scenes[&game.start_scene];
        if !scene.entry_points.contains_key(entry) {
            errors.push(format!(
                "start_entry `{entry}` does not exist in scene `{}`",
                game.start_scene
            ));
        }
    }

    if let Some(sprite) = &game.player.sprite {
        check_sprite(game, sprite, "player", None, &mut errors);
    }

    for (item_id, item) in &game.items {
        if let Some(icon) = &item.icon {
            check_sprite(game, icon, &format!("item `{item_id}`"), None, &mut errors);
        }
        for (other, actions) in &item.combine {
            if !game.items.contains_key(other) {
                errors.push(format!(
                    "item `{item_id}` combines with unknown item `{other}`"
                ));
            }
            check_actions(game, actions, &format!("item `{item_id}` combine `{other}`"), &mut errors);
        }
    }

    for (sprite_id, sprite) in &game.sprites {
        if sprite.frame_width == 0 || sprite.frame_height == 0 {
            errors.push(format!(
                "sprite `{sprite_id}` must declare a non zero frame size"
            ));
        }
        for (animation_id, animation) in &sprite.animations {
            if animation.frames.is_empty() {
                errors.push(format!(
                    "animation `{animation_id}` of sprite `{sprite_id}` has no frames"
                ));
            }
            if animation.fps <= 0.0 {
                errors.push(format!(
                    "animation `{animation_id}` of sprite `{sprite_id}` must have a positive fps"
                ));
            }
        }
    }

    for (dialogue_id, dialogue) in &game.dialogues {
        let context = format!("dialogue `{dialogue_id}`");
        if !dialogue.nodes.contains_key(&dialogue.start) {
            errors.push(format!(
                "{context} starts at unknown node `{}`",
                dialogue.start
            ));
        }
        for (node_id, node) in &dialogue.nodes {
            let context = format!("{context} node `{node_id}`");
            if let Some(goto) = &node.goto {
                if !dialogue.nodes.contains_key(goto) {
                    errors.push(format!("{context} jumps to unknown node `{goto}`"));
                }
            }
            if node.goto.is_none() && node.choices.is_empty() && !node.end {
                errors.push(format!(
                    "{context} has neither choices, a `goto` nor `end: true`"
                ));
            }
            check_actions(game, &node.actions, &context, &mut errors);
            for (index, choice) in node.choices.iter().enumerate() {
                let context = format!("{context} choice {index}");
                if let Some(goto) = &choice.goto {
                    if !dialogue.nodes.contains_key(goto) {
                        errors.push(format!("{context} jumps to unknown node `{goto}`"));
                    }
                }
                if choice.goto.is_none() && !choice.end && choice.actions.is_empty() {
                    errors.push(format!("{context} does nothing"));
                }
                if let Some(condition) = &choice.condition {
                    check_condition(game, condition, &context, &mut errors);
                }
                check_actions(game, &choice.actions, &context, &mut errors);
            }
        }
    }

    for (scene_id, scene) in &game.scenes {
        validate_scene(game, scene_id, scene, &mut errors);
    }

    errors.into_result()
}

fn validate_scene(
    game: &GameDefinition,
    scene_id: &str,
    scene: &SceneDefinition,
    errors: &mut ValidationErrors,
) {
    let context = format!("scene `{scene_id}`");
    if !scene.walkable_area.is_empty() && scene.walkable_area.len() < 3 {
        errors.push(format!(
            "{context} has a walkable_area with fewer than 3 points"
        ));
    }

    let mut hotspot_ids = BTreeSet::new();
    for hotspot in &scene.hotspots {
        if !hotspot_ids.insert(hotspot.id.as_str()) {
            errors.push(format!(
                "{context} declares hotspot `{}` more than once",
                hotspot.id
            ));
        }
        let context = format!("{context} hotspot `{}`", hotspot.id);
        for (verb, actions) in &hotspot.interactions {
            check_actions(game, actions, &format!("{context} verb `{verb:?}`"), errors);
        }
        for (item, actions) in &hotspot.use_with {
            if !game.items.contains_key(item) {
                errors.push(format!("{context} reacts to unknown item `{item}`"));
            }
            check_actions(game, actions, &format!("{context} use_with `{item}`"), errors);
        }
    }

    let mut actor_ids = BTreeSet::new();
    for actor in &scene.actors {
        if !actor_ids.insert(actor.id.as_str()) {
            errors.push(format!(
                "{context} declares actor `{}` more than once",
                actor.id
            ));
        }
        if actor.id == crate::engine::PLAYER_ID {
            errors.push(format!(
                "{context} declares an actor with the reserved id `{}`",
                crate::engine::PLAYER_ID
            ));
        }
        let context = format!("{context} actor `{}`", actor.id);
        if let Some(sprite) = &actor.sprite {
            check_sprite(game, sprite, &context, Some(&actor.animation), errors);
        }
        for (verb, actions) in &actor.interactions {
            check_actions(game, actions, &format!("{context} verb `{verb:?}`"), errors);
        }
        for (item, actions) in &actor.use_with {
            if !game.items.contains_key(item) {
                errors.push(format!("{context} reacts to unknown item `{item}`"));
            }
            check_actions(game, actions, &format!("{context} use_with `{item}`"), errors);
        }
    }

    check_actions(game, &scene.on_enter, &format!("{context} on_enter"), errors);
    check_actions(game, &scene.on_exit, &format!("{context} on_exit"), errors);
}

fn check_sprite(
    game: &GameDefinition,
    sprite_id: &str,
    context: &str,
    animation: Option<&str>,
    errors: &mut ValidationErrors,
) {
    let Some(sprite) = game.sprites.get(sprite_id) else {
        errors.push(format!("{context} uses unknown sprite `{sprite_id}`"));
        return;
    };
    if let Some(animation) = animation {
        if !sprite.animations.contains_key(animation) {
            errors.push(format!(
                "{context} uses unknown animation `{animation}` of sprite `{sprite_id}`"
            ));
        }
    }
}

fn check_actions(
    game: &GameDefinition,
    actions: &[Action],
    context: &str,
    errors: &mut ValidationErrors,
) {
    for action in actions {
        match action {
            Action::GoToScene { scene, entry } => match game.scenes.get(scene) {
                None => errors.push(format!("{context} jumps to unknown scene `{scene}`")),
                Some(target) => {
                    if let Some(entry) = entry {
                        if !target.entry_points.contains_key(entry) {
                            errors.push(format!(
                                "{context} uses unknown entry point `{entry}` of scene `{scene}`"
                            ));
                        }
                    }
                }
            },
            Action::StartDialogue { dialogue } => {
                if !game.dialogues.contains_key(dialogue) {
                    errors.push(format!("{context} starts unknown dialogue `{dialogue}`"));
                }
            }
            Action::GiveItem { item } | Action::RemoveItem { item } => {
                if !game.items.contains_key(item) {
                    errors.push(format!("{context} refers to unknown item `{item}`"));
                }
            }
            Action::SetHotspot {
                hotspot,
                scene: Some(scene),
                ..
            } => match game.scenes.get(scene) {
                None => errors.push(format!("{context} refers to unknown scene `{scene}`")),
                Some(target) => {
                    if target.hotspot(hotspot).is_none() {
                        errors.push(format!(
                            "{context} refers to unknown hotspot `{hotspot}` of scene `{scene}`"
                        ));
                    }
                }
            },
            Action::Wait { seconds } => {
                if *seconds < 0.0 {
                    errors.push(format!("{context} waits for a negative amount of time"));
                }
            }
            Action::If {
                condition,
                then,
                otherwise,
            } => {
                check_condition(game, condition, context, errors);
                check_actions(game, then, context, errors);
                check_actions(game, otherwise, context, errors);
            }
            _ => {}
        }
    }
}

fn check_condition(
    game: &GameDefinition,
    condition: &Condition,
    context: &str,
    errors: &mut ValidationErrors,
) {
    match condition {
        Condition::HasItem { item } => {
            if !game.items.contains_key(item) {
                errors.push(format!("{context} checks unknown item `{item}`"));
            }
        }
        Condition::VisitedScene { scene } => {
            if !game.scenes.contains_key(scene) {
                errors.push(format!("{context} checks unknown scene `{scene}`"));
            }
        }
        Condition::HotspotEnabled {
            hotspot,
            scene: Some(scene),
        } => match game.scenes.get(scene) {
            None => errors.push(format!("{context} checks unknown scene `{scene}`")),
            Some(target) => {
                if target.hotspot(hotspot).is_none() {
                    errors.push(format!(
                        "{context} checks unknown hotspot `{hotspot}` of scene `{scene}`"
                    ));
                }
            }
        },
        Condition::Not { condition } => check_condition(game, condition, context, errors),
        Condition::All { conditions } | Condition::Any { conditions } => {
            for condition in conditions {
                check_condition(game, condition, context, errors);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::definition::loader::parse_game;

    #[test]
    fn reports_unknown_scene_references() {
        let yaml = r#"
title: Broken
start_scene: nowhere
scenes:
  room:
    name: Room
    hotspots:
      - id: door
        name: Door
        area: { x: 0, y: 0, width: 10, height: 10 }
        interactions:
          use:
            - action: go_to_scene
              scene: attic
"#;
        let error = parse_game(yaml).expect_err("validation fails");
        let message = error.to_string();
        assert!(message.contains("start_scene `nowhere` does not exist"), "{message}");
        assert!(message.contains("unknown scene `attic`"), "{message}");
    }

    #[test]
    fn reports_unknown_dialogue_nodes_and_items() {
        let yaml = r#"
title: Broken
start_scene: room
dialogues:
  chat:
    start: hello
    nodes:
      hello:
        text: Hi
        choices:
          - text: Bye
            goto: missing
          - text: Trade
            condition:
              check: has_item
              item: gold
            end: true
scenes:
  room:
    name: Room
    on_enter:
      - action: give_item
        item: gold
"#;
        let error = parse_game(yaml).expect_err("validation fails");
        let message = error.to_string();
        assert!(message.contains("unknown node `missing`"), "{message}");
        assert!(message.contains("unknown item `gold`"), "{message}");
    }

    #[test]
    fn accepts_a_consistent_game() {
        let yaml = r#"
title: Fine
start_scene: room
items:
  gold:
    name: Gold coin
dialogues:
  chat:
    start: hello
    nodes:
      hello:
        text: Hi
        choices:
          - text: Bye
            end: true
scenes:
  room:
    name: Room
    entry_points:
      default: { x: 10, y: 20 }
    hotspots:
      - id: chest
        name: Chest
        area: { x: 0, y: 0, width: 10, height: 10 }
        interactions:
          take:
            - action: give_item
              item: gold
    actors:
      - id: sailor
        name: Sailor
        position: { x: 100, y: 200 }
        interactions:
          talk:
            - action: start_dialogue
              dialogue: chat
"#;
        parse_game(yaml).expect("game is valid");
    }
}
