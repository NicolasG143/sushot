use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::HotkeyKey;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    pub autostart: bool,
    pub last_save_dir: Option<PathBuf>,
    pub show_toast: bool,
    pub hotkey: HotkeyKey,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConfigError {
    line: usize,
    reason: &'static str,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.reason)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn parse(text: &str) -> Result<Config, ConfigError> {
        let mut config = Config::default();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let fail = |reason| ConfigError { line, reason };
            let raw = raw.trim();
            if raw.is_empty() || raw.starts_with('#') {
                continue;
            }
            let (key, value) = raw.split_once('=').ok_or(fail("expected `key = value`"))?;
            let key = key.trim();
            if !matches!(key, "autostart" | "show_toast" | "last_save_dir" | "hotkey") {
                continue;
            }
            let value = parse_value(value.trim()).ok_or(fail("invalid value"))?;
            match (key, value) {
                ("autostart", Value::Bool(enabled)) => config.autostart = enabled,
                ("show_toast", Value::Bool(enabled)) => config.show_toast = enabled,
                ("last_save_dir", Value::Text(dir)) if !dir.is_empty() => {
                    config.last_save_dir = Some(PathBuf::from(dir));
                }
                ("last_save_dir", Value::Text(_)) => config.last_save_dir = None,
                ("hotkey", Value::Text(name)) => {
                    config.hotkey =
                        HotkeyKey::from_name(&name).ok_or(fail("unknown hotkey name"))?;
                }
                _ => return Err(fail("value has the wrong type")),
            }
        }
        Ok(config)
    }

    pub fn to_toml(&self) -> String {
        let mut text = format!("autostart = {}\n", self.autostart);
        if let Some(dir) = self.last_save_dir.as_deref().and_then(Path::to_str) {
            text.push_str(&format!("last_save_dir = {}\n", quote(dir)));
        }
        text.push_str(&format!("show_toast = {}\n", self.show_toast));
        text.push_str(&format!("hotkey = \"{}\"\n", self.hotkey));
        text
    }
}

enum Value {
    Bool(bool),
    Text(String),
}

fn parse_value(token: &str) -> Option<Value> {
    if let Some(rest) = token.strip_prefix('"') {
        let (text, tail) = parse_basic_string(rest)?;
        return is_blank_or_comment(tail).then_some(Value::Text(text));
    }
    if let Some(rest) = token.strip_prefix('\'') {
        let (text, tail) = rest.split_once('\'')?;
        return is_blank_or_comment(tail).then(|| Value::Text(text.to_owned()));
    }
    match token.split('#').next()?.trim() {
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ => None,
    }
}

fn parse_basic_string(rest: &str) -> Option<(String, &str)> {
    let mut text = String::new();
    let mut chars = rest.char_indices();
    while let Some((index, c)) = chars.next() {
        match c {
            '"' => return Some((text, &rest[index + 1..])),
            '\\' => match chars.next()?.1 {
                '\\' => text.push('\\'),
                '"' => text.push('"'),
                'n' => text.push('\n'),
                't' => text.push('\t'),
                'r' => text.push('\r'),
                _ => return None,
            },
            _ => text.push(c),
        }
    }
    None
}

fn is_blank_or_comment(tail: &str) -> bool {
    let tail = tail.trim();
    tail.is_empty() || tail.starts_with('#')
}

fn quote(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for c in text.chars() {
        match c {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\t' => quoted.push_str("\\t"),
            '\r' => quoted.push_str("\\r"),
            _ => quoted.push(c),
        }
    }
    quoted.push('"');
    quoted
}

/// Reads the configuration at `path`. A missing or corrupt file yields the defaults
/// and is rewritten with them; this never fails.
pub fn load(path: &Path) -> Config {
    match fs::read_to_string(path) {
        Ok(text) => match Config::parse(&text) {
            Ok(config) => return config,
            Err(error) => log::warn!(
                "configuration at {} is corrupt ({error}); using defaults",
                path.display()
            ),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            log::info!("no configuration at {}; creating it", path.display());
        }
        Err(error) => log::warn!(
            "configuration at {} is unreadable ({error}); using defaults",
            path.display()
        ),
    }
    let config = Config::default();
    if let Err(error) = save(path, &config) {
        log::warn!(
            "defaults could not be written to {}: {error}",
            path.display()
        );
    }
    config
}

