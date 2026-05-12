use crate::paths;
use colored::Colorize;
use std::collections::HashMap;

pub fn sanitize_for_sandbox() -> HashMap<String, String> {
    let mut env: HashMap<String, String> = HashMap::new();

    for (key, value) in std::env::vars() {
        let upper = key.to_uppercase();
        let is_secret = paths::secret_env_var_patterns()
            .iter()
            .any(|pattern| upper.contains(pattern));
        let is_system = key == "PATH"
            || key == "SystemRoot"
            || key == "SYSTEMROOT"
            || key == "TEMP"
            || key == "TMP"
            || key == "TMPDIR"
            || key == "USER"
            || key == "USERNAME"
            || key == "LANG"
            || key == "LC_ALL"
            || key == "TERM"
            || key == "SHELL"
            || key == "COLORTERM"
            || key == "NO_COLOR"
            || key == "FORCE_COLOR"
            || key == "PNPM_HOME"
            || key == "XDG_CACHE_HOME"
            || key == "XDG_DATA_HOME"
            || key == "XDG_CONFIG_HOME";

        if is_system || (!is_secret && !key.starts_with("npm_") && !key.starts_with("NPM_")) {
            env.insert(key, value);
        }
    }

    env
}

pub fn print_env_summary(original_count: usize, stripped_count: usize) {
    let removed = original_count - stripped_count;
    println!(
        "{} Environment: {} vars passed, {} secret vars stripped",
        "→".bold(),
        stripped_count,
        removed.to_string().yellow()
    );
}
