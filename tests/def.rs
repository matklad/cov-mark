fn safe_divide(dividend: u32, divisor: u32) -> u32 {
    if divisor == 0 {
        cov_mark::hit!(save_divide_zero);
        return 0;
    }
    dividend / divisor
}

struct CoveredDropper;
impl Drop for CoveredDropper {
    fn drop(&mut self) {
        cov_mark::hit!(covered_dropper_drops);
    }
}

#[cfg(test)]
mod group {
    use super::*;

    cov_mark::def!(save_divide_zero);

    #[test]
    fn test_safe_divide_by_zero() {
        cov_mark::chk!(save_divide_zero);
        assert_eq!(safe_divide(92, 0), 0);
    }

    cov_mark::def!(covered_dropper_drops);

    #[test]
    #[cfg(feature = "thread-local")]
    fn test_drop_count() {
        cov_mark::chk_cnt!(covered_dropper_drops, 2);
        let _covered_dropper1 = CoveredDropper;
        let _covered_dropper2 = CoveredDropper;
    }
}
