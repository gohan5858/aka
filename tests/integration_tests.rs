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
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No aliases found"));

    // 2. Add alias
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "foo", "echo bar"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added alias 'foo' for 'echo bar'"));

    // 3. List with item
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("foo = 'echo bar'"));

    // 4. Remove alias
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["remove", "foo"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Removed alias 'foo' (1 definitions)",
        ));

    // 5. List empty again
    cmd()
        .env("NO_COLOR", "1")
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
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains("No aliases found"));

    // 2. Implicit Add: aka <alias> <command>
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["g", "git status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added alias 'g' for 'git status'"));

    // 3. Implicit List - no args
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .assert()
        .success()
        .stdout(predicate::str::contains("g = 'git status'"));

    // 4. Implicit Remove: aka <alias>
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .arg("g")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Removed alias 'g' (1 definitions)",
        ));

    // 5. Implicit List (empty)
    cmd()
        .env("NO_COLOR", "1")
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
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "ll", "ls -la"])
        .assert()
        .success();

    // Run 2: List (new process, same DB)
    cmd()
        .env("NO_COLOR", "1")
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
        .env("NO_COLOR", "1")
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
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .arg("init")
        .assert()
        .success();

    // Add an alias
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "hello", "echo world"])
        .assert()
        .success();

    // Init with dump should output the alias as shell function/alias
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["init", "--dump"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("hello")
                .and(predicate::str::contains("[[ -o aliases ]]")),
        );
}

#[test]
fn test_positional_args_substitution() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // Add alias with @1
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "grep_foo", "grep foo @1"])
        .assert()
        .success();

    // Init --dump should replace @1 with $1 and NOT append "$@"
    cmd()
        .env("NO_COLOR", "1")
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
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "home_echo", "echo $HOME"])
        .assert()
        .success();

    // Check init output
    let _assert = cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["init", "--dump"])
        .assert()
        .success();

    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "myawk", "awk '{print $1}'"])
        .assert()
        .success();

    let output = cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["init", "--dump"])
        .output()
        .expect("init failed");

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Check home_echo
    if stdout.contains("home_echo() {\n    echo $HOME\n}") {
        println!("BUG REPRODUCED: home_echo missing \"$@\"");
    } else if stdout.contains("alias home_echo='echo $HOME'") {
        println!("home_echo is alias (Good)");
    } else if stdout.contains("home_echo() {\n    echo $HOME \"$@\"\n}") {
        println!("home_echo has \"$@\" (Good)");
    } else {
        println!("Unclear output for home_echo: {}", stdout);
    }

    // Check myawk
    // "awk '{print $1}'"
    if stdout.contains("myawk() {\n    awk '{print $1}'\n}") {
        println!("BUG REPRODUCED: myawk missing \"$@\"");
    } else if stdout.contains("alias myawk='awk '\\''{print $1}'\\'''") {
        println!("myawk is alias (Good)");
    }

    // To make this a failing test that passes AFTER fix:
    // Assert that "home_echo" body has "$@".
    // Assert that "myawk" body has "$@".
    // Assert that "explicit_arg" does NOT have duplicate "$@".

    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "explicit", "echo $1"])
        .assert()
        .success();

    let output = cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["init", "--dump"])
        .output()
        .expect("init failed");
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Assertions
    // 1. home_echo should include "$@" OR be an alias
    let home_echo_ok =
        stdout.contains("echo $HOME \"$@\"") || stdout.contains("alias home_echo='echo $HOME'");
    assert!(
        home_echo_ok,
        "Failed: home_echo not correct. Output:\n{}",
        stdout
    );

    // 2. myawk should include "$@" OR be an alias
    let myawk_ok = stdout.contains("awk '{print $1}' \"$@\"")
        || stdout.contains("alias myawk='awk '\\''{print $1}'\\'''");
    assert!(myawk_ok, "Failed: myawk not correct. Output:\n{}", stdout);

    // 3. explicit should NOT include "$@" twice or at end if meant to be handled.
    // Logic: if $1 is present, we do NOT append "$@".
    // So output should be `echo $1`
    assert!(stdout.contains("echo $1"), "Failed: explicit arg not found");
    assert!(
        !stdout.contains("echo $1 \"$@\""),
        "Failed: explicit arg user got extra \"$@\""
    );
}

#[test]
fn test_aliases() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // 1. Add alias using full command
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "ll", "ls -la"])
        .assert()
        .success();

    // 2. List using 'ls' alias
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("ll = 'ls -la'"));

    // 3. Remove using 'rm' alias
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["rm", "ll"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed alias 'll'"));

    // 4. Verify removal with 'ls'
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("No aliases found"));
}

#[test]
fn test_scoped_aliases() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // 1. Add global alias
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "foo", "echo global"])
        .assert()
        .success();

    // 2. Add scoped alias (recursive)
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&[
            "add",
            "foo",
            "echo scoped",
            "--scope",
            "/tmp",
            "--recursive",
        ])
        .assert()
        .success();

    // 3. List should show both (use --all to see scoped one from outside)
    // On macOS /tmp is a symlink to /private/tmp, so we need to be flexible or check canonical path
    let tmp_path = std::fs::canonicalize("/tmp").unwrap();
    let tmp_str = tmp_path.to_string_lossy();

    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["list", "--all"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("foo = 'echo global' (Global)").and(predicate::str::contains(
                format!("foo = 'echo scoped' (Recursive: {})", tmp_str),
            )),
        );

    // 4. Init dump should show conditional logic
    let output = cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["init", "--dump"])
        .output()
        .expect("init failed");

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("foo() {"));
    assert!(stdout.contains(&format!(
        "if [[ \"$current_dir\" == \"{}\"* ]]; then",
        tmp_str
    )));
    assert!(stdout.contains("echo scoped \"$@\""));
    assert!(stdout.contains("else"));
    assert!(stdout.contains("echo global \"$@\""));
}

