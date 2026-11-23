use rfd::{FileDialog, MessageDialog};

/// Owner of the dialogs shown on the current thread. On Windows it takes the foreground
/// first, so the dialog opens in front of the active application; elsewhere the dialog
/// keeps its default placement.
pub(crate) struct DialogParent {
    #[cfg(windows)]
    owner: Option<windows_owner::Owner>,
}

impl DialogParent {
    pub(crate) fn in_front() -> Self {
        Self {
            #[cfg(windows)]
            owner: windows_owner::Owner::create(),
        }
    }

    pub(crate) fn file(&self, dialog: FileDialog) -> FileDialog {
        #[cfg(windows)]
        if let Some(owner) = &self.owner {
            return dialog.set_parent(owner);
        }
        dialog
    }

    pub(crate) fn message(&self, dialog: MessageDialog) -> MessageDialog {
        #[cfg(windows)]
        if let Some(owner) = &self.owner {
            return dialog.set_parent(owner);
        }
        dialog
    }
}

#[cfg(windows)]
mod windows_owner {
    use sushot_platform_windows::ForegroundOwner;
    use winit::raw_window_handle::{
        DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
        Win32WindowHandle, WindowHandle,
    };

    pub(super) struct Owner(ForegroundOwner);

    impl Owner {
        pub(super) fn create() -> Option<Self> {
            ForegroundOwner::new()
                .inspect_err(|error| log::warn!("dialog opens without an owner: {error}"))
                .ok()
                .map(Self)
        }
    }

    impl HasWindowHandle for Owner {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            let raw = RawWindowHandle::Win32(Win32WindowHandle::new(self.0.handle()));
            // SAFETY: the window lives until `ForegroundOwner` is dropped, which cannot
            // happen while this borrow of `self` is alive.
            Ok(unsafe { WindowHandle::borrow_raw(raw) })
        }
    }

    impl HasDisplayHandle for Owner {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            Ok(DisplayHandle::windows())
        }
    }
}
