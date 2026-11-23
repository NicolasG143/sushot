#![cfg(windows)]

use std::time::Instant;

use sushot_core::{Platform, encode_png};
use sushot_platform_windows::WindowsPlatform;

#[test]
#[ignore = "prints timings and replaces the clipboard contents"]
fn stage_timings_of_a_full_desktop_copy() {
    for round in 0..5 {
        let started = Instant::now();
        let frame = WindowsPlatform.grab_virtual_desktop().unwrap();
        let grabbed = started.elapsed();

        let encode_started = Instant::now();
        let png = encode_png(&frame).unwrap();
        let encoded = encode_started.elapsed();

        let copy_started = Instant::now();
        WindowsPlatform.copy_image(&frame).unwrap();
        let copied = copy_started.elapsed();

        eprintln!(
            "round {round}: grab {grabbed:?}, png {encoded:?} ({} KB), copy (png+dib+clipboard) {copied:?}",
            png.len() / 1024
        );
    }
}
