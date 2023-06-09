/**
 * Constant values
 */
pub const WORD_SIZE: i64 = 8;
pub const WORD_SIZE_SHIFT: i64 = 3;
pub const SNEK_NUMBER_TO_OFFSET_SHIFT: i64 = 2;
pub const OFFSET_TO_NUMBER_SHIFT: i64 = 3;

pub const I63_MIN: i64 = -4611686018427387904;
pub const I63_MAX: i64 = 4611686018427387903;

pub const NIL_VAL: i64 = 1;
pub const FALSE_VAL: i64 = 3;
pub const TRUE_VAL: i64 = 7;
pub const BOOLEAN_LSB: i64 = 0b11;

pub const ERR_NUM_OVERFLOW: i64 = 1;
pub const NUM_OVERFLOW_LABEL: &str = "error_numeric_overflow";

pub const ERR_INVALID_TYPE: i64 = 2;
pub const INVALID_TYPE_LABEL: &str = "error_invalid_type";

pub const ERR_INDEX_OUT_OF_BOUNDS: i64 = 3;
pub const INDEX_OUT_OF_BOUNDS_LABEL: &str = "error_index_out_of_bounds";

pub const ERR_NOT_VEC: i64 = 4;
pub const NOT_VEC_LABEL: &str = "error_not_vec";

pub const ERR_NOT_INDEX_OFFSET: i64 = 5;
pub const NOT_INDEX_OFFSET_LABEL: &str = "error_not_index_offset";

pub const ERR_INVALID_VEC_SIZE: i64 = 6;
pub const INVALID_VEC_SIZE_LABEL: &str = "error_invalid_vec_size";
