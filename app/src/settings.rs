use std::path::Path;

use sushot_core::{PlatformError, update_config};

/// Applies the autostart choice to the system first and remembers it in the
/// configuration only once the system accepted it, so the two never disagree.
pub(crate) fn set_autostart(
    config_path: Option<&Path>,
    enabled: bool,
    apply: impl FnOnce(bool) -> Result<(), PlatformError>,
) -> Result<(), PlatformError> {
    apply(enabled)?;
    if let Some(config_path) = config_path
        && let Err(error) = update_config(config_path, |config| config.autostart = enabled)
    {
        log::warn!("autostart choice was not saved to the configuration: {error}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use sushot_core::{Config, HotkeyKey, load_config, save_config};

    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("sushot-settings-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn config(&self) -> PathBuf {
            self.0.join("config.toml")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn accepted_choice_reaches_the_system_and_the_configuration() {
        let scratch = Scratch::new("accepted");
        let mut applied = None;

        set_autostart(Some(&scratch.config()), true, |enabled| {
            applied = Some(enabled);
            Ok(())
        })
        .unwrap();

        assert_eq!(applied, Some(true));
        assert!(load_config(&scratch.config()).autostart);
    }

    #[test]
    fn rejected_choice_leaves_the_configuration_untouched() {
        let scratch = Scratch::new("rejected");
        save_config(&scratch.config(), &Config::default()).unwrap();

        let result = set_autostart(Some(&scratch.config()), true, |_| {
            Err(PlatformError::new("update autostart entry", "denied"))
        });

        assert!(result.is_err());
        assert!(!load_config(&scratch.config()).autostart);
    }

    #[test]
    fn other_fields_survive_a_change() {
        let scratch = Scratch::new("others");
        save_config(
            &scratch.config(),
            &Config {
                show_toast: true,
                last_save_dir: Some(PathBuf::from("D:\\Shots")),
                ..Config::default()
            },
        )
        .unwrap();

        set_autostart(Some(&scratch.config()), true, |_| Ok(())).unwrap();

        assert_eq!(
            load_config(&scratch.config()),
            Config {
                autostart: true,
                show_toast: true,
                last_save_dir: Some(PathBuf::from("D:\\Shots")),
                hotkey: HotkeyKey::PrintScreen,
            }
        );
    }

    #[test]
    fn without_a_configuration_path_the_system_still_changes() {
        let mut applied = None;

        set_autostart(None, false, |enabled| {
            applied = Some(enabled);
            Ok(())
        })
        .unwrap();

        assert_eq!(applied, Some(false));
    }
}
