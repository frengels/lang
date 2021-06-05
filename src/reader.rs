use std::num::NonZeroU32;

use crate::stx::Stx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reader<'a> {
    line: NonZeroU32,
    column: u32,
    src_bytes: &'a[u8],
}

impl<'a> Reader<'a> {
    pub fn from_slice(src_bytes: &'a[u8]) -> Self {
        Reader {
            line: unsafe {NonZeroU32::new_unchecked(1)},
            column: 0,
            src_bytes
        }
    }

    pub fn read_one() -> Option<Stx>
    {
        None
    }
}
