/**
 * Constant values
 */
pub const WORD_SIZE: i64 = 8;

pub const I63_MIN: i64 = -4611686018427387904;
pub const I63_MAX: i64 = 4611686018427387903;

// const NIL_VAL: i64 = 1;
pub const FALSE_VAL: i64 = 3;
pub const TRUE_VAL: i64 = 7;
pub const BOOLEAN_LSB: i64 = 0b11;

pub const ERR_NUM_OVERFLOW: i64 = 1;
pub const NUM_OVERFLOW_LABEL: &str = "error_numeric_overflow";

pub const ERR_INVALID_TYPE: i64 = 2;
pub const INVALID_TYPE_LABEL: &str = "error_invalid_type";
