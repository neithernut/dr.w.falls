//! Error handling and reporting utilities

use std::error::Error;
use std::fmt;


/// Extension trait for `Result` and `Option`
///
/// This extension trait provides some convenience utilities such as functions
/// for reporting.
///
pub trait TryExt: Sized {
    /// Type transported/wrapped by the `Try` type
    ///
    type Output;

    /// Return the wrapped value or log
    ///
    /// If the instance transports a value, this function returns that value
    /// wrapped in an `Option`. Otherwise, the function logs the given `msg`
    /// with the given `level`.
    ///
    fn or_log(self, level: log::Level, msg: &str) -> Option<Self::Output>;

    /// Return the wrapped value or report an error
    ///
    /// Equivalent to `or_log(log::Level::Error, msg)`
    ///
    fn or_err(self, msg: &str) -> Option<Self::Output> {
        self.or_log(log::Level::Error, msg)
    }

    /// Return the wrapped value or warn
    ///
    /// Equivalent to `or_log(log::Level::Warn, msg)`
    ///
    fn or_warn(self, msg: &str) -> Option<Self::Output> {
        self.or_log(log::Level::Warn, msg)
    }

    /// Return the wrapped value or inform
    ///
    /// Equivalent to `or_log(log::Level::Info, msg)`
    ///
    fn or_info(self, msg: &str) -> Option<Self::Output> {
        self.or_log(log::Level::Info, msg)
    }
}

impl<T, E: Error> TryExt for Result<T, E> {
    type Output = T;

    fn or_log(self, level: log::Level, msg: &str) -> Option<Self::Output> {
        if let Err(e) = &self {
            use fmt::Write;

            let mut err_string = msg.to_string();
            let mut err: Option<&dyn Error> = Some(&e);
            while let Some(current) = err {
                let _ = writeln!(err_string, ":  {}", current);
                err = current.source();
            }
            log::log!(level, "{}", err_string);
        };
        self.ok()
    }
}

impl<T> TryExt for Option<T> {
    type Output = T;

    fn or_log(self, level: log::Level, msg: &str) -> Option<Self::Output> {
        if self.is_none() {
            log::log!(level, "{}", msg)
        };
        self
    }
}

