use lazy_static::lazy_static;

#[allow(unused)]
macro_rules! error {
    ($domain:literal, $($arg:tt)*) => {{
        $crate::logging::Logger::log_error($domain, format_args!($($arg)*));
    }}
}
pub(crate) use error;

#[allow(unused)]
macro_rules! critical {
    ($domain:literal, $($arg:tt)*) => {{
        $crate::logging::Logger::log_critical($domain, format_args!($($arg)*));
    }}
}
pub(crate) use critical;

#[allow(unused)]
macro_rules! warning {
    ($domain:literal, $($arg:tt)*) => {{
        $crate::logging::Logger::log_warn($domain, format_args!($($arg)*));
    }}
}
pub(crate) use warning;

#[allow(unused)]
macro_rules! message {
    ($domain:literal, $($arg:tt)*) => {{
        $crate::logging::Logger::log_message($domain, format_args!($($arg)*));
    }}
}
pub(crate) use message;

#[allow(unused)]
macro_rules! info {
    ($domain:literal, $($arg:tt)*) => {{
        $crate::logging::Logger::log_info($domain, format_args!($($arg)*));
    }}
}
pub(crate) use info;

#[allow(unused)]
macro_rules! debug {
    ($domain:literal, $($arg:tt)*) => {{
        $crate::logging::Logger::log_debug($domain, format_args!($($arg)*));
    }}
}
pub(crate) use debug;

macro_rules! now {
    () => {
        unsafe {
            let now = libc::time(std::ptr::null_mut());
            if now == core::mem::transmute(-1_i64) {
                std::mem::zeroed()
            } else {
                let tm = libc::localtime(&now);
                if tm.is_null() {
                    std::mem::zeroed()
                } else {
                    *tm
                }
            }
        }
    };
}

lazy_static! {
    static ref PID: u32 = unsafe { libc::getpid() } as _;
    static ref G_MESSAGES_DEBUG: Vec<std::sync::Arc<str>> = std::env::var("G_MESSAGES_DEBUG")
        .unwrap_or_default()
        .split(";")
        .map(|s| std::sync::Arc::<str>::from(s))
        .collect();
}

const F_COL_LIGHT_BLUE: &str = "\x1b[2;34m";
const F_RESET: &str = "\x1b[0m";

#[allow(dead_code)]
enum LogLevel {
    Error,
    Critical,
    Warning,
    Message,
    Info,
    Debug,
}

pub struct Logger;

#[allow(dead_code)]
impl Logger {
    pub fn log_error(domain: &str, args: std::fmt::Arguments<'_>) {
        let color = Self::log_level_to_color(LogLevel::Error);
        let now = now!();
        eprintln!(
            "\n(missioncenter-gatherer:{}): {}-{}{}{} **: {}{}:{}:{}.000{}: {}",
            *PID,
            domain,
            color,
            "ERROR",
            F_RESET,
            F_COL_LIGHT_BLUE,
            now.tm_hour,
            now.tm_min,
            now.tm_sec,
            F_RESET,
            args
        );
    }

    pub fn log_critical(domain: &str, args: std::fmt::Arguments<'_>) {
        let color = Self::log_level_to_color(LogLevel::Critical);
        let now = now!();
        eprintln!(
            "\n(missioncenter-gatherer:{}): {}-{}{}{} **: {}{}:{}:{}.000{}: {}",
            *PID,
            domain,
            color,
            "CRITICAL",
            F_RESET,
            F_COL_LIGHT_BLUE,
            now.tm_hour,
            now.tm_min,
            now.tm_sec,
            F_RESET,
            args
        );
    }

    pub fn log_warn(domain: &str, args: std::fmt::Arguments<'_>) {
        let color = Self::log_level_to_color(LogLevel::Warning);
        let now = now!();
        println!(
            "\n(missioncenter-gatherer:{}): {}-{}{}{} **: {}{}:{}:{}.000{}: {}",
            *PID,
            domain,
            color,
            "WARNING",
            F_RESET,
            F_COL_LIGHT_BLUE,
            now.tm_hour,
            now.tm_min,
            now.tm_sec,
            F_RESET,
            args
        );
    }

    pub fn log_message(domain: &str, args: std::fmt::Arguments<'_>) {
        let color = Self::log_level_to_color(LogLevel::Message);
        let now = now!();
        println!(
            "(missioncenter-gatherer:{}): {}-{}{}{}: {}{}:{}:{}.000{}: {}",
            *PID,
            domain,
            color,
            "MESSAGE",
            F_RESET,
            F_COL_LIGHT_BLUE,
            now.tm_hour,
            now.tm_min,
            now.tm_sec,
            F_RESET,
            args
        );
    }

    pub fn log_info(domain: &str, args: std::fmt::Arguments<'_>) {
        if !G_MESSAGES_DEBUG.is_empty()
            && (!G_MESSAGES_DEBUG.contains(&domain.into())
                && !G_MESSAGES_DEBUG.contains(&"all".into()))
        {
            return;
        }

        let color = Self::log_level_to_color(LogLevel::Info);
        let now = now!();
        println!(
            "(missioncenter-gatherer:{}): {}-{}{}{}: {}{}:{}:{}.000{}: {}\n",
            *PID,
            domain,
            color,
            "INFO",
            F_RESET,
            F_COL_LIGHT_BLUE,
            now.tm_hour,
            now.tm_min,
            now.tm_sec,
            F_RESET,
            args
        );
    }

    pub fn log_debug(domain: &str, args: std::fmt::Arguments<'_>) {
        if !G_MESSAGES_DEBUG.is_empty()
            && (!G_MESSAGES_DEBUG.contains(&domain.into())
                && !G_MESSAGES_DEBUG.contains(&"all".into()))
        {
            return;
        }

        let color = Self::log_level_to_color(LogLevel::Debug);
        let now = now!();
        println!(
            "(missioncenter-gatherer:{}): {}-{}{}{}: {}{}:{}:{}.000{}: {}",
            *PID,
            domain,
            color,
            "INFO",
            F_RESET,
            F_COL_LIGHT_BLUE,
            now.tm_hour,
            now.tm_min,
            now.tm_sec,
            F_RESET,
            args
        );
    }

    const fn log_level_to_color(level: LogLevel) -> &'static str {
        match level {
            LogLevel::Error => "\x1b[1;31m",    /* red */
            LogLevel::Critical => "\x1b[1;35m", /* magenta */
            LogLevel::Warning => "\x1b[1;33m",  /* yellow */
            LogLevel::Message => "\x1b[1;32m",  /* green */
            LogLevel::Info => "\x1b[1;32m",     /* green */
            LogLevel::Debug => "\x1b[1;32m",    /* green */
        }
    }
}
