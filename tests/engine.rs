//! End-to-end tests of the engine, driven exactly like the real game loop.

use saga::definition::loader::parse_game;
use saga::definition::{Value, Verb};
use saga::engine::{Engine, MouseButton, Target};
use saga::geometry::Point;

const GAME: &str = r#"
title: Test Adventure
start_scene: beach
start_entry: start
variables:
  talked: false
items:
  shovel:
    name: shovel
    description: A sturdy shovel.
  map:
    name: map
    combine:
      shovel:
        - action: say
          text: "Now I know where to dig."
        - action: set_variable
          name: knows_spot
          value: true
dialogues:
  sailor_chat:
    start: greeting
    nodes:
      greeting:
        speaker: Sailor
        text: "What do you want?"
        choices:
          - text: "Do you have a shovel?"
            once: true
            actions:
              - action: give_item
                item: shovel
            goto: gave_shovel
          - text: "Nothing, bye."
            end: true
      gave_shovel:
        speaker: Sailor
        text: "Here, take mine."
        actions:
          - action: set_variable
            name: talked
            value: true
        goto: greeting
scenes:
  beach:
    name: Beach
    walkable_area:
      - { x: 0, y: 200 }
      - { x: 320, y: 200 }
      - { x: 320, y: 340 }
      - { x: 0, y: 340 }
    entry_points:
      start: { x: 40, y: 300 }
      from_cave: { x: 300, y: 300 }
    hotspots:
      - id: sand
        name: sand
        area: { x: 10, y: 250, width: 60, height: 40 }
        walk_to: { x: 40, y: 300 }
        interactions:
          look:
            - action: say
              text: "Just sand."
          use:
            - action: if
              condition:
                check: has_item
                item: shovel
              then:
                - action: say
                  text: "I dig up a map."
                - action: give_item
                  item: map
              else:
                - action: say
                  text: "I need a tool."
        use_with:
          shovel:
            - action: say
              text: "I dig up a map."
            - action: give_item
              item: map
      - id: cave_entrance
        name: cave
        area: { x: 260, y: 220, width: 50, height: 80 }
        walk_to: { x: 280, y: 300 }
        interactions:
          walk:
            - action: go_to_scene
              scene: cave
              entry: from_beach
    actors:
      - id: sailor
        name: Sailor
        position: { x: 150, y: 300 }
        walk_to: { x: 140, y: 300 }
        interactions:
          talk:
            - action: start_dialogue
              dialogue: sailor_chat
  cave:
    name: Cave
    entry_points:
      from_beach: { x: 60, y: 300 }
    on_enter:
      - action: say
        text: "It is dark in here."
    hotspots:
      - id: treasure
        name: treasure
        area: { x: 100, y: 200, width: 40, height: 40 }
        interactions:
          take:
            - action: end_game
              text: "You win!"
"#;

/// Runs the engine until it becomes idle, mimicking the real frame loop while
/// a very fast player skips every caption.
fn settle(engine: &mut Engine) {
    for _ in 0..2000 {
        engine.update(1.0 / 60.0);
        if !engine.is_busy() || engine.is_finished() {
            return;
        }
        engine.advance();
    }
    panic!("engine did not settle");
}

fn settle_until_caption(engine: &mut Engine) {
    for _ in 0..600 {
        engine.update(1.0 / 60.0);
        if engine.state().caption.is_some() {
            return;
        }
    }
    panic!("no caption appeared");
}

fn caption(engine: &Engine) -> Option<String> {
    engine
        .state()
        .caption
        .as_ref()
        .map(|caption| caption.text.clone())
}

fn choice_texts(engine: &Engine) -> Vec<String> {
    engine
        .choices()
        .iter()
        .map(|(_, text)| text.to_string())
        .collect()
}

fn enter_cave(engine: &mut Engine) {
    engine.click(Point::new(280.0, 250.0), MouseButton::Left);
    for _ in 0..600 {
        engine.update(1.0 / 60.0);
        if engine.state().current_scene == "cave" {
            return;
        }
    }
    panic!("the cave was never entered");
}

fn engine() -> Engine {
    Engine::new(parse_game(GAME).expect("the test game is valid")).expect("engine starts")
}

#[test]
fn the_game_starts_in_the_configured_scene_and_entry_point() {
    let engine = engine();
    assert_eq!(engine.current_scene().name, "Beach");
    assert_eq!(engine.state().player.position, Point::new(40.0, 300.0));
    assert!(engine.state().visited_scenes.contains("beach"));
}

