//! Kernel print utilities.

use crate::dev::x86_64::serial::Serial;
use core::fmt::Write;
use spin_lock::SpinLock;

static SERIAL: SpinLock<Serial> = SpinLock::new(Serial::new());

#[doc(hidden)]
#[no_mangle]
pub fn _print(fmt: core::fmt::Arguments<'_>) {
    let _ = write!(&mut *SERIAL.lock(), "{}", fmt);
}

/// Prints out the message.
///
/// Use the format! syntax to write data to the standard output.
/// This first holds the lock for console device.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::kprint::_print(format_args!($($arg)*)));
}

/// Prints out the message with a newline.
///
/// Use the format! syntax to write data to the standard output.
/// This first holds the lock for console device.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Display an information message.
///
/// Use the format! syntax to write data to the standard output.
/// This first holds the lock for console device.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::kprint::_print(
            format_args!(
                "[INFO] {}\n",
                format_args!($($arg)*)
            )
        )
    );
}

/// Display a warning message.
///
/// Use the format! syntax to write data to the standard output.
/// This first holds the lock for console device.
#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => ($crate::kprint::_print(
            format_args!(
                "[WARNING] {}\n",
                format_args!($($arg)*)
            )
        )
    );
}

/// Print msg if debug build
#[macro_export]
macro_rules! debug {
    ($($e:tt)*) => {
        if cfg!(debug_assertions) {
            $crate::kprint::_print(
                format_args!(
                    "[DEBUG] {}\n",
                    format_args!($($arg)*)
                )
            )
        }
    }
}
