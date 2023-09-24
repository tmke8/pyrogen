/// Warn a user once, with uniqueness determined by the calling location itself.
#[macro_export]
macro_rules! warn_user_once {
    ($($arg:tt)*) => {
        use colored::Colorize;
        use log::warn;

        static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !WARNED.swap(true, std::sync::atomic::Ordering::SeqCst) {
            let message = format!("{}", format_args!($($arg)*));
            warn!("{}", message.bold());
        }
    };
}
