use crate::Result;
use crate::Store;

pub fn handle_list_command(store: &Store) -> Result<String> {
    let aliases = store.list()?;
    if aliases.is_empty() {
        Ok("No aliases found".to_string())
    } else {
        let mut output = String::new();
        for (alias, command) in aliases {
            output.push_str(&format!("{} = '{}'\n", alias, command));
        }

        if output.ends_with('\n') {
            output.pop();
        }
        Ok(output)
    }
}
