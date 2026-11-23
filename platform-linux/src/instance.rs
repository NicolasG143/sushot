use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

/// Takes an exclusive lock on `path`. The lock lasts as long as the returned file is open
/// and disappears if the process dies. `None` means another process holds it.
pub(crate) fn claim(path: &Path) -> io::Result<Option<File>> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn lock_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sushot-linux-lock-{name}-{}", std::process::id()))
    }

    #[test]
    fn second_claim_is_refused_until_the_first_lock_is_released() {
        let path = lock_file("claim");

        let first = claim(&path).unwrap();
        assert!(first.is_some());
        assert!(claim(&path).unwrap().is_none());

        drop(first);
        assert!(claim(&path).unwrap().is_some());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn claim_fails_when_the_directory_does_not_exist() {
        let path = lock_file("missing").join("nested").join("sushot.lock");

        assert!(claim(&path).is_err());
    }
}
