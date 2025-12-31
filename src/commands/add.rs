use crate::Store;

pub fn handle_add_command(
    store: &mut Store,
    alias: String,
    command: String,
) -> std::result::Result<String, crate::error::AkaError> {
    store.add(alias.clone(), command.clone())?;
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
        match handle_add_command(&mut store, alias, command) {
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
        match handle_add_command(&mut store, alias.clone(), command.clone()) {
            Ok(_) => assert!(true),
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }

        // Overwrite with modification
        let new_command = format!("{}_modified", command);
        match handle_add_command(&mut store, alias.clone(), new_command.clone()) {
            Ok(_) => assert!(true),
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }

        // Verify content
        let list = store.list().unwrap();
        assert_eq!(list.get(&alias), Some(&new_command));
    }
}
