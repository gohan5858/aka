#[cfg(test)]
mod tests {
    use aka::Store;
    use aka::error::AkaError;
    use tempfile::tempdir;

    #[test]
    fn test_error_pattern_matching() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aka.redb");
        std::fs::create_dir(&path).unwrap();

        let result = Store::load(&path);

        match result {
            Err(AkaError::DatabaseError(_)) => assert!(true), // redb should fail to open directory as valid file
            Err(e) => panic!("Expected DatabaseError, got {:?}", e),
            Ok(_) => panic!("Should have failed"),
        }
    }
}
