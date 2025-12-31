use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::{collections::HashMap, path::Path};

const TABLE: TableDefinition<&str, &str> = TableDefinition::new("aliases");

/// The storage for aliases
pub struct Store {
    db: Database,
}

impl Store {
    pub fn new() -> std::result::Result<Self, crate::error::AkaError> {
        let data_dir = if let Ok(dir) = std::env::var("aka_DATA_DIR") {
            std::path::PathBuf::from(dir)
        } else {
            dirs::data_dir().ok_or_else(|| {
                crate::error::AkaError::ConfigError("Data dir not found".to_string())
            })?
        };
        let base_path = data_dir.join("aka");
        let path = base_path.join("aka.redb");
        Self::load(&path)
    }

    pub fn load(path: &Path) -> std::result::Result<Self, crate::error::AkaError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let db = Database::create(path).map_err(crate::error::AkaError::from)?;
        Ok(Store { db })
    }

    pub fn add(
        &mut self,
        alias: String,
        command: String,
    ) -> std::result::Result<String, crate::error::AkaError> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;
            table.insert(alias.as_str(), command.as_str())?;
        }
        write_txn.commit()?;
        Ok(command)
    }

    pub fn remove(
        &mut self,
        alias: &str,
    ) -> std::result::Result<Option<String>, crate::error::AkaError> {
        let write_txn = self.db.begin_write()?;
        let res = {
            let mut table = write_txn.open_table(TABLE)?;
            table.remove(alias)?.map(|v| v.value().to_string())
        };
        write_txn.commit()?;
        Ok(res)
    }

    pub fn list(&self) -> std::result::Result<HashMap<String, String>, crate::error::AkaError> {
        let read_txn = self.db.begin_read()?;
        let mut map = HashMap::new();
        match read_txn.open_table(TABLE) {
            Ok(table) => {
                for item in table.iter()? {
                    let (k, v) = item?;
                    map.insert(k.value().to_string(), v.value().to_string());
                }
            }
            Err(redb::TableError::TableDoesNotExist(_)) => {
                // Table doesn't exist yet, return empty map
            }
            Err(e) => return Err(e.into()),
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[allow(unused_imports)]
    use tempfile::tempdir;

    #[test]
    fn test_store_ops() -> std::result::Result<(), crate::error::AkaError> {
        let dir = tempdir()?;
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path)?;

        // Test add
        let added = store.add("foo".to_string(), "echo foo".to_string())?;
        assert_eq!(added, "echo foo");

        // Test list
        let aliases = store.list()?;
        assert_eq!(aliases.get("foo").map(|s| s.as_str()), Some("echo foo"));

        // Test remove
        let removed = store.remove("foo")?;
        assert_eq!(removed.as_deref(), Some("echo foo"));

        let aliases = store.list()?;
        assert!(aliases.is_empty());

        // Test remove non-existent
        let removed = store.remove("bar")?;
        assert_eq!(removed, None);

        Ok(())
    }
}
