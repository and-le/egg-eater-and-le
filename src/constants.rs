/**
 * Constant values
 */

// Size of a word on our compiler's architecture: x86_64
pub const WORD_SIZE: i64 = 8;
// Number of bits to shift left to convert an index into an offset
pub const WORD_SIZE_SHIFT: i64 = 3;
// Number of bits to shift left to convert a Snek number into a memory offset
pub const SNEK_NUMBER_TO_OFFSET_SHIFT: i64 = 2;
// Number of bits to shift right to convert an offset into a number
pub const OFFSET_TO_NUMBER_SHIFT: i64 = 3;

pub const I63_MIN: i64 = -4611686018427387904;
pub const I63_MAX: i64 = 4611686018427387903;

pub const NIL_VAL: i64 = 1;
pub const FALSE_VAL: i64 = 3;
pub const TRUE_VAL: i64 = 7;
pub const BOOLEAN_LSB: i64 = 0b11;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i64)]
pub enum ErrCode {
    Overflow = 1,
    InvalidType = 2,
    IndexOutOfBounds = 3,
    InvalidVecSize = 4,
}

pub const NUM_OVERFLOW_LABEL: &str = "error_numeric_overflow";
pub const INVALID_TYPE_LABEL: &str = "error_invalid_type";
pub const INDEX_OUT_OF_BOUNDS_LABEL: &str = "error_index_out_of_bounds";
pub const INVALID_VEC_SIZE_LABEL: &str = "error_invalid_vec_size";
