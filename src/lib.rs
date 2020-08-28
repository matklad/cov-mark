//! # cov-mark
//!
//! This library at its core provides two macros, [`hit!`] and [`check!`],
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
//! * In the presence of threads, [`check!`] may falsely pass, if the
//!   mark is hit by an unrelated thread, unless the `thread-local` feature is
//!   enabled.
//! * Names of marks must be globally unique.
//! * [`check!`] can't be used in integration tests.
//!
//! # Implementation Details
//!
//! Each coverage mark is an `AtomicUsize` counter. [`hit!`] increments
//! this counter, [`check!`] returns a guard object which checks that
//! the mark was incremented. When the `thread-local` feature is enabled,
//! each counter is stored as a thread-local, allowing for more accurate
//! counting.
//!
//! Counters are declared using `#[no_mangle]` attribute, so that [`hit!`] and
//! [`check!`] both can find the mark without the need to declare it in a common
//! module. Aren't the linkers ~~horrible~~ wonderful?
//!
//! # Safety
//!
//! Technically, the [`hit!`] macro in this crate is unsound: it uses `extern "C"
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

#![cfg_attr(nightly_docs, deny(broken_intra_doc_links))]
#![cfg_attr(nightly_docs, feature(doc_cfg))]

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
                static $ident: $crate::__rt::HitCounter;
            }
            unsafe {
                $ident.hit();
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
        $crate::__cov_mark_private_create_mark! { static $ident }
        let _guard = $crate::__rt::Guard::new(&$ident, None);
    };
}

/// Checks that a specified mark was hit exactly the specified number of times.
///
/// # Example
///
/// ```
/// struct CoveredDropper;
/// impl Drop for CoveredDropper {
///     fn drop(&mut self) {
///         cov_mark::hit!(covered_dropper_drops);
///     }
/// }
///
/// #[test]
/// fn drop_count_test() {
///     cov_mark::check_count!(covered_dropper_drops, 2);
///     let _covered_dropper1 = CoveredDropper;
///     let _covered_dropper2 = CoveredDropper;
/// }
/// ```
#[cfg(feature = "thread-local")]
#[cfg_attr(nightly_docs, doc(cfg(feature = "thread-local")))]
#[macro_export]
macro_rules! check_count {
    ($ident:ident, $count: literal) => {
        $crate::__cov_mark_private_create_mark! { static $ident }
        let _guard = $crate::__rt::Guard::new(&$ident, Some($count));
    };
}

#[doc(hidden)]
#[macro_export]
#[cfg(feature = "thread-local")]
macro_rules! __cov_mark_private_create_mark {
    (static $ident:ident) => {
        mod $ident {
            thread_local! {
                #[allow(non_upper_case_globals)]
                pub(super) static $ident: $crate::__rt::AtomicUsize =
                    $crate::__rt::AtomicUsize::new(0);
            }
        }
        #[no_mangle]
        static $ident: $crate::__rt::HitCounter = $crate::__rt::HitCounter::new($ident::$ident);
    };
}

#[doc(hidden)]
#[macro_export]
#[cfg(not(feature = "thread-local"))]
macro_rules! __cov_mark_private_create_mark {
    (static $ident:ident) => {
        #[no_mangle]
        static $ident: $crate::__rt::HitCounter = $crate::__rt::HitCounter::new();
    };
}

#[doc(hidden)]
pub mod __rt {
    pub use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    #[cfg(feature = "thread-local")]
    use std::thread::LocalKey;

    #[cfg(not(feature = "thread-local"))]
    pub struct HitCounter(AtomicUsize);
    #[cfg(feature = "thread-local")]
    pub struct HitCounter(LocalKey<AtomicUsize>);

    #[cfg(not(feature = "thread-local"))]
    impl HitCounter {
        pub const fn new() -> Self {
            Self(AtomicUsize::new(0))
        }
        pub fn hit(&'static self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
        pub fn value(&'static self) -> usize {
            self.0.load(Ordering::Relaxed)
        }
    }

    #[cfg(feature = "thread-local")]
    impl HitCounter {
        pub const fn new(key: LocalKey<AtomicUsize>) -> Self {
            Self(key)
        }
        pub fn hit(&'static self) {
            self.0.with(|v| v.fetch_add(1, Ordering::Relaxed));
        }
        pub fn value(&'static self) -> usize {
            self.0.with(|v| v.load(Ordering::Relaxed))
        }
    }

    pub struct Guard {
        mark: &'static HitCounter,
        value_on_entry: usize,
        expected_hits: Option<usize>,
    }

    impl Guard {
        pub fn new(mark: &'static HitCounter, expected_hits: Option<usize>) -> Guard {
            let value_on_entry = mark.value();
            Guard {
                mark,
                value_on_entry,
                expected_hits,
            }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            if std::thread::panicking() {
                return;
            }
            let value_on_exit = self.mark.value();
            let hit_count = value_on_exit.wrapping_sub(self.value_on_entry);
            match self.expected_hits {
                Some(hits) => assert!(
                    hit_count == hits,
                    "mark was hit {} times, expected {}",
                    hit_count,
                    hits
                ),
                None => assert!(hit_count > 0, "mark was not hit"),
            }
        }
    }
}
