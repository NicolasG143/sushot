use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;

use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::screenshot::Screenshot;
use futures_util::StreamExt;
use klip_core::{Action, Frame, Mode, Output, PlatformError};

use crate::image::decode_png;
use crate::paths;
use crate::x11::fail;

const SHORTCUTS: [(&str, &str, &str, Mode, Output); 6] = [
    (
        "region",
        "Capture a region",
        "PRINT",
        Mode::Region,
        Output::Clipboard,
    ),
    (
        "full",
        "Capture the screen",
        "CTRL+PRINT",
        Mode::FullScreen,
        Output::Clipboard,
    ),
    (
        "window",
        "Capture a window",
        "SHIFT+PRINT",
        Mode::Window,
        Output::Clipboard,
    ),
    (
        "region-save",
        "Save a region",
        "ALT+PRINT",
        Mode::Region,
        Output::SaveDialog,
    ),
    (
        "full-save",
        "Save the screen",
        "CTRL+ALT+PRINT",
        Mode::FullScreen,
        Output::SaveDialog,
    ),
    (
        "window-save",
        "Save a window",
        "SHIFT+ALT+PRINT",
        Mode::Window,
        Output::SaveDialog,
    ),
];

/// Asks the desktop for a screenshot. With `interactive` the portal shows its own
/// selection UI, which replaces klip's overlay on Wayland.
pub(crate) fn screenshot(interactive: bool) -> Result<Frame, PlatformError> {
    let operation = "take a portal screenshot";
    let response = pollster::block_on(async {
        Screenshot::request()
            .interactive(interactive)
            .modal(true)
            .send()
            .await?
            .response()
    })
    .map_err(fail(operation))?;

    let path: PathBuf = paths::file_uri_path(response.uri().as_str())
        .ok_or_else(|| PlatformError::new(operation, "the portal returned a non-file location"))?;
    let file = fs::File::open(&path).map_err(fail(operation))?;
    let frame = decode_png(BufReader::new(file));
    if let Err(error) = fs::remove_file(&path) {
        log::warn!(
            "portal screenshot {} was not removed: {error}",
            path.display()
        );
    }
    frame
}

pub(crate) fn register_shortcuts(actions: Sender<Action>) -> Result<(), PlatformError> {
    let operation = "register portal shortcuts";
    let (ready_tx, ready_rx) = mpsc::channel();
    thread::Builder::new()
        .name("klip-portal-shortcuts".into())
        .spawn(move || {
            let outcome = pollster::block_on(listen(actions, &ready_tx));
            if let Err(error) = outcome {
                log::error!("portal shortcuts stopped: {error}");
                let _ = ready_tx.send(Err(error));
            }
        })
        .map_err(fail(operation))?;
    ready_rx
        .recv()
        .map_err(|_| PlatformError::new(operation, "the shortcut thread ended before reporting"))?
}

async fn listen(
    actions: Sender<Action>,
    ready: &Sender<Result<(), PlatformError>>,
) -> Result<(), PlatformError> {
    let operation = "register portal shortcuts";
    let portal = GlobalShortcuts::new().await.map_err(fail(operation))?;
    let session = portal
        .create_session(Default::default())
        .await
        .map_err(fail(operation))?;
    let wanted: Vec<NewShortcut> = SHORTCUTS
        .iter()
        .map(|(id, description, trigger, ..)| {
            NewShortcut::new(*id, *description).preferred_trigger(*trigger)
        })
        .collect();
    portal
        .bind_shortcuts(&session, &wanted, None, Default::default())
        .await
        .map_err(fail(operation))?
        .response()
        .map_err(fail(operation))?;
    let mut activations = portal.receive_activated().await.map_err(fail(operation))?;
    log::info!("portal shortcuts bound");
    let _ = ready.send(Ok(()));

    while let Some(activation) = activations.next().await {
        let Some(action) = action_for(activation.shortcut_id()) else {
            log::debug!("ignoring unknown shortcut {}", activation.shortcut_id());
            continue;
        };
        if actions.send(action).is_err() {
            break;
        }
    }
    Ok(())
}

fn action_for(id: &str) -> Option<Action> {
    SHORTCUTS
        .iter()
        .find(|(shortcut, ..)| *shortcut == id)
        .map(|(_, _, _, mode, output)| Action {
            mode: *mode,
            output: *output,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shortcut_id_maps_to_its_action() {
        assert_eq!(
            action_for("full-save"),
            Some(Action {
                mode: Mode::FullScreen,
                output: Output::SaveDialog
            })
        );
        assert_eq!(action_for("nothing"), None);
    }

    #[test]
    fn shortcut_ids_are_unique() {
        let mut ids: Vec<_> = SHORTCUTS.iter().map(|shortcut| shortcut.0).collect();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), SHORTCUTS.len());
    }
}
