pub fn is_dangerous(command: &str) -> bool {
    let lower = command.to_lowercase();
    let compact = lower.replace('\t', " ");
    const PATTERNS: &[&str] = &[
        "rm -rf",
        "rm -fr",
        "rm -r /",
        "mkfs",
        "format ",
        ":(){",
        "dd if=",
        "drop table",
        "drop database",
        "git push --force",
        "git push -f",
        "git reset --hard",
        "rd /s",
        "rmdir /s",
        "del /f /s",
        "remove-item -recurse",
        "invoke-expression",
        "shutdown",
        "mkfs.ext",
        "diskpart",
        "cipher /w",
    ];
    PATTERNS.iter().any(|p| compact.contains(p))
}

pub fn split_command_line(command: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = command.trim().chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(ch) = chars.next() {
        match (quote, ch) {
            (None, c) if c == '"' || c == '\'' => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, c) if c.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            (_, c) => current.push(c),
        }
    }
    if quote.is_some() {
        return Err("Unclosed quote in command".into());
    }
    if !current.is_empty() {
        args.push(current);
    }
    if args.is_empty() {
        return Err("Command is empty".into());
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_destructive_commands() {
        assert!(is_dangerous("rm -rf /"));
        assert!(is_dangerous("git push --force origin main"));
        assert!(!is_dangerous("pnpm dev"));
        assert!(!is_dangerous("cargo test"));
    }

    #[test]
    fn splits_argv_without_shell() {
        assert_eq!(
            split_command_line("pnpm run dev").unwrap(),
            vec!["pnpm", "run", "dev"]
        );
        assert_eq!(
            split_command_line("echo \"hello world\"").unwrap(),
            vec!["echo", "hello world"]
        );
    }
}
