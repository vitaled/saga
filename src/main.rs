//! The `saga` command line tool: plays and checks YAML games.

use std::path::PathBuf;
use std::process::ExitCode;

use saga::definition::loader::load_game;

const USAGE: &str = "\
SAGA - SAGA's Adventure Game Architecture

Usage:
  saga run <game.yaml>       Play a game (default command)
  saga validate <game.yaml>  Check a game definition and exit
  saga info <game.yaml>      Print a summary of a game definition
";

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut arguments = std::env::args().skip(1);
    let (command, path) = match (arguments.next(), arguments.next()) {
        (Some(command), Some(path)) => (command, PathBuf::from(path)),
        (Some(path), None) if !path.starts_with('-') => ("run".to_string(), PathBuf::from(path)),
        _ => {
            eprint!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    let game = match load_game(&path) {
        Ok(game) => game,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    match command.as_str() {
        "validate" => {
            println!("{} is valid.", path.display());
            ExitCode::SUCCESS
        }
        "info" => {
            print_info(&game);
            ExitCode::SUCCESS
        }
        "run" => match saga::app::run(game) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        other => {
            eprintln!("unknown command `{other}`\n");
            eprint!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn print_info(game: &saga::LoadedGame) {
    let definition = &game.definition;
    println!("{}", definition.title);
    if let Some(author) = &definition.author {
        println!("by {author}");
    }
    if let Some(version) = &definition.version {
        println!("version {version}");
    }
    println!(
        "{} scene(s), {} item(s), {} dialogue(s), {} sprite sheet(s)",
        definition.scenes.len(),
        definition.items.len(),
        definition.dialogues.len(),
        definition.sprites.len()
    );
    println!("starts in `{}`", definition.start_scene);
    for (id, scene) in &definition.scenes {
        println!(
            "  - {id}: {} ({} hotspot(s), {} actor(s))",
            scene.name,
            scene.hotspots.len(),
            scene.actors.len()
        );
    }
}
