#![cfg(target_os = "linux")]

use std::process::Command;

use sushot_core::{Frame, Platform, Rect};
use sushot_platform_linux::LinuxPlatform;

fn read_clipboard_png_from_another_process() -> Vec<u8> {
    let output = Command::new("xclip")
        .args(["-selection", "clipboard", "-t", "image/png", "-o"])
        .output()
        .expect("xclip is installed");
    assert!(
        output.status.success(),
        "xclip could not read a PNG from the clipboard: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn png_size(png: &[u8]) -> (u32, u32) {
    let reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .expect("the clipboard holds a valid PNG");
    let info = reader.info();
    (info.width, info.height)
}

#[test]
#[ignore = "overwrites the clipboard; needs an X server in DISPLAY and xclip"]
fn copied_image_stays_on_the_clipboard_for_other_applications() {
    let bounds = Rect {
        x: 0,
        y: 0,
        w: 3,
        h: 2,
    };
    let frame = Frame::new(bounds, vec![200; 3 * 2 * 4]).expect("buffer matches the bounds");

    LinuxPlatform
        .copy_image(&frame)
        .expect("the clipboard accepts the image");

    let png = read_clipboard_png_from_another_process();
    assert_eq!(png_size(&png), (3, 2), "PNG size");
}
