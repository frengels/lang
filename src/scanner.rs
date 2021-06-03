use core::str;
use std::slice::Iter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexemeKind {
    Whitespace,
    Tab,
    NewlineLf,
    NewlineCr,
    NewlineCrlf,
    Comment,

    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    Poison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lexeme<'a> {
    pub kind: LexemeKind,
    pub slice: &'a str,
}

fn is_newline_start(ch: u8) -> bool {
    ch == b'\r' || ch == b'\n'
}

pub struct ScanRes {
    kind: LexemeKind,
    slice_end: *const u8,
}

#[derive(Debug)]
pub struct Scanner<'a> {
    iter: Iter<'a, u8>,
}

impl<'a> Scanner<'a> {
    pub fn new(src: &'a str) -> Self {
        Scanner {
            iter: src.as_bytes().iter(),
        }
    }

    fn scan_whitespace(mut iter: Iter<u8>) -> ScanRes {
        let mut peek_iter = iter.clone();
        while let Some(ch) = peek_iter.next() {
            if *ch != b' ' {
                break;
            }

            iter = peek_iter.clone();
        }

        ScanRes {
            kind: LexemeKind::Whitespace,
            slice_end: iter.as_slice().as_ptr(),
        }
    }

    fn scan_tab(mut iter: Iter<u8>) -> ScanRes {
        let mut peek_iter = iter.clone();
        while let Some(ch) = peek_iter.next() {
            if *ch != b'\t' {
                break;
            }

            iter = peek_iter.clone();
        }

        ScanRes {
            kind: LexemeKind::Tab,
            slice_end: iter.as_slice().as_ptr(),
        }
    }

    fn scan_cr(iter: Iter<u8>) -> ScanRes {
        let mut peek_iter = iter.clone();

        if let Some(ch) = peek_iter.next() {
            if *ch == b'\n' {
                return ScanRes {
                    kind: LexemeKind::NewlineCrlf,
                    slice_end: peek_iter.as_slice().as_ptr(),
                };
            }
        }

        ScanRes {
            kind: LexemeKind::NewlineCr,
            slice_end: iter.as_slice().as_ptr(),
        }
    }

    fn scan_comment(mut iter: Iter<u8>) -> ScanRes {
        let mut peek_iter = iter.clone();

        while let Some(ch) = peek_iter.next() {
            if is_newline_start(*ch) {
                break;
            }

            iter = peek_iter.clone();
        }

        ScanRes {
            kind: LexemeKind::Comment,
            slice_end: iter.as_slice().as_ptr(),
        }
    }

    fn scan_poison(iter: Iter<u8>) -> ScanRes {
        ScanRes {
            kind: LexemeKind::Poison,
            slice_end: iter.as_slice().as_ptr(),
        }
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Lexeme<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut iter = self.iter.clone();

        let ch = iter.next()?;

        let res = match *ch {
            b' ' => Scanner::scan_whitespace(iter),
            b'\t' => Scanner::scan_tab(iter),
            b'\r' => Scanner::scan_cr(iter),
            b';' => Scanner::scan_comment(iter),
            b'\n' => ScanRes {
                kind: LexemeKind::NewlineLf,
                slice_end: iter.as_slice().as_ptr(),
            },
            b'(' => ScanRes {
                kind: LexemeKind::LParen,
                slice_end: iter.as_slice().as_ptr(),
            },
            b')' => ScanRes {
                kind: LexemeKind::RParen,
                slice_end: iter.as_slice().as_ptr(),
            },
            b'[' => ScanRes {
                kind: LexemeKind::LBracket,
                slice_end: iter.as_slice().as_ptr(),
            },
            b']' => ScanRes {
                kind: LexemeKind::RBracket,
                slice_end: iter.as_slice().as_ptr(),
            },
            b'{' => ScanRes {
                kind: LexemeKind::LBrace,
                slice_end: iter.as_slice().as_ptr(),
            },
            b'}' => ScanRes {
                kind: LexemeKind::RBrace,
                slice_end: iter.as_slice().as_ptr(),
            },
            _ => Scanner::scan_poison(iter),
        };

        let ptrs = self.iter.as_slice().as_ptr_range();
        let len = unsafe { res.slice_end.offset_from(ptrs.start) };

        let lexeme_bytes: &'a [u8] =
            unsafe { std::slice::from_raw_parts(ptrs.start, len as usize) };
        let lexeme_str = unsafe { std::str::from_utf8_unchecked(lexeme_bytes) };

        self.iter = unsafe {
            std::slice::from_raw_parts(
                res.slice_end,
                self.iter.as_slice().len() - lexeme_bytes.len(),
            )
        }
        .iter();

        Some(Lexeme {
            kind: res.kind,
            slice: lexeme_str,
        })
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_whitespace() {
        let src = "      ";

        let mut scanner = Scanner::new(src);

        let lex = scanner.next().unwrap();
        assert_eq!(lex.slice.len(), 6);
        assert_eq!(scanner.next(), None);
    }

    #[test]
    fn test_tab() {
        let src = "\t\t\t\t\t\t";

        let mut scanner = Scanner::new(src);

        let lex = scanner.next().unwrap();
        assert_eq!(lex.slice.len(), 6);
        assert_eq!(scanner.next(), None);
    }

    #[test]
    fn test_newline() {
        let src = "\n\r\r\n";

        let mut scanner = Scanner::new(src);

        assert_eq!(scanner.next().unwrap().kind, LexemeKind::NewlineLf);
        assert_eq!(scanner.next().unwrap().kind, LexemeKind::NewlineCr);
        assert_eq!(scanner.next().unwrap().kind, LexemeKind::NewlineCrlf);
        assert_eq!(scanner.next(), None);
    }

    #[test]
    fn test_comment() {
        let src = "\n ; hello world\r";

        let mut scanner = Scanner::new(src);

        assert_eq!(scanner.next().unwrap().kind, LexemeKind::NewlineLf);
        assert_eq!(scanner.next().unwrap().kind, LexemeKind::Whitespace);
        let lex = scanner.next().unwrap();
        assert_eq!(lex.slice.len(), 13);
        assert_eq!(lex.kind, LexemeKind::Comment);
        assert_eq!(scanner.next().unwrap().kind, LexemeKind::NewlineCr);
    }
}
