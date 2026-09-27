//! Plays the bundled demo game from the first scene to the ending, headlessly.
//!
//! This guards both the engine and `games/demo/game.yaml`: the shipped demo
//! has to stay winnable.

use saga::definition::loader::load_game;
use saga::definition::Verb;
use saga::engine::{Engine, MouseButton};
use saga::geometry::Point;

fn demo() -> Engine {
    let game = load_game(concat!(env!("CARGO_MANIFEST_DIR"), "/games/demo/game.yaml"))
        .expect("the demo game loads and validates");
    Engine::new(game.definition).expect("the demo game starts")
}

/// Runs frames until the engine is idle, skipping captions like an impatient player.
fn settle(engine: &mut Engine) {
    for _ in 0..5000 {
        engine.update(1.0 / 60.0);
        if engine.is_finished() || !engine.is_busy() {
            return;
        }
        engine.advance();
    }
    panic!("the engine never settled");
}

fn click(engine: &mut Engine, x: f32, y: f32) {
    engine.click(Point::new(x, y), MouseButton::Left);
    settle(engine);
}

fn choose(engine: &mut Engine, text: &str) {
    let index = engine
        .choices()
        .iter()
        .find(|(_, choice)| *choice == text)
        .map(|(index, _)| *index)
        .unwrap_or_else(|| panic!("choice `{text}` is not offered: {:?}", engine.choices()));
    engine.choose_dialogue(index);
    settle(engine);
}

fn select(engine: &mut Engine, item: &str) {
    let index = engine
        .state()
        .inventory
        .items()
        .iter()
        .position(|candidate| candidate == item)
        .unwrap_or_else(|| panic!("`{item}` is not in the inventory"));
    engine.click_inventory(index);
    assert_eq!(engine.state().selected_item.as_deref(), Some(item));
}

#[test]
fn the_demo_game_is_valid() {
    let engine = demo();
    assert_eq!(engine.definition().title, "The Keeper's Lamp");
    assert_eq!(engine.current_scene().name, "Stone Quay");
    assert_eq!(engine.state().player.position, Point::new(120.0, 320.0));
}

#[test]
fn the_demo_game_can_be_finished() {
    let mut engine = demo();
    settle(&mut engine);

    // Talk to Marla and ask for her rope.
    click(&mut engine, 300.0, 300.0);
    choose(&mut engine, "Could I borrow that rope?");
    assert!(engine.state().inventory.contains("rope"));
    choose(&mut engine, "Just enjoying the view.");
    assert!(engine.state().dialogue.is_none());

    // Fish the key out of the rain barrel with the rope.
    select(&mut engine, "rope");
    click(&mut engine, 486.0, 260.0);
    assert!(engine.state().inventory.contains("rusty_key"));

    // Take the storm lantern from the crates.
    engine.set_verb(Some(Verb::Take));
    click(&mut engine, 78.0, 244.0);
    engine.set_verb(None);
    assert!(engine.state().inventory.contains("lantern"));

    // Walk up the cliff path.
    click(&mut engine, 615.0, 240.0);
    assert_eq!(engine.state().current_scene, "cliff");

    // Light up the sea cave to find the oil can.
    select(&mut engine, "lantern");
    click(&mut engine, 70.0, 240.0);
    assert!(engine.state().inventory.contains("oil_can"));

    // Unlock the tower with the rusty key.
    select(&mut engine, "rusty_key");
    click(&mut engine, 505.0, 260.0);
    assert_eq!(engine.state().current_scene, "lighthouse");
    assert!(!engine.state().inventory.contains("rusty_key"));

    // Fill and light the great lamp.
    select(&mut engine, "oil_can");
    click(&mut engine, 295.0, 200.0);
    assert!(engine.is_finished());
    assert!(engine
        .state()
        .ending_text
        .as_deref()
        .unwrap_or_default()
        .contains("THE END"));
}

#[test]
fn locked_doors_stay_locked_without_the_key() {
    let mut engine = demo();
    settle(&mut engine);
    click(&mut engine, 615.0, 240.0);
    assert_eq!(engine.state().current_scene, "cliff");

    click(&mut engine, 505.0, 260.0);
    assert_eq!(engine.state().current_scene, "cliff");
}

#[test]
fn the_minimal_example_game_is_valid() {
    let game = load_game(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/games/minimal/game.yaml"
    ))
    .expect("the minimal game loads and validates");
    let mut engine = Engine::new(game.definition).expect("the minimal game starts");
    settle(&mut engine);

    engine.set_verb(Some(Verb::Take));
    click(&mut engine, 150.0, 240.0);
    assert!(engine.state().inventory.contains("key"));

    engine.set_verb(None);
    select(&mut engine, "key");
    click(&mut engine, 450.0, 220.0);
    assert!(engine.is_finished());
}
