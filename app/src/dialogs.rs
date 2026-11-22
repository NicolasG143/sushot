use std::io;
use std::path::{Path, PathBuf};

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

use crate::save::Dialogs;

const TITLE: &str = "klip";

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