#[test]
fn test_scoped_alias_implicit_dir() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // Add scoped alias with implicit dir (no value for --scope)
    // clap requires we pass arguments as if they were command line
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "here", "echo here", "--scope"])
        .assert()
        .success();

    let cwd = std::env::current_dir().unwrap();
    let cwd_str = cwd.to_string_lossy();

    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "here = 'echo here' (Exact: {})",
            cwd_str
        )));
}

#[test]
fn test_list_filtering() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // 1. Add scopes: Global, Global matching CWD (simulated via Exact), and Other Exact
    let cwd = std::env::current_dir().unwrap();
    let cwd_str = cwd.to_string_lossy();

    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "global_alias", "echo global"])
        .assert()
        .success();

    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["add", "current_exact", "echo current", "--scope", "."])
        .assert()
        .success();

    // Use a path that is definitely not CWD
    let other_dir = std::env::temp_dir();
    let other_dir_str = other_dir.to_string_lossy();
    cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&[
            "add",
            "other_exact",
            "echo other",
            "--scope",
            &other_dir_str,
        ])
        .assert()
        .success();

    // 2. List default (should show global and current, but NOT other)
    let assert = cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("global_alias"), "Missing global alias");
    assert!(
        stdout.contains("current_exact"),
        "Missing current scope alias"
    );
    assert!(
        !stdout.contains("other_exact"),
        "Should filter out other scope alias"
    );

    // 3. List --all (should show everything)
    let assert = cmd()
        .env("NO_COLOR", "1")
        .envs(env_vars.clone())
        .args(&["list", "--all"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("global_alias"));
    assert!(stdout.contains("current_exact"));
    assert!(
        stdout.contains("other_exact"),
        "Missing other exact with --all"
    );
}

#[test]
fn test_remove_all_flow() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // Add multiple aliases
    cmd()
        .envs(env_vars.clone())
        .args(&["add", "foo", "echo foo"])
        .assert()
        .success();

    cmd()
        .envs(env_vars.clone())
        .args(&["add", "bar", "echo bar"])
        .assert()
        .success();

    cmd()
        .envs(env_vars.clone())
        .args(&["add", "baz", "echo baz"])
        .assert()
        .success();

    // Verify aliases exist
    cmd()
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("foo")
                .and(predicate::str::contains("bar"))
                .and(predicate::str::contains("baz")),
        );

    // Remove all with --force
    cmd()
        .envs(env_vars.clone())
        .args(&["remove", "--all", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed 3 alias(es)"));

    // Verify all removed
    cmd()
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No aliases found"));
}

#[test]
fn test_remove_all_with_scope_flow() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // Add global aliases
    cmd()
        .envs(env_vars.clone())
        .args(&["add", "foo", "echo foo global"])
        .assert()
        .success();

    cmd()
        .envs(env_vars.clone())
        .args(&["add", "bar", "echo bar global"])
        .assert()
        .success();

    // Add scoped aliases
    let tmp_path = std::fs::canonicalize("/tmp").unwrap();
    let tmp_str = tmp_path.to_string_lossy();

    cmd()
        .envs(env_vars.clone())
        .args(&["add", "baz", "echo baz scoped", "--scope", "/tmp"])
        .assert()
        .success();

    cmd()
        .envs(env_vars.clone())
        .args(&["add", "qux", "echo qux scoped", "--scope", "/tmp"])
        .assert()
        .success();

    // Remove all global with --force
    cmd()
        .envs(env_vars.clone())
        .args(&["remove", "--all", "--scope", "global", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed 2 alias(es) from scope 'global'"));

    // Verify only scoped aliases remain
    cmd()
        .envs(env_vars.clone())
        .args(&["list", "--all"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("baz")
                .and(predicate::str::contains("qux"))
                .and(predicate::str::contains("foo").not())
                .and(predicate::str::contains("bar").not()),
        );
}

#[test]
fn test_remove_partial_scope_flow() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    let tmp_path = std::fs::canonicalize("/tmp").unwrap();
    let tmp_str = tmp_path.to_string_lossy();

    // Add alias with multiple scopes
    cmd()
        .envs(env_vars.clone())
        .args(&["add", "foo", "echo foo global"])
        .assert()
        .success();

    cmd()
        .envs(env_vars.clone())
        .args(&["add", "foo", "echo foo scoped", "--scope", "/tmp"])
        .assert()
        .success();

    // Remove only scoped definition
    cmd()
        .envs(env_vars.clone())
        .args(&["remove", "foo", "--scope", &tmp_str.to_string()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed alias 'foo' from scope"));

    // Verify global definition still exists
    cmd()
        .envs(env_vars.clone())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("foo = 'echo foo global' (Global)"));
}

#[test]
fn test_remove_scope_not_found() {
    let temp_dir = setup();
    let data_dir = temp_dir.path().to_str().unwrap();
    let env_vars = vec![("aka_DATA_DIR", data_dir)];

    // Add global alias only
    cmd()
        .envs(env_vars.clone())
        .args(&["add", "foo", "echo foo"])
        .assert()
        .success();

    // Try to remove non-existent scope
    cmd()
        .envs(env_vars.clone())
        .args(&["remove", "foo", "--scope", "/nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid scope path"));
}
