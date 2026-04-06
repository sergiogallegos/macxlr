use crate::DaemonState;
use crate::events::EventTriggers;
use anyhow::Result;
use std::sync::atomic::Ordering;
use tokio::sync::mpsc;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

pub fn handle_tray(state: DaemonState, tx: mpsc::Sender<EventTriggers>) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use std::sync::atomic::Ordering;
        use tokio::task;
        if state.show_tray.load(Ordering::Relaxed) {
            // We'll just spawn the tray and return.
            task::spawn(linux::handle_tray(state.shutdown, tx));
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        if state.show_tray.load(Ordering::Relaxed) {
            macos::handle_tray(state, tx)
        } else {
            Ok(())
        }
    }
    #[cfg(target_os = "windows")]
    {
        if state.show_tray.load(Ordering::Relaxed) {
            windows::handle_tray(state, tx)
        } else {
            Ok(())
        }
    }

    // For all other platforms, don't attempt to spawn a Tray Icon
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        // For now, don't spawn a tray icon.
        Ok(())
    }
}
