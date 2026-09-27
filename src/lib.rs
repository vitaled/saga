//! SAGA - *SAGA's Adventure Game Architecture*.
//!
//! SAGA is a 2D point-and-click adventure game framework. Games are written
//! entirely in YAML: scenes, hotspots, actors, dialogues, items and the
//! scripted actions tying them together. The crate is split in three layers:
//!
//! * [`definition`] - the YAML schema plus loading and validation,
//! * [`engine`] - the headless runtime (state, scripting, dialogue, inventory),
//! * [`render`], [`ui`] and [`app`] - a GPU accelerated wgpu renderer, the
//!   presentation layer and the winit main loop.
//!
//! ```no_run
//! let game = saga::definition::loader::load_game("games/demo/game.yaml")?;
//! saga::app::run(game)?;
//! # Ok::<(), saga::error::SagaError>(())
//! ```

pub mod app;
pub mod definition;
pub mod engine;
pub mod error;
pub mod geometry;
pub mod render;
pub mod ui;

pub use definition::loader::{load_game, parse_game, LoadedGame};
pub use definition::GameDefinition;
pub use engine::Engine;
pub use error::{Result, SagaError};
