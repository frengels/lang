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

    Identifier,

    IntLit,
    FloatLit,
    CharLit,
    StringLit,
    BoolLit,

    UnterminatedString,
    InvalidNumberSign,
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

fn is_atmosphere_start(ch: u8) -> bool {
    match ch {
        b' ' | b'\t' | b';' => true,
        x => is_newline_start(x),
    }
}

fn is_delimiter(ch: u8) -> bool {
    match ch {
        b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'"' => true,
        x => is_atmosphere_start(x),
    }
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

    pub unsafe fn as_str(&self) -> &str {
        std::str::from_utf8_unchecked(self.iter.as_slice())
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

    fn advance_to_delimiter(mut iter: Iter<u8>) -> *const u8 {
        let mut peek_iter = iter.clone();

        while let Some(ch) = peek_iter.next() {
            if is_delimiter(*ch) {
                break;
            }

            iter = peek_iter.clone();
        }

        iter.as_slice().as_ptr()
    }

    fn scan_identifier_continue(iter: Iter<u8>) -> ScanRes {
        let slice_end = Scanner::advance_to_delimiter(iter);
        ScanRes {
            kind: LexemeKind::Identifier,
            slice_end,
        }
    }

    fn scan_float(mut iter: Iter<u8>) -> ScanRes {
        let mut peek_iter = iter.clone();

        while let Some(ch) = peek_iter.next() {
            if is_delimiter(*ch) {
                break;
            } else if !ch.is_ascii_digit() {
                return Scanner::scan_identifier_continue(peek_iter);
            }

            iter = peek_iter.clone();
        }

        ScanRes {
            kind: LexemeKind::FloatLit,
            slice_end: iter.as_slice().as_ptr(),
        }
    }

    fn scan_number_continue(mut iter: Iter<u8>) -> ScanRes {
        let mut peek_iter = iter.clone();

        while let Some(ch) = peek_iter.next() {
            if !ch.is_ascii_digit() {
                break;
            }

            iter = peek_iter.clone();
        }

        peek_iter = iter.clone();

        if let Some(ch) = peek_iter.next() {
            if *ch == b'.' {
                return Scanner::scan_float(peek_iter);
            } else if !is_delimiter(*ch) {
                return Scanner::scan_identifier_continue(peek_iter);
            }

            // fallthrough for delimiter
        }

        ScanRes {
            kind: LexemeKind::IntLit,
            slice_end: iter.as_slice().as_ptr(),
        }
    }

    fn scan_string(mut iter: Iter<u8>) -> ScanRes {
        let mut escaping = false;

        while let Some(ch) = iter.next() {
            if *ch == b'\\' {
                escaping = true;
                continue;
            }

            if *ch == b'"' && !escaping {
                return ScanRes {
                    kind: LexemeKind::StringLit,
                    slice_end: iter.as_slice().as_ptr(),
                };
            }

            escaping = false;
        }

        ScanRes {
            kind: LexemeKind::UnterminatedString,
            slice_end: iter.as_slice().as_ptr(),
        }
    }

    fn scan_sign(mut iter: Iter<u8>) -> ScanRes {
        let ch = iter.next();

        ch.map_or(
            ScanRes {
                kind: LexemeKind::Identifier,
                slice_end: iter.as_slice().as_ptr(),
            },
            |ch| {
                if ch.is_ascii_digit() {
                    Scanner::scan_number_continue(iter)
                } else if is_delimiter(*ch) {
                    ScanRes {
                        kind: LexemeKind::Identifier,
                        slice_end: iter.as_slice().as_ptr(),
                    }
                } else {
                    Scanner::scan_identifier_continue(iter)
                }
            },
        )
    }

    fn scan_number_sign(iter: Iter<u8>) -> ScanRes {
        let mut peek_iter = iter.clone();

        if let Some(ch) = peek_iter.next() {
            match *ch {
                b't' | b'f' => {
                    let potential_end = peek_iter.as_slice().as_ptr();

                    if let Some(ch) = peek_iter.next() {
                        if !is_delimiter(*ch) {
                            let lex_end = Scanner::advance_to_delimiter(peek_iter);

                            return ScanRes {
                                kind: LexemeKind::InvalidNumberSign,
                                slice_end: lex_end,
                            };
                        }
                    }

                    ScanRes {
                        kind: LexemeKind::BoolLit,
                        slice_end: potential_end,
                    }
                }
                _ => ScanRes {
                    kind: LexemeKind::Poison,
                    slice_end: Scanner::advance_to_delimiter(peek_iter),
                },
            }
        } else {
            ScanRes {
                kind: LexemeKind::Poison,
                slice_end: iter.as_slice().as_ptr(),
            }
        }
    }

    fn _scan_poison(iter: Iter<u8>) -> ScanRes {
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
            b'"' => Scanner::scan_string(iter),
            b'+' | b'-' => Scanner::scan_sign(iter),
            b'#' => Scanner::scan_number_sign(iter),
            x if x.is_ascii_digit() => Scanner::scan_number_continue(iter),
            _ => Scanner::scan_identifier_continue(iter),
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

    #[test]
    fn test_int() {
        let src = "123\n00013432500231";

        let mut scanner = Scanner::new(src);

        assert_eq!(scanner.next().unwrap().kind, LexemeKind::IntLit);
        scanner.next();
        assert_eq!(scanner.next().unwrap().kind, LexemeKind::IntLit);
        assert_eq!(scanner.next(), None);
    }

    #[test]
    fn test_float() {
        let src = "0.1234\n0. 123432.0";

        let mut scanner = Scanner::new(src);

        assert_eq!(scanner.next().unwrap().kind, LexemeKind::FloatLit);
        scanner.next();
        assert_eq!(scanner.next().unwrap().kind, LexemeKind::FloatLit);
        scanner.next();
        assert_eq!(scanner.next().unwrap().kind, LexemeKind::FloatLit);
        assert_eq!(scanner.next(), None);
    }

    #[test]
    fn test_string() {
        let src = "\"hello world\"\"hello \\\"frengels\\\"\"   \"hello unterminated";

        let mut scanner = Scanner::new(src);


        assert_eq!(scanner.next().unwrap().kind, LexemeKind::StringLit);

        assert_eq!(scanner.next().unwrap().kind, LexemeKind::StringLit);
        scanner.next();
        assert_eq!(scanner.next().unwrap().kind, LexemeKind::UnterminatedString);
        assert_eq!(scanner.next(), None);
    }
}
