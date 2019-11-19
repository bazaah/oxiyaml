use super::*;

macro_rules! make_local {
    ($val:ident) => {
        macro_rules! discard_and {
            ( $after:expr ) => {{
                $val.discard();
                $after
            }};
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Marker {
    // Base
    Start,
    WhiteSpace,
    Ignore,
    LineEnd,
    LineStart,
    Done,

    // Map
    MapStart,
    MapKey,
    MapDelimiter,
    MapWhiteSpace,
    MapValue,
    MapEnd,
}

/* Base */
#[derive(Debug, Default)]
pub(super) struct Start;

impl Start {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Marker {
        match iter.peak() {
            Some(_) => Marker::LineStart,
            None => Marker::Done,
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct WhiteSpace;

impl WhiteSpace {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Marker {
        match iter.peak() {
            Some(b'\n') | Some(b'\r') => Marker::LineEnd,
            Some(_txt) => Marker::Ignore,
            None => Marker::Done,
        }
    }

    pub(super) fn skip_whitespace(&self, iter: &mut Scan<impl Iterator<Item = u8>>) {
        loop {
            match iter.peak() {
                Some(b' ') | Some(b'\t') => iter.discard(),
                _ => break,
            }
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Ignore;

impl Ignore {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Result<Marker> {
        match iter.peak() {
            Some(b'\n') | Some(b'\r') => Ok(Marker::LineEnd),
            Some(b' ') | Some(b'\t') => Ok(Marker::WhiteSpace),
            None => Ok(Marker::Done),
            Some(err) => Err(ErrorKind::InvalidChar.with_context(err))?,
        }
    }

    pub(super) fn skip_til_whitespace(&self, iter: &mut Scan<impl Iterator<Item = u8>>) {
        loop {
            match iter.peak() {
                Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => break,
                None => break,
                _ => iter.discard(),
            }
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct LineStart;

impl LineStart {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>, Active>) -> Marker {
        match iter.peak() {
            Some(b'\n') | Some(b'\r') => Marker::LineEnd,
            Some(_) => Marker::Ignore,
            None => Marker::Done,
        }
    }

    pub(super) fn update_indent(&self, iter: &mut Scan<impl Iterator<Item = u8>, Active>) {
        let indent = self.count_indent(iter);
        iter.update_indent(indent);
    }

    fn count_indent(&self, iter: &mut Scan<impl Iterator<Item = u8>, Active>) -> u16 {
        make_local!(iter);
        let mut count = 0;
        loop {
            match iter.peak() {
                Some(b' ') | Some(b'\t') => discard_and!(count += 1),
                _ => break,
            }
        }

        count
    }
}

#[derive(Debug, Default)]
pub(super) struct LineEnd;

impl LineEnd {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Result<Marker> {
        match iter.peak() {
            Some(b' ') | Some(b'\t') | Some(b'a'..=b'z') | Some(b'A'..=b'Z') => {
                Ok(Marker::LineStart)
            }
            None => Ok(Marker::Done),
            Some(err) => Err(ErrorKind::InvalidChar.with_context(err))?,
        }
    }

    pub(super) fn close_line(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Result<()> {
        make_local!(iter);
        loop {
            match iter.peak() {
                Some(b'\n') => discard_and!(break Ok(())),
                Some(b'\r') => discard_and!(match iter.peak() {
                    Some(b'\n') => discard_and!(break Ok(())),
                    _ => break Err(ErrorKind::SoloCarriageReturn)?,
                }),
                None => break Err(ErrorKind::InvalidEOL.with_context(b'\n'))?,
                _ => iter.discard(),
            }
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Done;

#[derive(Debug, Default)]
pub(super) struct Failure;

/* Map */

#[derive(Debug)]
pub(super) struct MapStart {
    indent_floor: u16,
    scratch: Vec<u8>,
}

impl MapStart {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Result<Marker> {
        match iter.peak() {
            Some(_) => Ok(Marker::MapKey),
            None => Err(ErrorKind::EOFMapping.into()),
        }
    }

    pub(super) fn new(current_floor: u16) -> Self {
        Self {
            indent_floor: current_floor,
            scratch: Default::default(),
        }
    }

    pub(super) fn extend_from(current_floor: u16, buffer: Vec<u8>) -> Self {
        Self {
            indent_floor: current_floor,
            scratch: buffer,
        }
    }
}

impl From<MapStart> for MapKey {
    fn from(prev: MapStart) -> Self {
        Self {
            indent_floor: prev.indent_floor,
            scratch: prev.scratch,
        }
    }
}

#[derive(Debug)]
pub(super) struct MapKey {
    indent_floor: u16,
    scratch: Vec<u8>,
}

impl MapKey {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Result<Marker> {
        match iter.peak() {
            Some(b':') => Ok(Marker::MapDelimiter),
            Some(err) => Err(ErrorKind::InvalidChar.with_context(([b':'], err)))?,
            None => Err(ErrorKind::EOFMapping)?,
        }
    }

    pub(super) fn parse_key(&mut self) -> Result<()> {
        while let Some(ch) = self.scratch.iter().next() {
            match ch {
                b'a'..=b'z' | b'A'..=b'Z' | b' ' | b'\t' => (),
                err => return Err(ErrorKind::InvalidChar.with_context(*err))?,
            }
        }

        while !self.scratch.is_empty() && is_whitespace(self.scratch.last().unwrap()) {
            self.scratch.pop();
        }

        Ok(())
    }
}

impl From<MapKey> for MapDelimiter {
    fn from(prev: MapKey) -> Self {
        Self {
            indent_floor: prev.indent_floor,
            key: prev.scratch,
        }
    }
}

#[derive(Debug)]
pub(super) struct MapDelimiter {
    indent_floor: u16,
    key: Vec<u8>,
}

impl MapDelimiter {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Result<Marker> {
        match iter.peak() {
            Some(b' ') | Some(b'\t') => Ok(Marker::MapWhiteSpace),
            Some(b'a'..=b'z') | Some(b'A'..=b'Z') => Ok(Marker::MapValue),
            Some(err) => Err(ErrorKind::InvalidChar.with_context(err))?,
            None => Err(ErrorKind::EOFMapping)?,
        }
    }

    pub(super) fn parse_delimiter(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Result<()> {
        make_local!(iter);

        match iter.peak() {
            Some(b':') => discard_and!(match iter.peak() {
                Some(b' ') | Some(b'\t') => discard_and!(Ok(())),
                Some(err) => Err(ErrorKind::InvalidChar.with_context((&[b' ', b'\t'], err)))?,
                None => Err(ErrorKind::EOFMapping)?,
            }),
            Some(err) => Err(ErrorKind::InvalidChar.with_context((&[b':'], err)))?,
            None => Err(ErrorKind::EOFMapping)?,
        }
    }
}

impl From<MapDelimiter> for MapWhiteSpace {
    fn from(prev: MapDelimiter) -> Self {
        Self {
            indent_floor: prev.indent_floor,
            key: prev.key,
        }
    }
}

#[derive(Debug)]
pub(super) struct MapWhiteSpace {
    indent_floor: u16,
    key: Vec<u8>,
}

impl MapWhiteSpace {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Result<Marker> {
        match iter.peak() {
            Some(b'a'..=b'z') | Some(b'A'..=b'Z') => Ok(Marker::MapValue),
            _ => Err(ErrorKind::Message("Unclear fail state".into()))?,
        }
    }

    pub(super) fn parse_whitespace(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Result<()> {
        loop {
            match iter.peak() {
                Some(b' ') | Some(b'\t') => iter.discard(),
                Some(b'a'..=b'z') | Some(b'A'..=b'Z') => break Ok(()),
                Some(b'|') | Some(b'>') => unimplemented!("Can't parse block/flow indicators!"),
                Some(b'\n') | Some(b'\r') => unimplemented!("Can't parse non-plain scalar keys!"),
                Some(err) => break Err(ErrorKind::InvalidChar.with_context(err))?,
                None => unimplemented!("Can't parse implicit (due to EOF) null values!"),
            }
        }
    }
}

impl From<MapWhiteSpace> for MapValue {
    fn from(prev: MapWhiteSpace) -> Self {
        Self {
            indent_floor: prev.indent_floor,
            key: prev.key,
            value: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(super) struct MapValue {
    indent_floor: u16,
    key: Vec<u8>,
    value: Vec<u8>,
}

impl MapValue {
    pub(super) fn parse_value(&mut self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Result<()> {
        make_local!(iter);

        loop {
            match iter.peak() {
                Some(ch @ b' ')
                | Some(ch @ b'\t')
                | Some(ch @ b'a'..=b'z')
                | Some(ch @ b'A'..=b'Z') => discard_and!(self.value.push(ch)),
                Some(b'\n') | Some(b'\r') | None => break Ok(()),
                Some(err) => break Err(ErrorKind::ScalarInvalid.with_context(err))?,
            }
        }
    }
}

#[derive(Debug)]
pub(super) struct MapEnd;

fn is_whitespace(c: &u8) -> bool {
    *c == b'\t' || *c == b' '
}
