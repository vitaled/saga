//! Window creation and the main loop.

use std::sync::Arc;
use std::time::Instant;

use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Fullscreen, WindowBuilder};

use crate::definition::loader::LoadedGame;
use crate::definition::Verb;
use crate::engine::{Engine, MouseButton};
use crate::error::{Result, SagaError};
use crate::geometry::Point;
use crate::render::Renderer;
use crate::ui::GameUi;

/// Largest frame time fed to the engine, so a hitch cannot teleport actors.
const MAX_FRAME_TIME: f32 = 0.1;

/// Opens a window and plays `game` until the player closes it.
pub fn run(game: LoadedGame) -> Result<()> {
    let window_config = game.definition.window.clone();
    let title = game.definition.title.clone();
    let mut engine = Engine::new(game.definition.clone())?;
    let mut ui = GameUi::new(game);

    let event_loop = EventLoop::new()
        .map_err(|error| SagaError::Graphics(format!("cannot create an event loop: {error}")))?;
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(PhysicalSize::new(window_config.width, window_config.height))
        .with_fullscreen(
            window_config
                .fullscreen
                .then_some(Fullscreen::Borderless(None)),
        )
        .build(&event_loop)
        .map_err(|error| SagaError::Graphics(format!("cannot create a window: {error}")))?;
    let window = Arc::new(window);

    let mut renderer = Renderer::new(
        window.clone(),
        (window_config.virtual_width, window_config.virtual_height),
    )?;
    let mut cursor = Point::new(-1.0, -1.0);
    let mut last_frame = Instant::now();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop
        .run(move |event, target| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
                WindowEvent::CursorMoved { position, .. } => {
                    cursor = renderer.window_to_virtual(position.x as f32, position.y as f32);
                    ui.set_cursor(cursor);
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button,
                    ..
                } => {
                    let button = match button {
                        WinitMouseButton::Right => Some(MouseButton::Right),
                        WinitMouseButton::Left => Some(MouseButton::Left),
                        _ => None,
                    };
                    if let Some(button) = button {
                        ui.click(&mut engine, cursor, button);
                    }
                }
                WindowEvent::KeyboardInput { event, .. }
                    if event.state == ElementState::Pressed =>
                {
                    match event.logical_key.as_ref() {
                        Key::Named(NamedKey::Escape) => target.exit(),
                        Key::Named(NamedKey::Space) | Key::Named(NamedKey::Enter) => {
                            engine.advance()
                        }
                        Key::Character(character) => handle_shortcut(&mut engine, character),
                        _ => {}
                    }
                }
                WindowEvent::RedrawRequested => {
                    let now = Instant::now();
                    let dt = (now - last_frame).as_secs_f32().min(MAX_FRAME_TIME);
                    last_frame = now;

                    engine.update(dt);
                    ui.draw(&engine, &mut renderer);
                    if let Err(error) = renderer.render() {
                        log::error!("{error}");
                        target.exit();
                    }
                }
                _ => {}
            },
            Event::AboutToWait => window.request_redraw(),
            _ => {}
        })
        .map_err(|error| SagaError::Graphics(format!("the event loop failed: {error}")))
}

/// Number keys select a verb, `0` clears the selection.
fn handle_shortcut(engine: &mut Engine, character: &str) {
    if character == "0" {
        engine.set_verb(None);
        return;
    }
    let Some(index) = character
        .parse::<usize>()
        .ok()
        .filter(|index| (1..=Verb::PRIORITY.len()).contains(index))
    else {
        return;
    };
    let verb = Verb::PRIORITY[index - 1];
    let active = engine.state().forced_verb == Some(verb);
    engine.set_verb((!active).then_some(verb));
}
