use std::env;

use crate::Result;
use crate::Store;
use crate::store::AliasScope;
use owo_colors::{OwoColorize, Stream};

/// ANSIエスケープコード付き文字列の表示幅を計算
fn visual_width(s: &str) -> usize {
    let mut width = 0;
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // ESCシーケンス開始
            for next_ch in chars.by_ref() {
                if next_ch == 'm' {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

/// 指定幅までスペースパディング
fn pad_to_width(s: &str, target_width: usize) -> String {
    let current_width = visual_width(s);
    if current_width >= target_width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(target_width - current_width))
    }
}

pub fn handle_list_command(store: &Store, all: bool) -> Result<String> {
    let aliases = store.list()?;
    if aliases.is_empty() {
        return Ok("No aliases found".to_string());
    }

    let current_dir = env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let use_colors = env::var("NO_COLOR").is_err();

    // 第1パス: データ収集と最大幅計算
    let mut items = Vec::new();
    let mut max_alias_width = 0;
    let mut max_cmd_width = 0;

    for (alias, defs) in aliases {
        for def in defs {
            // スコープフィルタリング
            if !all {
                let is_relevant = match &def.scope {
                    AliasScope::Global => true,
                    AliasScope::Recursive(p) => current_dir.starts_with(p),
                    AliasScope::Exact(p) => current_dir == *p,
                };
                if !is_relevant {
                    continue;
                }
            }

            let scope_str = match def.scope {
                AliasScope::Global => "(Global)".to_string(),
                AliasScope::Recursive(p) => format!("(Recursive: {})", p),
                AliasScope::Exact(p) => format!("(Exact: {})", p),
            };

            // 幅計算（色なしベース）
            max_alias_width = max_alias_width.max(alias.len());
            max_cmd_width = max_cmd_width.max(def.command.len());

            items.push((alias.clone(), def.command.clone(), scope_str));
        }
    }

    if items.is_empty() {
        return Ok("No aliases found".to_string());
    }

    // 第2パス: フォーマット出力
    let mut output = String::new();
    for (alias, command, scope_str) in items {
        if use_colors {
            let colored_alias = alias.if_supports_color(Stream::Stdout, |text| text.cyan());
            let colored_command = command.if_supports_color(Stream::Stdout, |text| text.white());
            let colored_scope =
                scope_str.if_supports_color(Stream::Stdout, |text| text.bright_black());

            let alias_str = format!("{}", colored_alias);
            let cmd_str = format!("'{}'", colored_command);
            let scope_final = format!("{}", colored_scope);

            let padded_alias = pad_to_width(&alias_str, max_alias_width);
            let padded_cmd = pad_to_width(&cmd_str, max_cmd_width + 2); // +2 for quotes

            output.push_str(&format!(
                "{} = {} {}\n",
                padded_alias, padded_cmd, scope_final
            ));
        } else {
            let padded_alias = pad_to_width(&alias, max_alias_width);
            let padded_cmd = pad_to_width(&format!("'{}'", command), max_cmd_width + 2);

            output.push_str(&format!(
                "{} = {} {}\n",
                padded_alias, padded_cmd, scope_str
            ));
        }
    }

    if output.ends_with('\n') {
        output.pop();
    }
    Ok(output)
}
