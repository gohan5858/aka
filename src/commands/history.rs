use crate::commands::add::handle_add_command;
use crate::error::AkaError;
use crate::Store;
use std::collections::HashSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DEFAULT_HISTORY_LIMIT: usize = 200;

/// 履歴から fzf でコマンドを選び、エイリアスとして登録する。
pub fn handle_history_command(
    store: &mut Store,
    alias: Option<String>,
    scope: Option<String>,
    recursive: bool,
    limit: usize,
) -> std::result::Result<String, AkaError> {
    let history_path = resolve_history_path()?;
    let entries = read_history_entries(&history_path, limit)?;
    if entries.is_empty() {
        return Ok("No history entries found".to_string());
    }

    let selected = match select_with_fzf(&entries)? {
        Some(value) => value,
        None => return Err(AkaError::OperationCancelled),
    };

    let alias_name = match alias {
        Some(value) => value,
        None => prompt_alias_name(&selected)?,
    };

    handle_add_command(store, alias_name, selected, scope, recursive)
}

/// 履歴ファイルのパスを解決する。
fn resolve_history_path() -> std::result::Result<PathBuf, AkaError> {
    if let Ok(path) = std::env::var("AKA_HISTORY_FILE") {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    if let Ok(path) = std::env::var("HISTFILE") {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    let home_dir = dirs::home_dir()
        .ok_or_else(|| AkaError::ConfigError("Could not find home directory".to_string()))?;

    let zsh_history = home_dir.join(".zsh_history");
    if zsh_history.exists() {
        return Ok(zsh_history);
    }

    let bash_history = home_dir.join(".bash_history");
    if bash_history.exists() {
        return Ok(bash_history);
    }

    Err(AkaError::ConfigError(
        "History file not found. Set HISTFILE or AKA_HISTORY_FILE".to_string(),
    ))
}

/// 履歴ファイルから最新のコマンドを抽出する。
fn read_history_entries(path: &Path, limit: usize) -> std::result::Result<Vec<String>, AkaError> {
    let bytes = std::fs::read(path)?;
    let content = String::from_utf8_lossy(&bytes);
    let max_entries = if limit == 0 {
        DEFAULT_HISTORY_LIMIT
    } else {
        limit
    };

    let mut entries = Vec::new();
    let mut seen = HashSet::new();

    for line in content.lines().rev() {
        if let Some(cmd) = parse_history_line(line) {
            let trimmed = cmd.trim();
            if trimmed.is_empty() {
                continue;
            }
            if seen.insert(trimmed.to_string()) {
                entries.push(trimmed.to_string());
                if entries.len() >= max_entries {
                    break;
                }
            }
        }
    }

    Ok(entries)
}

/// 1行の履歴からコマンド部分を抽出する。
fn parse_history_line(line: &str) -> Option<String> {
    if let Some(rest) = line.strip_prefix(": ") {
        if let Some((_, command)) = rest.split_once(';') {
            return Some(command.to_string());
        }
    }

    if let Some(rest) = line.strip_prefix('#') {
        if rest.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
    }

    Some(line.to_string())
}

/// fzf を使って候補から選択する。
fn select_with_fzf(entries: &[String]) -> std::result::Result<Option<String>, AkaError> {
    if entries.is_empty() {
        return Ok(None);
    }

    let fzf_bin = std::env::var("AKA_FZF_BIN").unwrap_or_else(|_| "fzf".to_string());
    let mut command = Command::new(&fzf_bin);
    command
        .arg("--exit-0")
        .arg("--reverse")
        .arg("--height=40%")
        .arg("--prompt=aka> ")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            AkaError::ConfigError(format!("fzf not found: {}", fzf_bin))
        } else {
            AkaError::IoError(e)
        }
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        let input = entries.join("\n");
        stdin.write_all(input.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(AkaError::OperationCancelled);
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(selected))
    }
}

/// エイリアス名を標準入力から取得する。
fn prompt_alias_name(command: &str) -> std::result::Result<String, AkaError> {
    let mut alias = String::new();
    loop {
        print!("Alias name (command: {}): ", command);
        io::stdout().flush()?;
        alias.clear();
        io::stdin().read_line(&mut alias)?;
        let trimmed = alias.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_history_line_zsh_extended() {
        let line = ": 1700000000:0;git status";
        let parsed = parse_history_line(line);
        assert_eq!(parsed, Some("git status".to_string()));
    }

    #[test]
    fn test_parse_history_line_bash_timestamp() {
        let line = "#1700000000";
        let parsed = parse_history_line(line);
        assert_eq!(parsed, None);
    }

    #[test]
    fn test_read_history_entries_with_invalid_utf8() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("history");

        let bytes = b": 1700000000:0;git status\n: 1700000001:0;echo \xFF\nls -la\n";
        std::fs::write(&path, bytes).unwrap();

        let entries = read_history_entries(&path, 10).unwrap();
        assert!(entries.iter().any(|entry| entry == "ls -la"));
        assert!(entries.iter().any(|entry| entry.starts_with("echo ")));
    }
}
