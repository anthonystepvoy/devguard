use colored::Colorize;

#[allow(dead_code)]
pub fn allowed_registry_hosts() -> Vec<&'static str> {
    vec![
        "registry.npmjs.org",
        "registry.yarnpkg.com",
        "pnpm.io",
        "github.com",
        "api.github.com",
    ]
}

#[allow(dead_code)]
pub fn blocked_hosts() -> Vec<&'static str> {
    vec![
        "api.github.com/repos/.*/private", // making repos public
        "api.github.com/user/keys",        // adding SSH keys
        "api.github.com/repos/.*/branches/.*/protection", // branch protection
        "webhook.site",
        "requestbin.com",
        "pastebin.com",
        "hastebin.com",
        "discord.com/api/webhooks",
        "hooks.slack.com",
        "oast.fun",
        "interact.sh",
        "burpcollaborator.net",
        "canarytokens.com",
        "pipedream.net",
        "hookbin.com",
        "beeceptor.com",
        "mockbin.org",
    ]
}

#[allow(dead_code)]
pub fn suspicious_patterns() -> Vec<&'static str> {
    vec![
        "publish",
        "npmjs.com/-/npm/v1/security-advisories",
        "api.github.com/user/repos.*private.*true",
    ]
}

pub fn print_network_policy(allow_network: bool) {
    if allow_network {
        println!(
            "{} Network: {} (full access)",
            "→".bold(),
            "ALLOWED".yellow().bold()
        );
    } else {
        println!(
            "{} Network: {} (policy listed, not enforced)",
            "→".bold(),
            "ADVISORY".yellow().bold()
        );
    }
}
