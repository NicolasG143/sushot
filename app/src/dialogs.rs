use std::io;
use std::path::{Path, PathBuf};
use std::thread;

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

use crate::save::Dialogs;

const TITLE: &str = "sushot";
const FIRST_RUN_NOTICE: &str = "sushot está activo en la bandeja del sistema.\n\n\
En Windows 11, Impr Pant puede estar asociada a Recortes en Configuración > Accesibilidad > \
Teclado. sushot tiene prioridad sobre Recortes, pero si Recortes se abre igual, desactivá esa opción.

Alt izquierdo + Shift izquierdo + Impr Pant también activa el Contraste alto de Windows. Usá Alt o Shift derechos, o desactivá el atajo en Configuración > Accesibilidad > Temas de contraste.";

#[derive(Clone, Copy)]
pub(crate) struct NativeDialogs;

impl Dialogs for NativeDialogs {
    fn pick_path(&self, directory: Option<&Path>, file_name: &str) -> Option<PathBuf> {
        let mut dialog = FileDialog::new()
            .set_title("Guardar captura")
            .set_file_name(file_name)
            .add_filter("PNG", &["png"]);
        if let Some(directory) = directory {
            dialog = dialog.set_directory(directory);
        }
        dialog.save_file().map(|path| match path.extension() {
            Some(_) => path,
            None => path.with_extension("png"),
        })
    }

    fn retry_after_failure(&self, path: &Path, error: &io::Error) -> bool {
        let answer = MessageDialog::new()
            .set_level(MessageLevel::Error)
            .set_title(TITLE)
            .set_description(format!(
                "No se pudo guardar la captura en {}.\n\n{error}\n\n¿Querés intentar de nuevo?",
                path.display()
            ))
            .set_buttons(MessageButtons::YesNo)
            .show();
        answer == MessageDialogResult::Yes
    }
}

/// Tells a first-time user where sushot lives and what to do about Windows 11 binding
/// Print Screen to Snipping Tool. Runs on its own thread because the dialog blocks.
pub(crate) fn announce_first_run() {
    let spawned = thread::Builder::new()
        .name("sushot-welcome".into())
        .spawn(|| {
            MessageDialog::new()
                .set_level(MessageLevel::Info)
                .set_title(TITLE)
                .set_description(FIRST_RUN_NOTICE)
                .set_buttons(MessageButtons::Ok)
                .show();
        });
    if let Err(error) = spawned {
        log::warn!("first run notice could not be shown: {error}");
    }
}
