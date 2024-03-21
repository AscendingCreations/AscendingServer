pub trait Shifting {
    type Unsigned;

    fn shift_signed(&self) -> Self::Unsigned;
    fn unshift_signed(input: &Self::Unsigned) -> Self;
}

impl Shifting for i8 {
    type Unsigned = u8;
    fn shift_signed(&self) -> Self::Unsigned {
        if *self >= 0 {
            *self as Self::Unsigned + Self::max_value() as Self::Unsigned + 1
        } else {
            (*self + Self::max_value() + 1) as Self::Unsigned
        }
    }

    fn unshift_signed(input: &Self::Unsigned) -> Self {
        if *input > Self::max_value() as Self::Unsigned {
            (*input - (Self::max_value() as Self::Unsigned + 1)) as Self
        } else {
            (*input as Self) - Self::max_value() - 1
        }
    }
}

impl Shifting for i16 {
    type Unsigned = u16;
    fn shift_signed(&self) -> Self::Unsigned {
        if *self >= 0 {
            *self as Self::Unsigned + Self::max_value() as Self::Unsigned + 1
        } else {
            (*self + Self::max_value() + 1) as Self::Unsigned
        }
    }

    fn unshift_signed(input: &Self::Unsigned) -> Self {
        if *input > Self::max_value() as Self::Unsigned {
            (*input - (Self::max_value() as Self::Unsigned + 1)) as Self
        } else {
            (*input as Self) - Self::max_value() - 1
        }
    }
}

impl Shifting for i32 {
    type Unsigned = u32;
    fn shift_signed(&self) -> Self::Unsigned {
        if *self >= 0 {
            *self as Self::Unsigned + Self::max_value() as Self::Unsigned + 1
        } else {
            (*self + Self::max_value() + 1) as Self::Unsigned
        }
    }

    fn unshift_signed(input: &Self::Unsigned) -> Self {
        if *input > Self::max_value() as Self::Unsigned {
            (*input - (Self::max_value() as Self::Unsigned + 1)) as Self
        } else {
            (*input as Self) - Self::max_value() - 1
        }
    }
}

impl Shifting for i64 {
    type Unsigned = u64;
    fn shift_signed(&self) -> Self::Unsigned {
        if *self >= 0 {
            *self as Self::Unsigned + Self::max_value() as Self::Unsigned + 1
        } else {
            (*self + Self::max_value() + 1) as Self::Unsigned
        }
    }

    fn unshift_signed(input: &Self::Unsigned) -> Self {
        if *input > Self::max_value() as Self::Unsigned {
            (*input - (Self::max_value() as Self::Unsigned + 1)) as Self
        } else {
            (*input as Self) - Self::max_value() - 1
        }
    }
}

impl Shifting for i128 {
    type Unsigned = u128;
    fn shift_signed(&self) -> Self::Unsigned {
        if *self >= 0 {
            *self as Self::Unsigned + Self::max_value() as Self::Unsigned + 1
        } else {
            (*self + Self::max_value() + 1) as Self::Unsigned
        }
    }

    fn unshift_signed(input: &Self::Unsigned) -> Self {
        if *input > Self::max_value() as Self::Unsigned {
            (*input - (Self::max_value() as Self::Unsigned + 1)) as Self
        } else {
            (*input as Self) - Self::max_value() - 1
        }
    }
}
