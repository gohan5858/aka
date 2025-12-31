use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aka"))
}

fn setup() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

fn write_executable(path: &std::path::Path, content: &str) {
    std::fs::write(path, content).expect("failed to write script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .expect("failed to read metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).expect("failed to set permissions");
    }
}

#[test]
fn test_history_adds_alias_from_zsh_history() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let history_path = temp_dir.path().join(".zsh_history");

    let history = [
        ": 1700000000:0;ls -la",
        ": 1700000001:0;git status",
        ": 1700000002:0;echo hello",
    ]
    .join("\n");
    std::fs::write(&history_path, history).expect("failed to write history");

    let fzf_path = temp_dir.path().join("fzf");
    write_executable(
        &fzf_path,
        "#!/bin/sh\ncat | head -n 1\n",
    );

    cmd()
        .env("NO_COLOR", "1")
        .env("aka_DATA_DIR", data_dir)
        .env("AKA_HISTORY_FILE", &history_path)
        .env("AKA_FZF_BIN", &fzf_path)
        .args(["add"])
        .write_stdin("gs\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Added alias 'gs' for 'echo hello'"));

    cmd()
        .env("NO_COLOR", "1")
        .env("aka_DATA_DIR", data_dir)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("gs = 'echo hello'"));
}

#[test]
fn test_history_fzf_not_found() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let history_path = temp_dir.path().join(".zsh_history");

    std::fs::write(&history_path, ": 1700000000:0;ls -la\n")
        .expect("failed to write history");

    cmd()
        .env("NO_COLOR", "1")
        .env("aka_DATA_DIR", data_dir)
        .env("AKA_HISTORY_FILE", &history_path)
        .env("AKA_FZF_BIN", temp_dir.path().join("missing_fzf"))
        .args(["add"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("fzf not found"));
}
