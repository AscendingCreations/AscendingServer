//This trait allows us to Shift an unsigned variable into a Signed variable with no loss of data.
//it allows us to store unsigned data into a Database that might only support signed types.
pub trait Shifting {
    type Unsigned;

    fn shift_signed(&self) -> Self::Unsigned;
    fn unshift_signed(input: &Self::Unsigned) -> Self;
}

impl Shifting for i8 {
    type Unsigned = u8;
    fn shift_signed(&self) -> Self::Unsigned {
        if *self >= 0 {
            *self as Self::Unsigned + Self::MAX as Self::Unsigned + 1
        } else {
            (*self + Self::MAX + 1) as Self::Unsigned
        }
    }

    fn unshift_signed(input: &Self::Unsigned) -> Self {
        if *input > Self::MAX as Self::Unsigned {
            (*input - (Self::MAX as Self::Unsigned + 1)) as Self
        } else {
            (*input as Self) - Self::MAX - 1
        }
    }
}

impl Shifting for i16 {
    type Unsigned = u16;
    fn shift_signed(&self) -> Self::Unsigned {
        if *self >= 0 {
            *self as Self::Unsigned + Self::MAX as Self::Unsigned + 1
        } else {
            (*self + Self::MAX + 1) as Self::Unsigned
        }
    }

    fn unshift_signed(input: &Self::Unsigned) -> Self {
        if *input > Self::MAX as Self::Unsigned {
            (*input - (Self::MAX as Self::Unsigned + 1)) as Self
        } else {
            (*input as Self) - Self::MAX - 1
        }
    }
}

impl Shifting for i32 {
    type Unsigned = u32;
    fn shift_signed(&self) -> Self::Unsigned {
        if *self >= 0 {
            *self as Self::Unsigned + Self::MAX as Self::Unsigned + 1
        } else {
            (*self + Self::MAX + 1) as Self::Unsigned
        }
    }

    fn unshift_signed(input: &Self::Unsigned) -> Self {
        if *input > Self::MAX as Self::Unsigned {
            (*input - (Self::MAX as Self::Unsigned + 1)) as Self
        } else {
            (*input as Self) - Self::MAX - 1
        }
    }
}

impl Shifting for i64 {
    type Unsigned = u64;
    fn shift_signed(&self) -> Self::Unsigned {
        if *self >= 0 {
            *self as Self::Unsigned + Self::MAX as Self::Unsigned + 1
        } else {
            (*self + Self::MAX + 1) as Self::Unsigned
        }
    }

    fn unshift_signed(input: &Self::Unsigned) -> Self {
        if *input > Self::MAX as Self::Unsigned {
            (*input - (Self::MAX as Self::Unsigned + 1)) as Self
        } else {
            (*input as Self) - Self::MAX - 1
        }
    }
}

impl Shifting for i128 {
    type Unsigned = u128;
    fn shift_signed(&self) -> Self::Unsigned {
        if *self >= 0 {
            *self as Self::Unsigned + Self::MAX as Self::Unsigned + 1
        } else {
            (*self + Self::MAX + 1) as Self::Unsigned
        }
    }

    fn unshift_signed(input: &Self::Unsigned) -> Self {
        if *input > Self::MAX as Self::Unsigned {
            (*input - (Self::MAX as Self::Unsigned + 1)) as Self
        } else {
            (*input as Self) - Self::MAX - 1
        }
    }
}
