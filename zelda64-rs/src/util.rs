use std::convert::TryInto;
use std::ops::Range;

pub fn to_signed_hex(n: isize) -> String {
    if n < 0 {
        format!("-{:X}", -n)
    } else {
        format!("{:X}", n)
    }
}

pub trait ConvertRangeExt {
    fn to_usize(&self) -> Range<usize>;
}

impl ConvertRangeExt for Range<u32> {
    fn to_usize(&self) -> Range<usize> {
        let start: usize = (self.start as usize).try_into().unwrap();
        let end: usize = (self.end as usize).try_into().unwrap();
        Range { start, end }
    }
}
