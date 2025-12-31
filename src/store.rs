use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

const TABLE: TableDefinition<&str, &str> = TableDefinition::new("aliases");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AliasScope {
    Global,
    Recursive(String),
    Exact(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AliasDefinition {
    pub command: String,
    pub scope: AliasScope,
}

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
        scope: AliasScope,
    ) -> std::result::Result<(), crate::error::AkaError> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TABLE)?;

            // Read existing definitions
            let mut definitions = if let Some(value) = table.get(alias.as_str())? {
                let s = value.value();
                match serde_json::from_str::<Vec<AliasDefinition>>(s) {
                    Ok(defs) => defs,
                    Err(_) => {
                        // Legacy: treat as single global alias
                        vec![AliasDefinition {
                            command: s.to_string(),
                            scope: AliasScope::Global,
                        }]
                    }
                }
            } else {
                Vec::new()
            };

            // Remove existing definition for same scope if exists (overwrite)
            definitions.retain(|d| d.scope != scope);

            // Add new definition
            definitions.push(AliasDefinition { command, scope });

            let json = serde_json::to_string(&definitions)
                .map_err(|e| crate::error::AkaError::ConfigError(e.to_string()))?;
            table.insert(alias.as_str(), json.as_str())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn remove(
        &mut self,
        alias: &str,
    ) -> std::result::Result<Option<Vec<AliasDefinition>>, crate::error::AkaError> {
        let write_txn = self.db.begin_write()?;
        let res = {
            let mut table = write_txn.open_table(TABLE)?;
            if let Some(value) = table.remove(alias)? {
                let s = value.value();
                match serde_json::from_str::<Vec<AliasDefinition>>(s) {
                    Ok(defs) => Some(defs),
                    Err(_) => Some(vec![AliasDefinition {
                        command: s.to_string(),
                        scope: AliasScope::Global,
                    }]),
                }
            } else {
                None
            }
        };
        write_txn.commit()?;
        Ok(res)
    }

    /// Remove all aliases from the store.
    ///
    /// Returns the number of aliases that were removed.
    pub fn remove_all(&mut self) -> std::result::Result<usize, crate::error::AkaError> {
        let write_txn = self.db.begin_write()?;
        let count = {
            let mut table = write_txn.open_table(TABLE)?;
            let count = table.len()?;

            // Collect all keys first to avoid iterator invalidation
            let keys: Vec<String> = table
                .iter()?
                .map(|item| item.map(|(k, _)| k.value().to_string()))
                .collect::<std::result::Result<Vec<_>, _>>()?;

            // Remove all entries
            for key in keys {
                table.remove(key.as_str())?;
            }

            count as usize
        };
        write_txn.commit()?;
        Ok(count)
    }

    /// Remove a specific scope from an alias.
    ///
    /// If the alias has no remaining definitions after removal, the alias key is removed entirely.
    /// Returns the removed definition, or None if the alias or scope was not found.
    pub fn remove_scope_from_alias(
        &mut self,
        alias: &str,
        scope: &AliasScope,
    ) -> std::result::Result<Option<AliasDefinition>, crate::error::AkaError> {
        let write_txn = self.db.begin_write()?;
        let removed = {
            let mut table = write_txn.open_table(TABLE)?;

            // Read current definitions
            let definitions = if let Some(value) = table.get(alias)? {
                let s = value.value().to_string();
                match serde_json::from_str::<Vec<AliasDefinition>>(&s) {
                    Ok(defs) => Some(defs),
                    Err(_) => Some(vec![AliasDefinition {
                        command: s,
                        scope: AliasScope::Global,
                    }]),
                }
            } else {
                None
            };

            if let Some(mut defs) = definitions {
                // Find and remove the matching scope
                let initial_len = defs.len();
                let mut removed_def = None;
                defs.retain(|d| {
                    if &d.scope == scope {
                        removed_def = Some(d.clone());
                        false
                    } else {
                        true
                    }
                });

                // If nothing was removed, return None
                if defs.len() == initial_len {
                    None
                } else {
                    // If no definitions remain, remove the key entirely
                    if defs.is_empty() {
                        table.remove(alias)?;
                    } else {
                        // Otherwise, update with remaining definitions
                        let json = serde_json::to_string(&defs)
                            .map_err(|e| crate::error::AkaError::ConfigError(e.to_string()))?;
                        table.insert(alias, json.as_str())?;
                    }
                    removed_def
                }
            } else {
                None
            }
        };
        write_txn.commit()?;
        Ok(removed)
    }

    /// Remove all definitions with the specified scope from all aliases.
    ///
    /// Returns a map of alias names to the definitions that were removed.
    pub fn remove_all_in_scope(
        &mut self,
        scope: &AliasScope,
    ) -> std::result::Result<HashMap<String, Vec<AliasDefinition>>, crate::error::AkaError> {
        let write_txn = self.db.begin_write()?;
        let removed = {
            let mut table = write_txn.open_table(TABLE)?;
            let mut removed_map: HashMap<String, Vec<AliasDefinition>> = HashMap::new();

            // Read all aliases first
            let all_aliases: Vec<(String, String)> = table
                .iter()?
                .map(|item| {
                    let (k, v) = item?;
                    Ok((k.value().to_string(), v.value().to_string()))
                })
                .collect::<std::result::Result<Vec<_>, redb::Error>>()?;

            // Process each alias
            for (alias, value_str) in all_aliases {
                let mut definitions = match serde_json::from_str::<Vec<AliasDefinition>>(&value_str)
                {
                    Ok(defs) => defs,
                    Err(_) => vec![AliasDefinition {
                        command: value_str,
                        scope: AliasScope::Global,
                    }],
                };

                // Filter out definitions with matching scope
                let mut removed_defs = Vec::new();
                definitions.retain(|d| {
                    if &d.scope == scope {
                        removed_defs.push(d.clone());
                        false
                    } else {
                        true
                    }
                });

                // If any were removed, update or delete the alias
                if !removed_defs.is_empty() {
                    removed_map.insert(alias.clone(), removed_defs);

                    if definitions.is_empty() {
                        table.remove(alias.as_str())?;
                    } else {
                        let json = serde_json::to_string(&definitions)
                            .map_err(|e| crate::error::AkaError::ConfigError(e.to_string()))?;
                        table.insert(alias.as_str(), json.as_str())?;
                    }
                }
            }

            removed_map
        };
        write_txn.commit()?;
        Ok(removed)
    }

    pub fn list(
        &self,
    ) -> std::result::Result<HashMap<String, Vec<AliasDefinition>>, crate::error::AkaError> {
        let read_txn = self.db.begin_read()?;
        let mut map = HashMap::new();
        match read_txn.open_table(TABLE) {
            Ok(table) => {
                for item in table.iter()? {
                    let (k, v) = item?;
                    let s = v.value();
                    let defs = match serde_json::from_str::<Vec<AliasDefinition>>(s) {
                        Ok(d) => d,
                        Err(_) => vec![AliasDefinition {
                            command: s.to_string(),
                            scope: AliasScope::Global,
                        }],
                    };
                    map.insert(k.value().to_string(), defs);
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

        // Test add global
        store.add(
            "foo".to_string(),
            "echo foo".to_string(),
            AliasScope::Global,
        )?;

        // Test list
        let aliases = store.list()?;
        let defs = aliases.get("foo").unwrap();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].command, "echo foo");
        assert_eq!(defs[0].scope, AliasScope::Global);

        // Test add scoped (append)
        store.add(
            "foo".to_string(),
            "echo bar".to_string(),
            AliasScope::Exact("/tmp".to_string()),
        )?;
        let aliases = store.list()?;
        let defs = aliases.get("foo").unwrap();
        assert_eq!(defs.len(), 2);

        // Test remove
        let removed = store.remove("foo")?;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().len(), 2);

        let aliases = store.list()?;
        assert!(aliases.is_empty());

        Ok(())
    }

    #[test]
    fn test_remove_all() -> std::result::Result<(), crate::error::AkaError> {
        let dir = tempdir()?;
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path)?;

        // Add multiple aliases
        store.add("foo".to_string(), "echo foo".to_string(), AliasScope::Global)?;
        store.add("bar".to_string(), "echo bar".to_string(), AliasScope::Global)?;
        store.add(
            "baz".to_string(),
            "echo baz".to_string(),
            AliasScope::Exact("/tmp".to_string()),
        )?;

        // Verify they exist
        let aliases = store.list()?;
        assert_eq!(aliases.len(), 3);

        // Remove all
        let count = store.remove_all()?;
        assert_eq!(count, 3);

        // Verify all are gone
        let aliases = store.list()?;
        assert!(aliases.is_empty());

        Ok(())
    }

    #[test]
    fn test_remove_scope_from_alias_partial() -> std::result::Result<(), crate::error::AkaError> {
        let dir = tempdir()?;
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path)?;

        // Add alias with multiple scopes
        store.add("foo".to_string(), "echo foo".to_string(), AliasScope::Global)?;
        store.add(
            "foo".to_string(),
            "echo bar".to_string(),
            AliasScope::Exact("/tmp".to_string()),
        )?;

        // Remove only the scoped definition
        let removed =
            store.remove_scope_from_alias("foo", &AliasScope::Exact("/tmp".to_string()))?;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().command, "echo bar");

        // Verify global definition still exists
        let aliases = store.list()?;
        let defs = aliases.get("foo").unwrap();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].scope, AliasScope::Global);

        Ok(())
    }

    #[test]
    fn test_remove_scope_from_alias_complete() -> std::result::Result<(), crate::error::AkaError>
    {
        let dir = tempdir()?;
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path)?;

        // Add alias with single scope
        store.add("foo".to_string(), "echo foo".to_string(), AliasScope::Global)?;

        // Remove the only definition
        let removed = store.remove_scope_from_alias("foo", &AliasScope::Global)?;
        assert!(removed.is_some());

        // Verify alias is completely removed
        let aliases = store.list()?;
        assert!(aliases.get("foo").is_none());

        Ok(())
    }

    #[test]
    fn test_remove_scope_from_alias_not_found() -> std::result::Result<(), crate::error::AkaError>
    {
        let dir = tempdir()?;
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path)?;

        // Add alias with global scope only
        store.add("foo".to_string(), "echo foo".to_string(), AliasScope::Global)?;

        // Try to remove non-existent scope
        let removed =
            store.remove_scope_from_alias("foo", &AliasScope::Exact("/tmp".to_string()))?;
        assert!(removed.is_none());

        // Verify global definition still exists
        let aliases = store.list()?;
        let defs = aliases.get("foo").unwrap();
        assert_eq!(defs.len(), 1);

        Ok(())
    }

    #[test]
    fn test_remove_all_in_scope_global() -> std::result::Result<(), crate::error::AkaError> {
        let dir = tempdir()?;
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path)?;

        // Add multiple aliases with different scopes
        store.add("foo".to_string(), "echo foo".to_string(), AliasScope::Global)?;
        store.add("bar".to_string(), "echo bar".to_string(), AliasScope::Global)?;
        store.add(
            "baz".to_string(),
            "echo baz".to_string(),
            AliasScope::Exact("/tmp".to_string()),
        )?;
        store.add(
            "qux".to_string(),
            "echo qux global".to_string(),
            AliasScope::Global,
        )?;
        store.add(
            "qux".to_string(),
            "echo qux scoped".to_string(),
            AliasScope::Exact("/tmp".to_string()),
        )?;

        // Remove all global definitions
        let removed = store.remove_all_in_scope(&AliasScope::Global)?;

        // Verify correct aliases were removed
        assert_eq!(removed.len(), 3); // foo, bar, qux
        assert!(removed.contains_key("foo"));
        assert!(removed.contains_key("bar"));
        assert!(removed.contains_key("qux"));
        assert_eq!(removed.get("foo").unwrap().len(), 1);
        assert_eq!(removed.get("qux").unwrap().len(), 1);

        // Verify remaining aliases
        let aliases = store.list()?;
        assert_eq!(aliases.len(), 2); // baz and qux
        assert!(aliases.contains_key("baz"));
        assert!(aliases.contains_key("qux"));

        // qux should still have the scoped definition
        let qux_defs = aliases.get("qux").unwrap();
        assert_eq!(qux_defs.len(), 1);
        assert_eq!(qux_defs[0].scope, AliasScope::Exact("/tmp".to_string()));

        Ok(())
    }
}
