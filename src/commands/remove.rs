use crate::Store;

pub fn handle_remove_command(
    store: &mut Store,
    alias: String,
) -> std::result::Result<String, crate::error::AkaError> {
    match store.remove(&alias)? {
        Some(command) => Ok(format!("Removed alias '{}' ('{}')", alias, command)),
        None => Err(crate::error::AkaError::AliasNotFound(alias)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use tempfile::tempdir;

    #[rstest]
    #[case("test")]
    #[case("test_prams")]
    fn test_remove_command(#[case] alias: String) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path).unwrap();
        // Setup: add alias first
        store.add(alias.clone(), "echo test".to_string()).unwrap();

        match handle_remove_command(&mut store, alias.clone()) {
            Ok(_) => assert!(true),
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }
    }

    #[rstest]
    #[case("test")]
    #[case("test_prams")]
    fn test_remove_command_not_found(#[case] alias: String) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path).unwrap();
        // remove returns Ok even if not found (just explicit message)
        match handle_remove_command(&mut store, alias.clone()) {
            Ok(_) => panic!("Expected Err, got Ok"),
            Err(crate::error::AkaError::AliasNotFound(a)) => assert_eq!(a, alias),
            Err(e) => panic!("Expected AliasNotFound, got {:?}", e),
        }
    }
}
