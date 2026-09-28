/* ==========================================================================
 * Fatal failure surface
 * A release build has no console a user can read, so a failure that stops the
 * application from starting — or a panic that kills a worker — would otherwise
 * be invisible: the window never appears, or disappears, and nothing says why.
 * This module is the one place that turns such a failure into something the
 * user can see and act on.
 *
 * Two rules keep it honest:
 *
 * - the log is written first, so the log file always holds the detail even when
 *   the dialog is dismissed with the keyboard;
 * - the dialog is shown at most once per process, so a panic loop cannot bury
 *   the window under stacked windows that cannot be dismissed.
 *
 * Nothing here runs on a healthy path: the window appears, and none of this is
 * reached.
 * ========================================================================== */

use std::sync::atomic::{AtomicBool, Ordering};

/// Whether a fatal dialog has already been shown. A second failure while the
/// user is reading the first one is logged, not stacked on top of it.
static SHOWN: AtomicBool = AtomicBool::new(false);

/// Reports a failure that prevents CrossPort from running, then lets the caller
/// stop. Logs first, then shows the message once.
pub fn show_fatal(title: &str, message: &str) {
    log::error!("{title}: {message}");
    show_once(title, message);
}

/// Installs a panic hook that logs the panic and tells the user.
///
/// The previous hook still runs, so the process keeps its usual panic output;
/// this only adds the two things a desktop user needs: a durable log entry and
/// a window that says the app has to stop.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let detail = match info.location() {
            Some(location) => format!(
                "{} ({}:{}:{})",
                panic_message(info.payload()),
                location.file(),
                location.line(),
                location.column()
            ),
            None => panic_message(info.payload()),
        };
        log::error!("CrossPort panicked: {detail}");
        previous(info);

        show_once(
            "CrossPort stopped unexpectedly",
            "CrossPort hit an unexpected problem and has to stop.\n\n\
             Work that had already finished is kept. If a transfer was running, \
             reopening CrossPort will show it as interrupted so it can be \
             restarted or discarded — nothing is restarted on its own.\n\n\
             The details are in CrossPort's log folder.",
        );
    }));
}

/// The message a panic payload carries, in every shape a payload comes in.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic".to_string()
}

/// Shows the message if nothing has been shown yet.
fn show_once(title: &str, message: &str) {
    if SHOWN.swap(true, Ordering::SeqCst) {
        return;
    }
    present(title, message);
}

/// Presents the message. Tests replace the presenter so the one-dialog budget
/// is asserted without opening a modal window on the machine running the suite.
fn present(title: &str, message: &str) {
    #[cfg(test)]
    {
        if let Some(presenter) = crate::errors::dialog::test_presenter::take() {
            presenter(title, message);
            return;
        }
    }
    show(title, message);
}

#[cfg(windows)]
fn show(title: &str, message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MB_TOPMOST,
    };

    let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let message: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();

    // SAFETY: both strings are null-terminated UTF-16 buffers that outlive the
    // call. The dialog is modal and belongs to this thread only, which is
    // exactly what a message that has to be read before the process stops
    // should be.
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_TOPMOST | MB_SETFOREGROUND,
        );
    }
}

/// Non-Windows hosts keep the console report; the desktop targets are Windows.
#[cfg(not(windows))]
fn show(title: &str, message: &str) {
    eprintln!("{title}: {message}");
}

#[cfg(test)]
pub(crate) mod test_presenter {
    use std::sync::Mutex;

    type Presenter = Box<dyn Fn(&str, &str) + Send + Sync>;

    static PRESENTER: Mutex<Option<Presenter>> = Mutex::new(None);

    /// Installs the presenter the next dialog uses, once.
    pub fn install(presenter: impl Fn(&str, &str) + Send + Sync + 'static) {
        *PRESENTER.lock().expect("the presenter lock is available") = Some(Box::new(presenter));
    }

    /// Consumes the installed presenter, if there is one.
    pub fn take() -> Option<Presenter> {
        PRESENTER
            .lock()
            .expect("the presenter lock is available")
            .take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Both tests touch the process-wide guard, so they take turns.
    static TURN: Mutex<()> = Mutex::new(());

    #[test]
    fn a_panic_message_is_readable_in_every_shape() {
        assert_eq!(panic_message(&"plain"), "plain");
        assert_eq!(panic_message(&"owned".to_string()), "owned");
        assert_eq!(panic_message(&42u32), "unknown panic");
    }

    #[test]
    fn only_the_first_failure_opens_a_dialog() {
        let _turn = TURN.lock().expect("the turn lock is available");
        SHOWN.store(false, Ordering::SeqCst);

        let seen = std::sync::Arc::new(Mutex::new(Vec::new()));
        for index in 0..2 {
            let seen = std::sync::Arc::clone(&seen);
            test_presenter::install(move |title: &str, _message: &str| {
                seen.lock()
                    .expect("the record lock is available")
                    .push(title.to_string());
                let _ = index;
            });
            show_once("first", "a failure");
        }

        let seen = seen.lock().expect("the record lock is available");
        assert_eq!(
            seen.len(),
            1,
            "a second failure must be logged, not stacked on a dialog the user cannot dismiss"
        );
        assert_eq!(seen[0], "first");
    }
}
