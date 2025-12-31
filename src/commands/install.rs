use crate::error::AkaError;
use std::fs::OpenOptions;
use std::io::{Read, Write};

pub fn handle_install_command() -> Result<String, AkaError> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| AkaError::ConfigError("Could not find home directory".to_string()))?;
    let zshrc_path = home_dir.join(".zshrc");

    // Ensure file exists (create if not) or read it
    // We open with read/write/create to ensure existence and check content
    let mut content = String::new();
    {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&zshrc_path)?;
        file.read_to_string(&mut content)?;
    }

    let init_str = r#"eval "$(aka init)""#;

    if content.contains(init_str) {
        return Ok("Already installed in .zshrc".to_string());
    }

    let append_content = format!("\n\n# aka alias manager\n{}\n", init_str);

    let mut file = OpenOptions::new().append(true).open(&zshrc_path)?;
    file.write_all(append_content.as_bytes())?;

    Ok(format!("Installed to {}", zshrc_path.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_install_command() {
        let dir = tempdir().unwrap();
        let home_path = dir.path().to_path_buf();

        unsafe {
            std::env::set_var("HOME", &home_path);
        }

        // 1. Install to empty
        let res = handle_install_command();
        assert!(res.is_ok());
        let msg = res.unwrap();
        assert!(msg.contains("Installed to"));

        let zshrc = home_path.join(".zshrc");
        assert!(zshrc.exists());
        let content = std::fs::read_to_string(&zshrc).unwrap();
        assert!(content.contains("eval \"$(aka init)\""));

        // 2. Install again (idempotency)
        let res = handle_install_command();
        assert!(res.is_ok());
        let msg = res.unwrap();
        assert_eq!(msg, "Already installed in .zshrc");

        let content_again = std::fs::read_to_string(&zshrc).unwrap();
        // Should appear only once (matches count)
        let matches = content_again.matches("eval \"$(aka init)\"").count();
        assert_eq!(matches, 1);
    }
}
