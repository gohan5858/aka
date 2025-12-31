use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aka"))
}

fn setup() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

#[test]
fn test_explicit_flow() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // 1. List empty
    cmd()
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No aliases found"));

    // 2. Add alias
    cmd()
        .envs(env_vars.clone())
        .args(&["add", "foo", "echo bar"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added alias 'foo' for 'echo bar'"));

    // 3. List with item
    cmd()
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("foo = 'echo bar'"));

    // 4. Remove alias
    cmd()
        .envs(env_vars.clone())
        .args(&["remove", "foo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed alias 'foo' ('echo bar')"));

    // 5. List empty again
    cmd()
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No aliases found"));
}

#[test]
fn test_implicit_flow() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // 1. Implicit List (empty) - no args
    cmd()
        .envs(env_vars.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains("No aliases found"));

    // 2. Implicit Add: aka <alias> <command>
    cmd()
        .envs(env_vars.clone())
        .args(&["g", "git status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added alias 'g' for 'git status'"));

    // 3. Implicit List - no args
    cmd()
        .envs(env_vars.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains("g = 'git status'"));

    // 4. Implicit Remove: aka <alias>
    cmd()
        .envs(env_vars.clone())
        .arg("g")
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed alias 'g' ('git status')"));

    // 5. Implicit List (empty)
    cmd()
        .envs(env_vars.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains("No aliases found"));
}

#[test]
fn test_persistence() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // Run 1: Add
    cmd()
        .envs(env_vars.clone())
        .args(&["add", "ll", "ls -la"])
        .assert()
        .success();

    // Run 2: List (new process, same DB)
    cmd()
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("ll = 'ls -la'"));
}

#[test]
fn test_remove_non_existent() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();

    // Attempt remove non-existent
    cmd()
        .env("aka_DATA_DIR", &root)
        .arg("remove")
        .arg("ghost")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Alias not found: ghost"));
}

#[test]
fn test_init_command() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // Init without dump
    cmd().envs(env_vars.clone()).arg("init").assert().success();

    // Add an alias
    cmd()
        .envs(env_vars.clone())
        .args(&["add", "hello", "echo world"])
        .assert()
        .success();

    // Init with dump should output the alias as shell function/alias
    cmd()
        .envs(env_vars.clone())
        .args(&["init", "--dump"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn test_positional_args_substitution() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // Add alias with @1
    cmd()
        .envs(env_vars.clone())
        .args(&["add", "grep_foo", "grep foo @1"])
        .assert()
        .success();

    // Init --dump should replace @1 with $1 and NOT append "$@"
    cmd()
        .envs(env_vars.clone())
        .args(&["init", "--dump"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("grep foo $1").and(predicate::str::contains("\"$@\"").not()),
        );
}

#[test]
fn test_arg_detection_edge_cases() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // 1. Env var usage ($HOME) - Should SHOULD append "$@" because user didn't use positional args
    cmd()
        .envs(env_vars.clone())
        .args(&["add", "home_echo", "echo $HOME"])
        .assert()
        .success();

    // Check init output
    let _assert = cmd()
        .envs(env_vars.clone())
        .args(&["init", "--dump"])
        .assert()
        .success();

    cmd()
        .envs(env_vars.clone())
        .args(&["add", "myawk", "awk '{print $1}'"])
        .assert()
        .success();

    let output = cmd()
        .envs(env_vars.clone())
        .args(&["init", "--dump"])
        .output()
        .expect("init failed");

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Check home_echo
    if stdout.contains("home_echo() {\n    echo $HOME\n}") {
        println!("BUG REPRODUCED: home_echo missing \"$@\"");
        // This confirms the bug.
    } else if stdout.contains("home_echo() {\n    echo $HOME \"$@\"\n}") {
        println!("home_echo has \"$@\" (Good)");
    } else {
        println!("Unclear output for home_echo: {}", stdout);
    }

    // Check myawk
    // Note: single quotes might be handled weirdly in dump?
    // "awk '{print $1}'"
    if stdout.contains("myawk() {\n    awk '{print $1}'\n}") {
        println!("BUG REPRODUCED: myawk missing \"$@\"");
    }

    // To make this a failing test that passes AFTER fix:
    // Assert that "home_echo" body has "$@".
    // Assert that "myawk" body has "$@".
    // Assert that "explicit_arg" does NOT have duplicate "$@".

    cmd()
        .envs(env_vars.clone())
        .args(&["add", "explicit", "echo $1"])
        .assert()
        .success();

    let output = cmd()
        .envs(env_vars.clone())
        .args(&["init", "--dump"])
        .output()
        .expect("init failed");
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Assertions
    // 1. home_echo should include "$@"
    assert!(
        stdout.contains("echo $HOME \"$@\""),
        "Failed: $HOME usage suppressed positional args. Output:\n{}",
        stdout
    );

    // 2. myawk should include "$@"
    assert!(
        stdout.contains("awk '{print $1}' \"$@\""),
        "Failed: awk usage suppressed positional args. Output:\n{}",
        stdout
    );

    // 3. explicit should NOT include "$@" twice or at end if meant to be handled.
    // Logic: if $1 is present, we do NOT append "$@".
    // So output should be `echo $1`
    assert!(stdout.contains("echo $1"), "Failed: explicit arg not found");
    assert!(
        !stdout.contains("echo $1 \"$@\""),
        "Failed: explicit arg user got extra \"$@\""
    );
}
