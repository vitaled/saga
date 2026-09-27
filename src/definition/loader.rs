//! Loading game definitions from YAML files.
//!
//! A game may be written as a single file or split across several files using
//! the top level `include:` key. Included documents are merged (deeply, for
//! mappings) into the including document, and keys of the including document
//! win over included ones.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_yaml::Value as Yaml;

use super::GameDefinition;
use crate::error::{Result, SagaError};

/// A parsed game together with the directory its assets are relative to.
#[derive(Debug, Clone)]
pub struct LoadedGame {
    pub definition: GameDefinition,
    pub base_dir: PathBuf,
}

impl LoadedGame {
    /// Resolves an asset path declared in the YAML against the game directory.
    ///
    /// Absolute paths and paths escaping the game directory are rejected so a
    /// game definition cannot reach arbitrary files on the host.
    pub fn asset_path(&self, relative: &str) -> Result<PathBuf> {
        let path = Path::new(relative);
        if path.is_absolute() {
            return Err(SagaError::Asset(format!(
                "asset path `{relative}` must be relative to the game directory"
            )));
        }
        if path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(SagaError::Asset(format!(
                "asset path `{relative}` must not leave the game directory"
            )));
        }
        Ok(self.base_dir.join(path))
    }
}

/// Loads and validates a game definition from `path`.
pub fn load_game(path: impl AsRef<Path>) -> Result<LoadedGame> {
    let path = path.as_ref();
    let mut visited = BTreeSet::new();
    let document = load_document(path, &mut visited)?;
    let definition: GameDefinition =
        serde_yaml::from_value(document).map_err(|source| SagaError::Yaml {
            path: path.to_path_buf(),
            source,
        })?;
    super::validate::validate(&definition)?;
    let base_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(LoadedGame {
        definition,
        base_dir,
    })
}

/// Parses and validates a game definition from a YAML string.
///
/// `include` is not supported here since there is no directory to resolve
/// relative paths against.
pub fn parse_game(yaml: &str) -> Result<GameDefinition> {
    let definition: GameDefinition =
        serde_yaml::from_str(yaml).map_err(|source| SagaError::Yaml {
            path: PathBuf::from("<memory>"),
            source,
        })?;
    super::validate::validate(&definition)?;
    Ok(definition)
}

fn load_document(path: &Path, visited: &mut BTreeSet<PathBuf>) -> Result<Yaml> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !visited.insert(canonical) {
        return Err(SagaError::Asset(format!(
            "circular include detected for `{}`",
            path.display()
        )));
    }

    let text = std::fs::read_to_string(path).map_err(|source| SagaError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut document: Yaml = serde_yaml::from_str(&text).map_err(|source| SagaError::Yaml {
        path: path.to_path_buf(),
        source,
    })?;

    let includes = take_includes(&mut document, path)?;
    if includes.is_empty() {
        return Ok(document);
    }

    let directory = path.parent().unwrap_or_else(|| Path::new("."));
    let mut merged = Yaml::Mapping(Default::default());
    for include in includes {
        let included = load_document(&directory.join(include), visited)?;
        merge(&mut merged, included);
    }
    merge(&mut merged, document);
    Ok(merged)
}

fn take_includes(document: &mut Yaml, path: &Path) -> Result<Vec<String>> {
    let Yaml::Mapping(mapping) = document else {
        return Err(SagaError::Asset(format!(
            "`{}` must contain a YAML mapping",
            path.display()
        )));
    };
    let Some(value) = mapping.remove(Yaml::String("include".to_string())) else {
        return Ok(Vec::new());
    };
    match value {
        Yaml::String(single) => Ok(vec![single]),
        Yaml::Sequence(values) => values
            .into_iter()
            .map(|value| match value {
                Yaml::String(name) => Ok(name),
                other => Err(SagaError::Asset(format!(
                    "`include` entries must be strings, found {other:?} in `{}`",
                    path.display()
                ))),
            })
            .collect(),
        other => Err(SagaError::Asset(format!(
            "`include` must be a string or a list of strings, found {other:?} in `{}`",
            path.display()
        ))),
    }
}

/// Deep merges `overlay` into `base`; scalars and sequences from the overlay win.
fn merge(base: &mut Yaml, overlay: Yaml) {
    match (base, overlay) {
        (Yaml::Mapping(base), Yaml::Mapping(overlay)) => {
            for (key, value) in overlay {
                match base.get_mut(&key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: &str = r#"
title: Test Game
start_scene: room
scenes:
  room:
    name: Room
"#;

    #[test]
    fn parses_a_minimal_game() {
        let game = parse_game(MAIN).expect("game parses");
        assert_eq!(game.title, "Test Game");
        assert_eq!(game.window.virtual_width, 640);
        assert!(game.scenes.contains_key("room"));
    }

    #[test]
    fn merges_included_documents() {
        let dir = std::env::temp_dir().join(format!("saga-include-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("scenes.yaml"),
            "scenes:\n  hall:\n    name: Hall\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("game.yaml"),
            format!("include:\n  - scenes.yaml\n{MAIN}"),
        )
        .unwrap();

        let loaded = load_game(dir.join("game.yaml")).expect("game loads");
        assert_eq!(loaded.definition.scenes.len(), 2);
        assert!(loaded.definition.scenes.contains_key("hall"));
        assert_eq!(loaded.base_dir, dir);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_asset_paths_escaping_the_game_directory() {
        let loaded = LoadedGame {
            definition: parse_game(MAIN).unwrap(),
            base_dir: PathBuf::from("/games/demo"),
        };
        assert!(loaded.asset_path("art/room.png").is_ok());
        assert!(loaded.asset_path("../../etc/passwd").is_err());
        assert!(loaded.asset_path("/etc/passwd").is_err());
    }
}