/// Replaces the file through a temporary sibling so a crash never leaves it half written.
pub fn save(path: &Path, config: &Config) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut staging = path.as_os_str().to_owned();
    staging.push(".tmp");
    let staging = PathBuf::from(staging);
    fs::write(&staging, config.to_toml())?;
    fs::rename(&staging, path).inspect_err(|_| {
        let _ = fs::remove_file(&staging);
    })
}

/// Loads, applies `change` and saves, so writers touching different fields do not
/// overwrite each other with stale copies.
pub fn update(path: &Path, change: impl FnOnce(&mut Config)) -> io::Result<Config> {
    let mut config = load(path);
    change(&mut config);
    save(path, &config)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("klip-config-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self) -> PathBuf {
            self.0.join("klip").join("config.toml")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn empty_text_yields_the_defaults() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
        assert!(!Config::default().autostart);
        assert!(!Config::default().show_toast);
        assert_eq!(Config::default().last_save_dir, None);
    }

    #[test]
    fn serialized_config_parses_back_to_the_same_values() {
        let config = Config {
            autostart: true,
            last_save_dir: Some(PathBuf::from(r#"C:\Users\Ana "Pics"\Imágenes"#)),
            show_toast: true,
            hotkey: HotkeyKey::Function(9),
        };

        assert_eq!(Config::parse(&config.to_toml()).unwrap(), config);
    }

    #[test]
    fn parser_accepts_literal_strings_comments_and_unknown_keys() {
        let text = "# klip\nautostart = true # on\n\nlast_save_dir = 'C:\\Users\\Ana'\nfuture = 3\nshow_toast=false\n";

        let config = Config::parse(text).unwrap();

        assert!(config.autostart);
        assert!(!config.show_toast);
        assert_eq!(config.last_save_dir, Some(PathBuf::from(r"C:\Users\Ana")));
    }

    #[test]
    fn parser_rejects_malformed_lines_and_wrong_types() {
        for text in [
            "autostart",
            "autostart = maybe",
            "autostart = \"true\"",
            "last_save_dir = true",
            "last_save_dir = \"unterminated",
            "last_save_dir = \"bad \\q escape\"",
            "[section]",
            "hotkey = \"Enter\"",
            "hotkey = true",
        ] {
            assert!(Config::parse(text).is_err(), "{text:?} should be rejected");
        }
    }

    #[test]
    fn parser_reports_the_offending_line() {
        let error = Config::parse("autostart = true\nshow_toast = 7\n").unwrap_err();

        assert_eq!(error.line, 2);
    }

    #[test]
    fn load_creates_the_missing_file_with_the_defaults() {
        let scratch = Scratch::new("missing");

        let config = load(&scratch.file());

        assert_eq!(config, Config::default());
        assert_eq!(
            Config::parse(&fs::read_to_string(scratch.file()).unwrap()).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn load_replaces_a_corrupt_file_with_the_defaults() {
        let scratch = Scratch::new("corrupt");
        fs::create_dir_all(scratch.file().parent().unwrap()).unwrap();
        fs::write(scratch.file(), "autostart = ???").unwrap();

        let config = load(&scratch.file());

        assert_eq!(config, Config::default());
        assert_eq!(
            fs::read_to_string(scratch.file()).unwrap(),
            Config::default().to_toml()
        );
    }

    #[test]
    fn update_persists_one_field_and_keeps_the_others() {
        let scratch = Scratch::new("update");
        save(
            &scratch.file(),
            &Config {
                autostart: true,
                ..Config::default()
            },
        )
        .unwrap();

        update(&scratch.file(), |config| {
            config.last_save_dir = Some(PathBuf::from("D:\\Shots"));
        })
        .unwrap();

        assert_eq!(
            load(&scratch.file()),
            Config {
                autostart: true,
                last_save_dir: Some(PathBuf::from("D:\\Shots")),
                show_toast: false,
                hotkey: HotkeyKey::PrintScreen,
            }
        );
    }
}
