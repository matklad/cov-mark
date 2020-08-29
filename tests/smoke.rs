fn safe_divide(dividend: u32, divisor: u32) -> u32 {
    if divisor == 0 {
        cov_mark::hit!(save_divide_zero);
        return 0;
    }
    cov_mark::hit!(divide_ok);
    dividend / divisor
}

#[test]
fn test_safe_divide_by_zero() {
    cov_mark::check!(save_divide_zero);
    assert_eq!(safe_divide(92, 0), 0);
}

cov_mark::define!(divide_ok);

#[test]
fn test_division_ok_first() {
    cov_mark::check!(defined divide_ok);
    assert_eq!(safe_divide(10, 1), 10);
}

#[test]
fn test_division_ok_second() {
    cov_mark::check!(defined divide_ok);
    assert_eq!(safe_divide(20, 2), 10);
}

#[test]
#[cfg(feature = "thread-local")]
fn test_division_twice() {
    cov_mark::check_count!(defined divide_ok, 2);
    safe_divide(10, 1);
    safe_divide(20, 2);
}

struct CoveredDropper;
impl Drop for CoveredDropper {
    fn drop(&mut self) {
        cov_mark::hit!(covered_dropper_drops);
    }
}

#[test]
#[cfg(feature = "thread-local")]
fn test_drop_count() {
    cov_mark::check_count!(covered_dropper_drops, 2);
    let _covered_dropper1 = CoveredDropper;
    let _covered_dropper2 = CoveredDropper;
}
