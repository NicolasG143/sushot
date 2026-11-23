use std::path::{Path, PathBuf};

/// Environment values only count when they are absolute Unix paths.
fn absolute(value: Option<&str>) -> Option<PathBuf> {
    value
        .filter(|text| text.starts_with('/'))
        .map(PathBuf::from)
}

/// `$XDG_CONFIG_HOME`, or `~/.config` when it is unset, empty or not absolute.
fn config_home(xdg_config_home: Option<&str>, home: Option<&Path>) -> Option<PathBuf> {
    absolute(xdg_config_home).or_else(|| home.map(|home| home.join(".config")))
}

pub(crate) fn config_path(xdg_config_home: Option<&str>, home: Option<&Path>) -> Option<PathBuf> {
    config_home(xdg_config_home, home).map(|dir| dir.join("sushot").join("config.toml"))
}

pub(crate) fn autostart_entry_path(
    xdg_config_home: Option<&str>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    config_home(xdg_config_home, home).map(|dir| dir.join("autostart").join("sushot.desktop"))
}

pub(crate) fn lock_path(xdg_runtime_dir: Option<&str>) -> Option<PathBuf> {
    absolute(xdg_runtime_dir).map(|dir| dir.join("sushot.lock"))
}

pub(crate) fn socket_path(xdg_runtime_dir: Option<&str>) -> Option<PathBuf> {
    lock_path(xdg_runtime_dir).map(|lock| lock.with_file_name("sushot.sock"))
}

/// Reads `XDG_PICTURES_DIR` from the text of `user-dirs.dirs`, falling back to `~/Pictures`.
pub(crate) fn pictures_dir(home: Option<&Path>, user_dirs: Option<&str>) -> Option<PathBuf> {
    let home = home?;
    let configured = user_dirs
        .into_iter()
        .flat_map(str::lines)
        .filter_map(|line| line.trim().strip_prefix("XDG_PICTURES_DIR="))
        .find_map(|value| {
            let quoted = value.trim().strip_prefix('"')?.strip_suffix('"')?;
            match quoted.strip_prefix("$HOME") {
                Some(rest) => Some(PathBuf::from(format!("{}{rest}", home.display()))),
                None => absolute(Some(quoted)),
            }
        });
    Some(configured.unwrap_or_else(|| home.join("Pictures")))
}

/// Turns a `file://` URI, as portals return them, into a path.
pub(crate) fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let encoded = uri.strip_prefix("file://")?;
    let encoded = encoded.strip_prefix("localhost").unwrap_or(encoded);
    if !encoded.starts_with('/') {
        return None;
    }
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = encoded.get(index + 1..index + 3)?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unix(path: Option<PathBuf>) -> Option<String> {
        path.map(|path| path.to_string_lossy().replace('\\', "/"))
    }

    #[test]
    fn config_lives_under_xdg_config_home_when_it_is_absolute() {
        let path = config_path(Some("/custom/cfg"), Some(Path::new("/home/ana")));

        assert_eq!(
            unix(path),
            Some("/custom/cfg/sushot/config.toml".to_owned())
        );
    }

    #[test]
    fn config_falls_back_to_dot_config_for_unset_empty_or_relative_values() {
        let home = Some(Path::new("/home/ana"));
        let expected = Some("/home/ana/.config/sushot/config.toml".to_owned());

        assert_eq!(unix(config_path(None, home)), expected);
        assert_eq!(unix(config_path(Some(""), home)), expected);
        assert_eq!(unix(config_path(Some("relative/dir"), home)), expected);
    }

    #[test]
    fn config_is_unknown_without_any_base_directory() {
        assert_eq!(config_path(None, None), None);
    }

    #[test]
    fn autostart_entry_sits_next_to_the_config_home() {
        let path = autostart_entry_path(None, Some(Path::new("/home/ana")));

        assert_eq!(
            unix(path),
            Some("/home/ana/.config/autostart/sushot.desktop".to_owned())
        );
    }

    #[test]
    fn runtime_files_need_an_absolute_runtime_directory() {
        assert_eq!(
            unix(lock_path(Some("/run/user/1000"))),
            Some("/run/user/1000/sushot.lock".to_owned())
        );
        assert_eq!(
            unix(socket_path(Some("/run/user/1000"))),
            Some("/run/user/1000/sushot.sock".to_owned())
        );
        assert_eq!(lock_path(Some("tmp")), None);
        assert_eq!(lock_path(Some("")), None);
        assert_eq!(lock_path(None), None);
    }

    #[test]
    fn pictures_dir_expands_home_in_user_dirs() {
        let home = Path::new("/home/ana");
        let text = "XDG_DESKTOP_DIR=\"$HOME/Escritorio\"\nXDG_PICTURES_DIR=\"$HOME/Imágenes\"\n";

        assert_eq!(
            unix(pictures_dir(Some(home), Some(text))),
            Some("/home/ana/Imágenes".to_owned())
        );
    }

    #[test]
    fn pictures_dir_accepts_absolute_paths_and_defaults_otherwise() {
        let home = Path::new("/home/ana");

        assert_eq!(
            unix(pictures_dir(
                Some(home),
                Some("XDG_PICTURES_DIR=\"/data/pics\"")
            )),
            Some("/data/pics".to_owned())
        );
        assert_eq!(
            unix(pictures_dir(
                Some(home),
                Some("XDG_PICTURES_DIR=\"relative\"")
            )),
            Some("/home/ana/Pictures".to_owned())
        );
        assert_eq!(
            unix(pictures_dir(Some(home), None)),
            Some("/home/ana/Pictures".to_owned())
        );
        assert_eq!(pictures_dir(None, None), None);
    }

    #[test]
    fn file_uri_decodes_percent_escapes() {
        assert_eq!(
            unix(file_uri_path("file:///home/ana/Im%C3%A1genes/a%20b.png")),
            Some("/home/ana/Imágenes/a b.png".to_owned())
        );
        assert_eq!(
            unix(file_uri_path("file://localhost/tmp/x.png")),
            Some("/tmp/x.png".to_owned())
        );
    }

    #[test]
    fn file_uri_rejects_other_schemes_and_broken_escapes() {
        assert_eq!(file_uri_path("https://example.com/x.png"), None);
        assert_eq!(file_uri_path("file://host/x.png"), None);
        assert_eq!(file_uri_path("file:///bad%zz"), None);
        assert_eq!(file_uri_path("file:///cut%2"), None);
    }
}
