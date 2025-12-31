use crate::Store;

pub fn handle_init_command(
    store: Option<&Store>,
    dump: bool,
) -> std::result::Result<String, crate::error::AkaError> {
    if dump {
        let mut output = String::new();
        if let Some(store) = store {
            for (alias, command) in store.list()? {
                // Replace @n with $n
                let command = replace_placeholders(&command);

                // Determine if the command already contains argument placeholders
                let has_args = has_positional_args(&command);

                let body = if has_args {
                    command
                } else {
                    format!("{} \"$@\"", command)
                };

                output.push_str(&format!("{}() {{\n    {}\n}}\n", alias, body));
            }
        }

        if output.ends_with('\n') {
            output.pop();
        }
        return Ok(output);
    }

    // Determine the path to the executable
    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_string_lossy();

    Ok(format!(
        r#"
# Add this to your ~/.zshrc or ~/.bashrc
aka() {{
    OUTPUT="$("{}" "$@")"
    echo "$OUTPUT"

    # Check for removal message and unset the alias immediately
    REMOVED_ALIAS=$(echo "$OUTPUT" | grep "^Removed alias " | sed "s/Removed alias '\(.*\)' .*/\1/")
    if [ -n "$REMOVED_ALIAS" ]; then
        unset -f "$REMOVED_ALIAS" 2>/dev/null
        unalias "$REMOVED_ALIAS" 2>/dev/null
    fi

    eval "$("{}" init --dump)"
}}
eval "$("{}" init --dump)""#,
        exe_path_str, exe_path_str, exe_path_str
    ))
}

fn replace_placeholders(command: &str) -> String {
    let mut output = String::with_capacity(command.len());
    let mut chars = command.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '@' {
            if let Some(&next) = chars.peek() {
                if next.is_ascii_digit() {
                    output.push('$');
                    continue;
                }
            }
        }
        output.push(c);
    }
    output
}

fn has_positional_args(command: &str) -> bool {
    let mut chars = command.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }

        if c == '\\' {
            escaped = true;
            continue;
        }

        if in_single_quote {
            if c == '\'' {
                in_single_quote = false;
            }
            continue;
        }

        if in_double_quote {
            if c == '"' {
                in_double_quote = false;
                continue; // Consumed quote
            }
            // Fallthrough to check for $ inside double quotes
        } else {
            // Not in any quote
            if c == '\'' {
                in_single_quote = true;
                continue;
            }
            if c == '"' {
                in_double_quote = true;
                continue;
            }
        }

        // Check for $ (valid in unquoted or double-quoted)
        if c == '$' {
            if let Some(&next) = chars.peek() {
                // Check for $1, $2, ... $9, $0
                if next.is_ascii_digit() {
                    return true;
                }
                // Check for $@, $*, $#
                if matches!(next, '@' | '*' | '#') {
                    return true;
                }
                // Check for ${...}
                if next == '{' {
                    let mut lookahead = chars.clone();
                    lookahead.next();

                    let mut content_type = None;

                    for inner in lookahead {
                        if inner == '}' {
                            if content_type == Some(true) {
                                return true;
                            }
                            break;
                        }
                        if inner.is_ascii_digit() || matches!(inner, '@' | '*' | '#') {
                            if content_type == Some(false) {
                                // Mixed digits and letters? e.g. ${1foo}. Not positional.
                                break;
                            }
                            content_type = Some(true);
                        } else {
                            // Any other char implies named variable
                            content_type = Some(false);
                        }
                    }
                }
            }
        }
    }
    false
}
