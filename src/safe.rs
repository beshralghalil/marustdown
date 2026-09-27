use std::cell::Cell;
use std::panic::{self, UnwindSafe};

thread_local! {
    static CATCHING: Cell<bool> = const { Cell::new(false) };
}

/// Runs third-party rendering code, turning a panic into `None`.
pub fn catch<T>(f: impl FnOnce() -> T + UnwindSafe) -> Option<T> {
    CATCHING.set(true);
    let result = panic::catch_unwind(f);
    CATCHING.set(false);
    result.ok()
}

/// Makes the panic hook silent for panics that `catch` recovers from.
pub fn install_hook() {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if !CATCHING.get() {
            hook(info);
        }
    }));
}

/// True while `catch` is running, so outer hooks can stay out of the way too.
pub fn catching() -> bool {
    CATCHING.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panics_become_none() {
        assert_eq!(catch(|| 7), Some(7));
        assert_eq!(catch(|| -> u8 { panic!("renderer bug") }), None);
        assert!(!catching());
    }
}
