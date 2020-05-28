# cov-mark

Verify that your tests exercise the conditions you think they are exercising

```rust
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
```

See [the docs](https://docs.rs/cov-mark) for details
