use std::fs;
use std::io;
use std::path::Path;

/// Quotes an `Exec` argument as the Desktop Entry specification requires.
fn quote_exec(argument: &str) -> String {
    let mut quoted = String::from("\"");
    for c in argument.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    quoted.replace('%', "%%")
}

pub(crate) fn desktop_entry(exe: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=klip\nComment=Screenshot utility\nExec={}\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        quote_exec(&exe.to_string_lossy())
    )
}

/// Writes the autostart entry at `entry` for `exe`, or removes it.
pub(crate) fn set(entry: &Path, exe: &Path, enabled: bool) -> io::Result<()> {
    if enabled {
        if let Some(parent) = entry.parent() {
            fs::create_dir_all(parent)?;
        }
        return fs::write(entry, desktop_entry(exe));
    }
    match fs::remove_file(entry) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "klip-linux-autostart-{name}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn entry_launches_the_executable_quoted() {
        let entry = desktop_entry(Path::new("/opt/my apps/klip"));

        assert!(entry.starts_with("[Desktop Entry]\n"));
        assert!(entry.contains("Type=Application\n"));
        assert!(entry.contains("Exec=\"/opt/my apps/klip\"\n"));
    }

    #[test]
    fn exec_escapes_reserved_characters() {
        assert_eq!(quote_exec("a\"b$c`d\\e"), "\"a\\\"b\\$c\\`d\\\\e\"");
        assert_eq!(quote_exec("100%"), "\"100%%\"");
    }

    #[test]
    fn enabling_creates_the_missing_directory_and_disabling_removes_the_file() {
        let scratch = Scratch::new("toggle");
        let entry = scratch.0.join("autostart").join("klip.desktop");

        set(&entry, Path::new("/usr/bin/klip"), true).unwrap();
        assert!(
            fs::read_to_string(&entry)
                .unwrap()
                .contains("/usr/bin/klip")
        );

        set(&entry, Path::new("/usr/bin/klip"), false).unwrap();
        assert!(!entry.exists());
    }

    #[test]
    fn disabling_without_an_entry_is_not_an_error() {
        let scratch = Scratch::new("missing");

        set(
            &scratch.0.join("klip.desktop"),
            Path::new("/usr/bin/klip"),
            false,
        )
        .unwrap();
    }
}
