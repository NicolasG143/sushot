use std::error::Error;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::thread;

use sushot_core::Action;
use winit::event_loop::EventLoopProxy;

use crate::app::UiEvent;
use crate::cli::{decode_request, encode_request};

/// Starts accepting capture requests from `sushot --region` and its siblings. A stale
/// socket left by a crashed instance is replaced; the single-instance lock held by the
/// caller guarantees nobody else is listening.
pub(crate) fn listen(path: &Path, proxy: EventLoopProxy<UiEvent>) -> Result<(), Box<dyn Error>> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.into()),
        _ => {}
    }
    let listener = UnixListener::bind(path)?;
    log::info!("accepting capture requests on {}", path.display());
    thread::Builder::new()
        .name("sushot-ipc".into())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => serve(stream, &proxy),
                    Err(error) => log::warn!("capture request connection failed: {error}"),
                }
            }
        })?;
    Ok(())
}

fn serve(stream: UnixStream, proxy: &EventLoopProxy<UiEvent>) {
    let mut line = String::new();
    if let Err(error) = BufReader::new(stream).read_line(&mut line) {
        log::warn!("capture request was not read: {error}");
        return;
    }
    match decode_request(&line) {
        Some(action) => {
            if proxy.send_event(UiEvent::Action(action)).is_err() {
                log::debug!("capture request dropped: the event loop is closed");
            }
        }
        None => log::warn!("ignoring malformed capture request {line:?}"),
    }
}

/// Hands `action` to the running instance listening on `path`.
pub(crate) fn send(path: &Path, action: Action) -> Result<(), Box<dyn Error>> {
    let mut stream = UnixStream::connect(path)
        .map_err(|error| format!("sushot is not running ({}: {error})", path.display()))?;
    writeln!(stream, "{}", encode_request(action))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::os::unix::net::UnixListener;

    use sushot_core::{Mode, Output};

    use super::*;

    #[test]
    fn a_request_reaches_the_listener_as_one_line() {
        let path =
            std::env::temp_dir().join(format!("sushot-ipc-test-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();

        send(
            &path,
            Action {
                mode: Mode::Window,
                output: Output::SaveDialog,
            },
        )
        .unwrap();

        let (mut stream, _) = listener.accept().unwrap();
        let mut received = String::new();
        stream.read_to_string(&mut received).unwrap();
        assert_eq!(received, "window save\n");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sending_without_a_listener_explains_that_sushot_is_not_running() {
        let path = std::env::temp_dir().join("sushot-ipc-test-missing.sock");

        let error = send(
            &path,
            Action {
                mode: Mode::Region,
                output: Output::Clipboard,
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("sushot is not running"));
    }
}
