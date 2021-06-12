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
        if $crate::__rt::enabled() {
            $crate::__rt::hit(stringify!($ident));
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
        let _guard = $crate::__rt::Guard::new(stringify!($ident), None);
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
#[macro_export]
macro_rules! check_count {
    ($ident:ident, $count: literal) => {
        let _guard = $crate::__rt::Guard::new(stringify!($ident), Some($count));
    };
}

#[doc(hidden)]
pub mod __rt {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    thread_local! {
        static LEVEL: Cell<u32> = Cell::new(0);
        static ACTIVE: RefCell<Vec<Rc<GuardInner>>> = Default::default();
    }

    #[inline]
    pub fn enabled() -> bool {
        LEVEL.with(|it| it.get() > 0)
    }

    #[cold]
    pub fn hit(key: &'static str) {
        ACTIVE.with(|it| it.borrow().iter().for_each(|g| g.hit(key)))
    }

    struct GuardInner {
        mark: &'static str,
        hits: Cell<usize>,
        expected_hits: Option<usize>,
    }

    pub struct Guard {
        inner: Rc<GuardInner>,
    }

    impl GuardInner {
        fn hit(&self, key: &'static str) {
            if key == self.mark {
                self.hits.set(self.hits.get().saturating_add(1))
            }
        }
    }

    impl Guard {
        pub fn new(mark: &'static str, expected_hits: Option<usize>) -> Guard {
            let inner = GuardInner {
                mark,
                hits: Cell::new(0),
                expected_hits,
            };
            let inner = Rc::new(inner);
            LEVEL.with(|it| it.set(it.get() + 1));
            ACTIVE.with(|it| it.borrow_mut().push(Rc::clone(&inner)));
            Guard { inner }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            LEVEL.with(|it| it.set(it.get() - 1));
            let last = ACTIVE.with(|it| it.borrow_mut().pop());

            if std::thread::panicking() {
                return;
            }

            let last = last.unwrap();
            assert!(Rc::ptr_eq(&last, &self.inner));
            let hit_count = last.hits.get();
            match last.expected_hits {
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
