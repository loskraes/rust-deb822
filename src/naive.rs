use std::collections::BTreeMap as Map;
use std::iter::Peekable;

use icu_casemap::CaseMapper;
use itertools::Itertools;

use crate::error::Error;

type IterInnerBox<'a> =
    dyn Iterator<Item = Result<Option<(usize, String, Vec<&'a str>)>, Error>> + 'a;
pub type Iter<'a> = Peekable<Box<IterInnerBox<'a>>>;

pub fn iter_from_str(s: &str) -> Iter {
    let case_mapper = CaseMapper::new();
    let b: Box<IterInnerBox> = Box::new(
        s.lines()
            .enumerate()
            // Ignore empty line at begin of file
            .skip_while(|(_, s)| s.trim().is_empty())
            // Ignore comment
            .filter(|(_, s)| !s.starts_with('#'))
            .peekable()
            // Ignore multiple consecutive void line
            .batching(|it| {
                let (line_no, next) = it.next()?;
                if next.trim().is_empty() {
                    loop {
                        match it.peek() {
                            None => break,
                            Some((_, s)) if s.trim().is_empty() => {
                                it.next();
                                continue;
                            }
                            Some(_) => break,
                        }
                    }
                    Some(None)
                } else {
                    Some(Some((line_no, next)))
                }
            })
            .peekable()
            .batching(move |it| {
                let (line_no, line) = match it.next()? {
                    None => return Some(Ok(None)),
                    Some(i) => i,
                };
                dbg!(line);
                let (key, value) = match line.split_once(':') {
                    Some(d) => d,
                    None => return Some(Err(Error::MissingColon(line_no))),
                };
                let key = case_mapper.fold_string(key.trim());
                let value = value.strip_prefix(' ').unwrap_or(value);
                let mut data = vec![value];
                data.extend(
                    it.peeking_take_while(|i| matches!(i, Some((_, s)) if s.starts_with(' ')))
                        .map(|i| &i.unwrap().1[1..]),
                );

                Some(Ok(Some((line_no, key, data))))
            }),
    );
    b.peekable()
}
pub fn from_str(s: &str) -> Result<Vec<Map<String, String>>, String> {
    let case_mapper = CaseMapper::new();
    s.lines()
        .enumerate()
        // Ignore empty line at begin of file
        .skip_while(|(_, s)| s.trim().is_empty())
        // Ignore comment
        .filter(|(_, s)| !s.starts_with('#'))
        .peekable()
        // Ignore multiple consecutive void line
        .batching(|it| {
            let (line, next) = it.next()?;
            if next.trim().is_empty() {
                loop {
                    match it.peek() {
                        None => break,
                        Some((_, s)) if s.trim().is_empty() => {
                            it.next();
                            continue;
                        }
                        Some(_) => break,
                    }
                }
                Some((line, ""))
            } else {
                Some((line, next))
            }
        })
        .batching(|it| {
            let stanza: Result<Map<_, _>, _> = it
                .take_while(|(_, s)| !s.is_empty())
                .peekable()
                .batching(|it| {
                    let (line, s) = it.next()?;
                    let (key, value) = match s.split_once(':') {
                        Some(d) => d,
                        None => return Some(Err(format!("Missing colon at line {line}"))),
                    };
                    let key = case_mapper.fold_string(key.trim());
                    let value = value.strip_prefix(' ').unwrap_or(value);
                    let mut data = vec![value];
                    loop {
                        match it.peek() {
                            None => break,
                            Some((_, s)) if s.starts_with(' ') => {
                                data.push(&s[1..]);
                                it.next();
                            }
                            Some(_) => break,
                        }
                    }
                    Some(Ok((line, key, data)))
                })
                .map(|r| {
                    let (_line, key, values) = match r {
                        Ok(ok) => ok,
                        Err(err) => return Err(err),
                    };
                    let mut value = values.into_iter().peekable();
                    if value.peek() == Some(&"") {
                        value.next();
                    }

                    Ok((key, value.join("\n")))
                })
                .collect();
            match stanza {
                Ok(stanza) if stanza.is_empty() => None,
                other => Some(other),
            }
        })
        .collect()
}

#[test]
fn a() {
    let s = r#"



Origin: Debian
Architectures: all amd64 arm64 armel armhf i386 mips64el ppc64el riscv64 s390x
Components: main contrib non-free-firmware non-free
Description: Experimental packages - not released; use at your own risk.
MD5Sum:
 3cc222d6694b2de9734c081122a17cb3  3030586 contrib/Contents-all
 1f7d9d3e63b59533f6f5dadc83e71cc7    63339 contrib/Contents-all.diff/Index
 aa5dc8f6f4ab68b4e5b76df04a0532c4   291019 contrib/Contents-all.gz
 55a5553654b03c6a75cd61f79a31257e   271634 contrib/Contents-amd64
 ed5005daa6257830e623e78691c29475    63339 contrib/Contents-amd64.diff/Index


Origin: Debian
Architectures: all amd64 arm64 armel armhf i386 mips64el ppc64el riscv64 s390x
Components: main contrib non-free-firmware non-free
Description: Experimental packages - not released; use at your own risk.
MD5Sum:
 3cc222d6694b2de9734c081122a17cb3  3030586 contrib/Contents-all
 1f7d9d3e63b59533f6f5dadc83e71cc7    63339 contrib/Contents-all.diff/Index
 aa5dc8f6f4ab68b4e5b76df04a0532c4   291019 contrib/Contents-all.gz
 55a5553654b03c6a75cd61f79a31257e   271634 contrib/Contents-amd64
 ed5005daa6257830e623e78691c29475    63339 contrib/Contents-amd64.diff/Index

Origin: Debian
Architectures: all amd64 arm64 armel armhf i386 mips64el ppc64el riscv64 s390x
Components: main contrib non-free-firmware non-free
Description: Experimental packages - not released; use at your own risk.
MD5Sum:
 3cc222d6694b2de9734c081122a17cb3  3030586 contrib/Contents-all
 1f7d9d3e63b59533f6f5dadc83e71cc7    63339 contrib/Contents-all.diff/Index
 aa5dc8f6f4ab68b4e5b76df04a0532c4   291019 contrib/Contents-all.gz
 55a5553654b03c6a75cd61f79a31257e   271634 contrib/Contents-amd64
 ed5005daa6257830e623e78691c29475    63339 contrib/Contents-amd64.diff/Index

"#;
    let h = from_str(s);
    dbg!(h.unwrap());
}
