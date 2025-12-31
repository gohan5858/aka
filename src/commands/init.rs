use crate::store::{AliasScope, Store};

pub fn handle_init_command(
    store: Option<&Store>,
    dump: bool,
) -> std::result::Result<String, crate::error::AkaError> {
    if dump {
        let mut output = String::new();
        let mut managed_aliases = Vec::new();

        output.push_str("if [ -n \"$ZSH_VERSION\" ]; then\n");
        output.push_str("    if [[ -o aliases ]]; then\n");
        output.push_str("        _aka_aliases_was_on=1\n");
        output.push_str("    else\n");
        output.push_str("        _aka_aliases_was_on=0\n");
        output.push_str("    fi\n");
        output.push_str("    unsetopt aliases\n");
        output.push_str("elif [ -n \"$BASH_VERSION\" ]; then\n");
        output.push_str("    _aka_aliases_was_on=$(shopt -q expand_aliases && echo 1 || echo 0)\n");
        output.push_str("    shopt -u expand_aliases\n");
        output.push_str("fi\n");

        // Cleanup previous aliases
        output.push_str("if [ -n \"$AKA_MANAGED_ALIASES\" ]; then\n");
        output.push_str("    for al in $AKA_MANAGED_ALIASES; do unalias $al 2>/dev/null; unset -f $al 2>/dev/null; done\n");
        output.push_str("fi\n");

        if let Some(store) = store {
            for (alias, definitions) in store.list()? {
                managed_aliases.push(alias.clone());

                output.push_str(&format!(
                    "unalias {} 2>/dev/null; unset -f {} 2>/dev/null\n",
                    alias, alias
                ));
                output.push_str(&format!("{}() {{\n", alias));
                output.push_str("    local current_dir=\"$PWD\"\n");

                // Sort definitions: Exact > Recursive (longest first) > Global
                let mut defs = definitions.clone();
                defs.sort_by(|a, b| {
                    match (&a.scope, &b.scope) {
                        (AliasScope::Exact(p1), AliasScope::Exact(p2)) => p2.len().cmp(&p1.len()), // Longest path first
                        (AliasScope::Exact(_), _) => std::cmp::Ordering::Less,
                        (_, AliasScope::Exact(_)) => std::cmp::Ordering::Greater,

                        (AliasScope::Recursive(p1), AliasScope::Recursive(p2)) => {
                            p2.len().cmp(&p1.len())
                        }
                        (AliasScope::Recursive(_), _) => std::cmp::Ordering::Less,
                        (_, AliasScope::Recursive(_)) => std::cmp::Ordering::Greater,

                        (AliasScope::Global, AliasScope::Global) => std::cmp::Ordering::Equal,
                    }
                });

                let mut if_started = false;
                let mut has_global = false;

                for def in defs {
                    let cmd_body = prepare_command_body(&def.command);

                    match &def.scope {
                        AliasScope::Exact(path) => {
                            let op = if if_started { "elif" } else { "if" };
                            output.push_str(&format!(
                                "    {} [[ \"$current_dir\" == \"{}\" ]]; then\n",
                                op, path
                            ));
                            output.push_str(&format!("        {}\n", cmd_body));
                            if_started = true;
                        }
                        AliasScope::Recursive(path) => {
                            let op = if if_started { "elif" } else { "if" };
                            output.push_str(&format!(
                                "    {} [[ \"$current_dir\" == \"{}\"* ]]; then\n",
                                op, path
                            ));
                            output.push_str(&format!("        {}\n", cmd_body));
                            if_started = true;
                        }
                        AliasScope::Global => {
                            if if_started {
                                output.push_str("    else\n");
                            }
                            output.push_str(&format!("        {}\n", cmd_body));
                            has_global = true;
                        }
                    }
                }

                if !has_global {
                    if if_started {
                        output.push_str("    else\n");
                    }
                    output.push_str(&format!("        command {} \"$@\"\n", alias));
                }

                if if_started {
                    output.push_str("    fi\n");
                }

                output.push_str("}\n");
            }
        }

        output.push_str(&format!(
            "export AKA_MANAGED_ALIASES=\"{}\"\n",
            managed_aliases.join(" ")
        ));

        output.push_str("if [ -n \"$ZSH_VERSION\" ]; then\n");
        output.push_str("    if [ \"${_aka_aliases_was_on:-0}\" = \"1\" ]; then\n");
        output.push_str("        setopt aliases\n");
        output.push_str("    fi\n");
        output.push_str("elif [ -n \"$BASH_VERSION\" ]; then\n");
        output.push_str("    if [ \"${_aka_aliases_was_on:-0}\" = \"1\" ]; then\n");
        output.push_str("        shopt -s expand_aliases\n");
        output.push_str("    fi\n");
        output.push_str("fi\n");
        output.push_str("unset _aka_aliases_was_on\n");

        if output.ends_with('\n') {
            output.pop();
        }
        return Ok(output);
    }

    Ok(r#"
# Add this to your ~/.zshrc (Bash support is best-effort)
if [ -n "$ZSH_VERSION" ]; then
    autoload -Uz add-zsh-hook

    _aka_precmd() {
        # 1. Capture last command
        export AKA_LAST_CMD="$(fc -ln -1 | sed 's/^[[:space:]]*//')"

        # 2. Check if we need to reload aliases (if last command was 'aka')
        if [[ "$AKA_LAST_CMD" == aka* ]]; then
             eval "$(command aka init --dump)"
        fi
    }
    add-zsh-hook precmd _aka_precmd

elif [ -n "$BASH_VERSION" ]; then
    # Bash fallback using PROMPT_COMMAND
    _aka_prompt_command() {
        # Capture last command
        export AKA_LAST_CMD="$(history 1 | sed 's/^[[:space:]]*[0-9]*[[:space:]]*//')"

        if [[ "$AKA_LAST_CMD" == aka* ]]; then
             eval "$(command aka init --dump)"
        fi
    }
    PROMPT_COMMAND="_aka_prompt_command;$PROMPT_COMMAND"
fi

eval "$(command aka init --dump)"
"#
    .to_string())
}

fn prepare_command_body(command: &str) -> String {
    let command = replace_placeholders(command);
    if has_positional_args(&command) {
        command
    } else {
        // Append "$@" if no args usage
        format!("{} \"$@\"", command)
    }
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
