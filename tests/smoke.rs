fn safe_divide(dividend: u32, divisor: u32) -> u32 {
    if divisor == 0 {
        cov_mark::hit!(save_divide_zero);
        return 0;
    }
    dividend / divisor
}

#[test]
fn test_safe_divide_by_zero() {
    cov_mark::check!(save_divide_zero);
    assert_eq!(safe_divide(92, 0), 0);
}

struct CoveredDropper;
impl Drop for CoveredDropper {
    fn drop(&mut self) {
        cov_mark::hit!(covered_dropper_drops);
    }
}

#[test]
fn test_drop_count() {
    cov_mark::check_count!(covered_dropper_drops, 2);
    let _covered_dropper1 = CoveredDropper;
    let _covered_dropper2 = CoveredDropper;
}
