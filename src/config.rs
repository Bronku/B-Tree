pub const DEGREE: usize = 2;
pub const MAX_KEYS: usize = DEGREE * 2 + 1;
pub const PAGE_SIZE: usize = 1024;

#[cfg(test)]
pub mod test_config {
    use super::*;

    pub const MIN_OPS: usize = 1;
    pub const MAX_OPS: usize = MAX_KEYS * 100;

    pub const MIN_RECORDS: usize = 0;
    pub const MAX_RECORDS: usize = MAX_KEYS * 200;

    pub const PROPTEST_CASES: u32 = 200;
}
