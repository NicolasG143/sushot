#![cfg(windows)]

use sushot_core::Platform;
use sushot_platform_windows::WindowsPlatform;

#[test]
fn local_time_is_a_plausible_calendar_moment() {
    let now = WindowsPlatform.local_time();

    assert!(now.year >= 2024);
    assert!((1..=12).contains(&now.month));
    assert!((1..=31).contains(&now.day));
    assert!(now.hour < 24 && now.minute < 60 && now.second < 61);
}

#[test]
fn config_path_points_to_sushot_config_inside_the_roaming_profile() {
    let path = WindowsPlatform.config_path().expect("a roaming profile");

    assert!(path.ends_with(r"sushot\config.toml"));
    assert!(path.is_absolute());
}

#[test]
fn pictures_dir_is_an_existing_folder() {
    let dir = WindowsPlatform.pictures_dir().expect("a pictures folder");

    assert!(dir.is_dir());
}