#[test]
fn clicking_the_floor_walks_the_player_inside_the_walkable_area() {
    let mut engine = engine();
    engine.click(Point::new(200.0, 500.0), MouseButton::Left);
    assert_eq!(engine.state().player.target, Some(Point::new(200.0, 340.0)));

    for _ in 0..600 {
        engine.update(1.0 / 60.0);
    }
    assert_eq!(engine.state().player.position, Point::new(200.0, 340.0));
    assert!(engine.state().player.target.is_none());
}

#[test]
fn right_clicking_a_hotspot_looks_at_it_after_walking_there() {
    let mut engine = engine();
    engine.click(Point::new(30.0, 270.0), MouseButton::Right);
    assert_eq!(engine.state().player.target, Some(Point::new(40.0, 300.0)));
    settle_until_caption(&mut engine);
    assert_eq!(caption(&engine).as_deref(), Some("Just sand."));
}

#[test]
fn hotspots_fall_back_to_a_default_response() {
    let mut engine = engine();
    engine.set_verb(Some(Verb::Take));
    engine.click(Point::new(30.0, 270.0), MouseButton::Left);
    settle_until_caption(&mut engine);
    assert_eq!(caption(&engine).as_deref(), Some("I cannot take the sand."));
}

#[test]
fn conditional_actions_follow_the_inventory() {
    let mut engine = engine();
    engine.set_verb(Some(Verb::Use));
    engine.click(Point::new(30.0, 270.0), MouseButton::Left);
    settle_until_caption(&mut engine);
    assert_eq!(caption(&engine).as_deref(), Some("I need a tool."));
}

#[test]
fn dialogue_choices_run_actions_and_branch() {
    let mut engine = engine();
    assert_eq!(
        engine.target_at(Point::new(150.0, 280.0)),
        Some(Target::Actor("sailor".to_string()))
    );
    engine.click(Point::new(150.0, 280.0), MouseButton::Left);
    settle(&mut engine);

    assert_eq!(
        choice_texts(&engine),
        vec!["Do you have a shovel?", "Nothing, bye."]
    );

    engine.choose_dialogue(0);
    settle(&mut engine);
    assert!(engine.state().inventory.contains("shovel"));
    assert_eq!(engine.state().variable("talked"), Some(&Value::Bool(true)));

    // The `once` choice is gone the second time the node is shown.
    assert_eq!(choice_texts(&engine), vec!["Nothing, bye."]);

    engine.choose_dialogue(0);
    settle(&mut engine);
    assert!(engine.state().dialogue.is_none());
}

#[test]
fn items_can_be_used_on_hotspots_and_combined() {
    let mut engine = engine();
    engine.click(Point::new(150.0, 280.0), MouseButton::Left);
    settle(&mut engine);
    engine.choose_dialogue(0);
    settle(&mut engine);
    engine.choose_dialogue(0);
    settle(&mut engine);

    engine.click_inventory(0);
    assert_eq!(engine.state().selected_item.as_deref(), Some("shovel"));
    assert_eq!(
        engine.hover_label(Point::new(30.0, 270.0)).as_deref(),
        Some("Use shovel with sand")
    );
    engine.click(Point::new(30.0, 270.0), MouseButton::Left);
    settle(&mut engine);
    assert!(engine.state().inventory.contains("map"));

    engine.click_inventory(0);
    engine.click_inventory(1);
    settle_until_caption(&mut engine);
    assert_eq!(caption(&engine).as_deref(), Some("Now I know where to dig."));
    settle(&mut engine);
    assert_eq!(
        engine.state().variable("knows_spot"),
        Some(&Value::Bool(true))
    );
}

#[test]
fn scene_transitions_place_the_player_and_run_on_enter() {
    let mut engine = engine();
    enter_cave(&mut engine);
    assert_eq!(engine.state().player.position, Point::new(60.0, 300.0));
    settle_until_caption(&mut engine);
    assert_eq!(caption(&engine).as_deref(), Some("It is dark in here."));
}

#[test]
fn the_game_can_end() {
    let mut engine = engine();
    enter_cave(&mut engine);
    settle(&mut engine);
    engine.click(Point::new(120.0, 220.0), MouseButton::Left);
    settle(&mut engine);
    assert!(engine.is_finished());
    assert_eq!(engine.state().ending_text.as_deref(), Some("You win!"));
}
