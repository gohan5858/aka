use crate::store::AliasScope;
use crate::Store;
use std::io::{self, Write};

/// Display a confirmation prompt and read user input.
///
/// Returns true if the user confirms (enters 'y' or 'yes'), false otherwise.
fn confirm_removal(count: usize, scope: Option<&str>) -> std::result::Result<bool, crate::error::AkaError> {
    let scope_text = scope.map_or("all scopes".to_string(), |s| format!("scope '{}'", s));
    print!("Are you sure you want to remove {} alias(es) from {}? (y/N): ", count, scope_text);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    Ok(input == "y" || input == "yes")
}

/// Find a matching scope in the list of definitions.
///
/// For "global", returns AliasScope::Global.
/// For path strings, normalizes the path and searches for an exact or recursive scope match.
fn match_scope_in_definitions(
    definitions: &[crate::store::AliasDefinition],
    scope_str: &str,
) -> std::result::Result<AliasScope, crate::error::AkaError> {
    if scope_str.to_lowercase() == "global" {
        return Ok(AliasScope::Global);
    }

    // Normalize the input path
    let path = std::path::PathBuf::from(scope_str);
    let normalized = path
        .canonicalize()
        .map_err(|e| crate::error::AkaError::InvalidScopePath(e.to_string()))?;
    let normalized_str = normalized
        .to_str()
        .ok_or_else(|| crate::error::AkaError::InvalidScopePath("Invalid UTF-8 in path".to_string()))?;

    // Search for matching scope in definitions
    for def in definitions {
        match &def.scope {
            AliasScope::Exact(p) | AliasScope::Recursive(p) => {
                if p == normalized_str {
                    return Ok(def.scope.clone());
                }
            }
            _ => {}
        }
    }

    Err(crate::error::AkaError::InvalidScopePath(format!(
        "No matching scope found for path: {}",
        scope_str
    )))
}

pub fn handle_remove_command(
    store: &mut Store,
    alias: Option<String>,
    all: bool,
    scope: Option<String>,
    force: bool,
) -> std::result::Result<String, crate::error::AkaError> {
    match (all, alias, scope) {
        // Case 1: Remove all aliases (all scopes)
        (true, None, None) => {
            let count = store.list()?.len();
            if count == 0 {
                return Ok("No aliases to remove".to_string());
            }

            if !force && !confirm_removal(count, None)? {
                return Err(crate::error::AkaError::OperationCancelled);
            }

            let removed_count = store.remove_all()?;
            Ok(format!("Removed {} alias(es)", removed_count))
        }

        // Case 2: Remove all aliases in a specific scope
        (true, None, Some(scope_str)) => {
            // Parse the scope
            let target_scope = if scope_str.to_lowercase() == "global" {
                AliasScope::Global
            } else {
                let path = std::path::PathBuf::from(&scope_str);
                let normalized = path
                    .canonicalize()
                    .map_err(|e| crate::error::AkaError::InvalidScopePath(e.to_string()))?;
                let normalized_str = normalized
                    .to_str()
                    .ok_or_else(|| {
                        crate::error::AkaError::InvalidScopePath("Invalid UTF-8 in path".to_string())
                    })?
                    .to_string();

                // Need to determine if it's Exact or Recursive by checking existing definitions
                // For now, we'll try both and use whichever matches
                let all_aliases = store.list()?;
                let mut found_scope: Option<AliasScope> = None;

                for defs in all_aliases.values() {
                    for def in defs {
                        match &def.scope {
                            AliasScope::Exact(p) | AliasScope::Recursive(p) => {
                                if p == &normalized_str {
                                    found_scope = Some(def.scope.clone());
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    if found_scope.is_some() {
                        break;
                    }
                }

                found_scope.unwrap_or(AliasScope::Exact(normalized_str))
            };

            let removed = store.remove_all_in_scope(&target_scope)?;
            let count = removed.len();

            if count == 0 {
                return Ok(format!("No aliases found in scope '{}'", scope_str));
            }

            if !force && !confirm_removal(count, Some(&scope_str))? {
                return Err(crate::error::AkaError::OperationCancelled);
            }

            // Re-execute since we already consumed the result for counting
            store.remove_all_in_scope(&target_scope)?;
            Ok(format!(
                "Removed {} alias(es) from scope '{}'",
                count, scope_str
            ))
        }

        // Case 3: Remove a specific alias (all scopes)
        (false, Some(alias_name), None) => match store.remove(&alias_name)? {
            Some(defs) => {
                let count = defs.len();
                Ok(format!(
                    "Removed alias '{}' ({} definitions)",
                    alias_name, count
                ))
            }
            None => Err(crate::error::AkaError::AliasNotFound(alias_name)),
        },

        // Case 4: Remove a specific alias from a specific scope
        (false, Some(alias_name), Some(scope_str)) => {
            // Get the alias definitions first
            let all_aliases = store.list()?;
            let definitions = all_aliases
                .get(&alias_name)
                .ok_or_else(|| crate::error::AkaError::AliasNotFound(alias_name.clone()))?;

            // Match the scope
            let target_scope = match_scope_in_definitions(definitions, &scope_str)?;

            // Remove the specific scope
            match store.remove_scope_from_alias(&alias_name, &target_scope)? {
                Some(_) => {
                    let remaining = store
                        .list()?
                        .get(&alias_name)
                        .map(|defs| defs.len())
                        .unwrap_or(0);

                    if remaining == 0 {
                        Ok(format!(
                            "Removed alias '{}' from scope '{}' (no definitions remaining)",
                            alias_name, scope_str
                        ))
                    } else {
                        Ok(format!(
                            "Removed alias '{}' from scope '{}' ({} definitions remaining)",
                            alias_name, scope_str, remaining
                        ))
                    }
                }
                None => Err(crate::error::AkaError::ScopeNotFoundInAlias(
                    alias_name,
                    scope_str,
                )),
            }
        }

        // Invalid combinations (should not happen due to clap validation)
        _ => Err(crate::error::AkaError::ConfigError(
            "Invalid argument combination".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::AliasScope;
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
        store
            .add(alias.clone(), "echo test".to_string(), AliasScope::Global)
            .unwrap();

        match handle_remove_command(&mut store, Some(alias.clone()), false, None, false) {
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
        match handle_remove_command(&mut store, Some(alias.clone()), false, None, false) {
            Ok(_) => panic!("Expected Err, got Ok"),
            Err(crate::error::AkaError::AliasNotFound(a)) => assert_eq!(a, alias),
            Err(e) => panic!("Expected AliasNotFound, got {:?}", e),
        }
    }

    #[test]
    fn test_remove_all_with_force() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aka.redb");
        let mut store = Store::load(&path).unwrap();

        // Add some aliases
        store
            .add("foo".to_string(), "echo foo".to_string(), AliasScope::Global)
            .unwrap();
        store
            .add("bar".to_string(), "echo bar".to_string(), AliasScope::Global)
            .unwrap();

        // Remove all with force flag
        let result = handle_remove_command(&mut store, None, true, None, true);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Removed 2 alias(es)"));

        // Verify all removed
        assert!(store.list().unwrap().is_empty());
    }
}
