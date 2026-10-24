//! Watches `~/.local/share/fonts` so the list refreshes when fonts come and go
//! (cosmic-settings reads its font list once, at startup, and never again).

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::installer::user_font_dir;

/// A running watcher. Dropping it stops the watch.
pub struct FontDirWatcher {
    _watcher: RecommendedWatcher,
    rx: Receiver<()>,
    pub dir: PathBuf,
}

impl FontDirWatcher {
    /// Starts watching, creating the directory first if needed (a missing one can't be watched).
    ///
    /// # Errors
    /// If the watcher can't be created or the directory can't be watched.
    pub fn start() -> notify::Result<Self> {
        let dir = user_font_dir()
            .ok_or_else(|| notify::Error::generic("no home directory, no user font directory"))?;
        let _ = std::fs::create_dir_all(&dir);

        let (tx, rx) = mpsc::channel::<()>();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res
                && matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_)
                )
            {
                let _ = tx.send(());
            }
        })?;

        watcher.watch(&dir, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            rx,
            dir,
        })
    }

    /// Whether anything changed since the last call, without blocking. A burst (installing
    /// 20 files) counts once.
    #[must_use]
    pub fn poll(&self) -> bool {
        let mut changed = false;
        while self.rx.try_recv().is_ok() {
            changed = true;
        }
        changed
    }

    /// Blocks until something changes. Returns `false` if the watcher stopped.
    #[must_use]
    pub fn wait(&self) -> bool {
        if self.rx.recv().is_ok() {
            // Drain the burst.
            while self.rx.try_recv().is_ok() {}
            true
        } else {
            false
        }
    }
}
