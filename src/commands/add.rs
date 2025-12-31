use crate::store::{AliasScope, Store};

pub fn handle_add_command(
    store: &mut Store,
    alias: String,
    command: String,
    scope: Option<String>,
    recursive: bool,
) -> std::result::Result<String, crate::error::AkaError> {
    let scope = if let Some(d) = scope {
        let path = std::fs::canonicalize(d)
            .map_err(|e| crate::error::AkaError::ConfigError(e.to_string()))?;
        let path_str = path.to_string_lossy().to_string();
        if recursive {
            AliasScope::Recursive(path_str)
        } else {
            AliasScope::Exact(path_str)
        }
    } else {
        AliasScope::Global
    };

    store.add(alias.clone(), command.clone(), scope)?;
    Ok(format!(
        "Added alias '{}' for '{}'\n(Reload shell to apply)",
        alias, command
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;
    use rstest::rstest;
    use tempfile::tempdir;

    #[rstest]
    #[case("test", "echo test")]
    #[case("test_prams", "echo test @1 @2")]
    fn test_add_command(#[case] alias: String, #[case] command: String) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path).unwrap();
        match handle_add_command(&mut store, alias, command, None, false) {
            Ok(_) => assert!(true),
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }
    }

    #[rstest]
    #[case("test", "echo test")]
    #[case("test_prams", "echo test @1 @2")]
    fn test_add_command_overwrite(#[case] alias: String, #[case] command: String) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path).unwrap();

        // Initial add
        match handle_add_command(&mut store, alias.clone(), command.clone(), None, false) {
            Ok(_) => assert!(true),
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }

        // Overwrite with modification
        let new_command = format!("{}_modified", command);
        match handle_add_command(&mut store, alias.clone(), new_command.clone(), None, false) {
            Ok(_) => assert!(true),
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }

        // Verify content
        let list = store.list().unwrap();
        let defs = list.get(&alias).unwrap();
        assert_eq!(defs[0].command, new_command);
    }
}
