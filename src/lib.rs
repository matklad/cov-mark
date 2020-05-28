//! # cov-mark
//!
//! This library provides two macros, `cov_mark::hit!` and `cov_mark::check!`,
//! which can be used to verify that a certain test exercises a certain code
//! path.
//!
//! Here's a short example:
//!
//! ```
//! fn parse_date(s: &str) -> Option<(u32, u32, u32)> {
//!     if 10 != s.len() {
//!         // By using `cov_mark::hit!`
//!         // we signal which test exercises this code.
//!         cov_mark::hit!(short_date);
//!         return None;
//!     }
//!
//!     if "-" != &s[4..5] || "-" != &s[7..8] {
//!         cov_mark::hit!(bad_dashes);
//!         return None;
//!     }
//!     // ...
//! #    unimplemented!()
//! }
//!
//! #[test]
//! fn test_parse_date() {
//!     {
//!         // `cov_mark::check!` creates a guard object
//!         // that verifies that by the end of the scope we've
//!         // executed the corresponding `cov_mark::hit`.
//!         cov_mark::check!(short_date);
//!         assert!(parse_date("92").is_none());
//!     }
//!
//! //  This will fail. Although the test looks like
//! //  it exercises the second condition, it does not.
//! //  The call to `covers!` call catches this bug in the test.
//! //  {
//! //      cov_mark::check!(bad_dashes);;
//! //      assert!(parse_date("27.2.2013").is_none());
//! //  }
//!
//!     {
//!         cov_mark::check!(bad_dashes);
//!         assert!(parse_date("27.02.2013").is_none());
//!     }
//! }
//!
//! # fn main() {}
//! ```
//!
//! Here's why coverage marks are useful:
//!
//! * Verifying that something doesn't happen for the *right* reason.
//! * Finding the test that exercises the code (grep for `check!(mark_name)`).
//! * Finding the code that the test is supposed to check (grep for `hit!(mark_name)`).
//! * Making sure that code and tests don't diverge during refactorings.
//! * (If used pervasively) Verifying that each branch has a corresponding test.
//!
//! # Limitations
//!
//! * In the presence of threads, `cov_mark::check!` may falsely pass, if the
//!   mark is hit by an unrelated thread.
//! * Names of marks must be globally unique.
//! * `cov_mark::check!` can't be used in integration tests.
//!
//! # Implementation Details
//!
//! Each coverage mark is an `AtomicUsize` counter. `cov_mark::hit!` increments
//! this counter, `cov_mark::check!` returns a guard object which checks that
//! the mark was incremented.
//!
//! Counters are declared using `#[no_mangle]` attribute, so that `hit!` and
//! `cover!` both can find the mark without the need to declare it in a common
//! module. Aren't the linkers ~~horrible~~ wonderful?
//!
//! # Safety
//!
//! Technically, the `hit!` macro in this crate is unsound: it uses `extern "C"
//! #[no_mangle]` symbol, which *could* clash with an existing symbol and cause
//! UB. For example, `cov_mark::hit!(main)` may segfault. That said:
//!
//! * If there's no existing symbol, the result is a linker error.
//! * If there exists corresponding `cov_mark::check!`, the result is a linker
//!   error.
//! * Code inside `cov_mark::hit!` is hidden under `#[cfg(test)]`.
//!
//! It is believed that it is practically impossible to cause UB by accident
//! when using this crate. For this reason, the `hit` macro hides unsafety
//! inside.

/// Hit a mark with a specified name.
///
/// # Example
///
/// ```
/// fn safe_divide(dividend: u32, divisor: u32) -> u32 {
///     if divisor == 0 {
///         cov_mark::hit!(save_divide_zero);
///         return 0;
///     }
///     dividend / divisor
/// }
/// ```
#[macro_export]
macro_rules! hit {
    ($ident:ident) => {{
        #[cfg(test)]
        {
            extern "C" {
                #[no_mangle]
                static $ident: $crate::__rt::AtomicUsize;
            }
            unsafe {
                $ident.fetch_add(1, $crate::__rt::Ordering::Relaxed);
            }
        }
    }};
}

/// Checks that a specified mark was hit.
///
/// # Example
///
/// ```
/// #[test]
/// fn test_safe_divide_by_zero() {
///     cov_mark::check!(save_divide_zero);
///     assert_eq!(safe_divide(92, 0), 0);
/// }
/// # fn safe_divide(dividend: u32, divisor: u32) -> u32 {
/// #     if divisor == 0 {
/// #         cov_mark::hit!(save_divide_zero);
/// #         return 0;
/// #     }
/// #     dividend / divisor
/// # }
/// ```
#[macro_export]
macro_rules! check {
    ($ident:ident) => {
        #[no_mangle]
        static $ident: $crate::__rt::AtomicUsize = $crate::__rt::AtomicUsize::new(0);
        let _guard = $crate::__rt::Guard::new(&$ident);
    };
}

#[doc(hidden)]
pub mod __rt {
    pub use std::sync::atomic::{AtomicUsize, Ordering};

    pub struct Guard {
        mark: &'static AtomicUsize,
        value_on_entry: usize,
    }

    impl Guard {
        pub fn new(mark: &'static AtomicUsize) -> Guard {
            let value_on_entry = mark.load(Ordering::Relaxed);
            Guard {
                mark,
                value_on_entry,
            }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            if std::thread::panicking() {
                return;
            }
            let value_on_exit = self.mark.load(Ordering::Relaxed);
            assert!(value_on_exit > self.value_on_entry, "mark was not hit")
        }
    }
}
