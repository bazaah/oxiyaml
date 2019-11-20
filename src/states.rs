use super::{
    error::{Error, ErrorKind, Result},
    scanner::*,
};

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
    //WhiteSpace,
    //Ignore,
    LineEnd,
    LineStart,
    Done,

    // Ambiguous
    AmbiguousScalar,
    AmbiguousColon,

    // Map
    MapStart,
    MapVerifyKey,
    MapWhiteSpace,
    MapValue,
    MapEnd,

    // Scalar
    ScalarLiteral,
}

/* Base */
#[derive(Debug, Default)]
pub(super) struct Start;

impl Start {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = Byte>>) -> Result<Marker> {
        match iter.peak()? {
            Some(_) => Ok(Marker::LineStart),
            None => Ok(Marker::Done),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct AmbiguousScalar {
    scratch: Vec<u8>,
}

impl AmbiguousScalar {
    pub(super) fn find_next(
        &mut self,
        iter: &mut Scan<impl Iterator<Item = Byte>>,
    ) -> Result<Marker> {
        make_local!(iter);

        loop {
            match iter.peak()? {
                Some(ch @ b' ')
                | Some(ch @ b'\t')
                | Some(ch @ b'a'..=b'z')
                | Some(ch @ b'A'..=b'Z') => discard_and!(self.scratch.push(ch)),
                Some(b':') => break Ok(Marker::AmbiguousColon),
                Some(err) => Err(ErrorKind::InvalidChar.with_context(err))?,
                None => break Ok(Marker::ScalarLiteral),
            }
        }
    }
}

impl From<AmbiguousScalar> for AmbiguousColon {
    fn from(prev: AmbiguousScalar) -> Self {
        Self {
            scratch: prev.scratch,
        }
    }
}

#[derive(Debug)]
pub(super) struct AmbiguousColon {
    pub scratch: Vec<u8>,
}

impl AmbiguousColon {
    pub(super) fn find_next(
        &mut self,
        iter: &mut Scan<impl Iterator<Item = Byte>>,
    ) -> Result<Marker> {
        make_local!(iter);

        match iter.peak()? {
            Some(b':') => discard_and!(match iter.peak()? {
                Some(b' ') | Some(b'\t') => discard_and!(Ok(Marker::MapStart)),
                Some(b'a'..=b'z') | Some(b'A'..=b'Z') => Ok(Marker::AmbiguousScalar),
                Some(err) => Err(ErrorKind::InvalidChar.with_context(err))?,
                None => Err(ErrorKind::InvalidEOF)?,
            }),
            Some(err) => Err(ErrorKind::InvalidChar.with_context(([b':'], err)))?,
            None => Err(ErrorKind::InvalidEOF)?,
        }
    }
}

impl From<AmbiguousColon> for AmbiguousScalar {
    fn from(prev: AmbiguousColon) -> Self {
        Self {
            scratch: prev.scratch,
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct LineStart;

impl LineStart {
    pub(super) fn find_next(
        &self,
        iter: &mut Scan<impl Iterator<Item = Byte>, Active>,
    ) -> Result<Marker> {
        match iter.peak()? {
            Some(b'\n') | Some(b'\r') => Ok(Marker::LineEnd),
            Some(_) => Ok(Marker::AmbiguousScalar),
            None => Ok(Marker::Done),
        }
    }

    pub(super) fn update_indent(
        &self,
        iter: &mut Scan<impl Iterator<Item = Byte>, Active>,
    ) -> Result<()> {
        let indent = self.count_indent(iter)?;
        iter.update_indent(indent);

        Ok(())
    }

    fn count_indent(&self, iter: &mut Scan<impl Iterator<Item = Byte>, Active>) -> Result<u16> {
        make_local!(iter);
        let mut count = 0;
        loop {
            match iter.peak()? {
                Some(b' ') | Some(b'\t') => discard_and!(count += 1),
                _ => break,
            }
        }

        Ok(count)
    }
}

#[derive(Debug, Default)]
pub(super) struct LineEnd;

impl LineEnd {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = Byte>>) -> Result<Marker> {
        match iter.peak()? {
            Some(b' ') | Some(b'\t') | Some(b'a'..=b'z') | Some(b'A'..=b'Z') => {
                Ok(Marker::LineStart)
            }
            None => Ok(Marker::Done),
            Some(err) => Err(ErrorKind::InvalidChar.with_context(err))?,
        }
    }

    pub(super) fn close_line(&self, iter: &mut Scan<impl Iterator<Item = Byte>>) -> Result<()> {
        make_local!(iter);
        loop {
            match iter.peak()? {
                Some(b'\n') => discard_and!(break Ok(())),
                Some(b'\r') => discard_and!(match iter.peak()? {
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

#[derive(Debug)]
pub(super) struct Failure(Option<Error>);

impl Failure {
    pub(super) fn error(&mut self) -> Error {
        self.0
            .take()
            .unwrap_or_else(|| ErrorKind::RepeatFailure.into())
    }
}

impl From<Error> for Failure {
    fn from(e: Error) -> Self {
        Self(Some(e))
    }
}

/* Scalar */

#[derive(Debug, Default)]
pub(super) struct ScalarLiteral {
    pub scalar: Vec<u8>,
}

impl ScalarLiteral {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = Byte>>) -> Result<Marker> {
        match iter.peak()? {
            Some(_) => unimplemented!("Can't parse after scalar literal"),
            None => Ok(Marker::Done),
        }
    }
}

impl From<AmbiguousScalar> for ScalarLiteral {
    fn from(prev: AmbiguousScalar) -> Self {
        Self {
            scalar: prev.scratch,
        }
    }
}

/* Map */
#[derive(Debug)]
pub(super) struct MapStart {
    indent_floor: u16,
    scratch: Vec<u8>,
}

impl MapStart {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = Byte>>) -> Result<Marker> {
        match iter.peak()? {
            Some(_) => Ok(Marker::MapVerifyKey),
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

impl From<MapStart> for MapVerifyKey {
    fn from(prev: MapStart) -> Self {
        Self {
            indent_floor: prev.indent_floor,
            key: prev.scratch,
        }
    }
}

#[derive(Debug)]
pub(super) struct MapVerifyKey {
    indent_floor: u16,
    pub key: Vec<u8>,
}

impl MapVerifyKey {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = Byte>>) -> Result<Marker> {
        match iter.peak()? {
            Some(_) => Ok(Marker::MapWhiteSpace),
            None => Err(ErrorKind::EOFMapping)?,
        }
    }

    pub(super) fn parse_key(&mut self, _: &mut Scan<impl Iterator<Item = Byte>>) -> Result<()> {
        for ch in self.key.iter() {
            match ch {
                b'a'..=b'z' | b'A'..=b'Z' | b' ' | b'\t' => (),
                err => return Err(ErrorKind::InvalidChar.with_context(*err))?,
            }
        }

        while !self.key.is_empty() && is_whitespace(self.key.last().unwrap()) {
            dbg!(self.key.pop());
        }

        Ok(())
    }
}

impl From<MapVerifyKey> for MapWhiteSpace {
    fn from(prev: MapVerifyKey) -> Self {
        Self {
            indent_floor: prev.indent_floor,
        }
    }
}

#[derive(Debug)]
pub(super) struct MapWhiteSpace {
    indent_floor: u16,
}

impl MapWhiteSpace {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = Byte>>) -> Result<Marker> {
        match iter.peak()? {
            Some(b'a'..=b'z') | Some(b'A'..=b'Z') => Ok(Marker::MapValue),
            _ => Err(ErrorKind::Message("Unclear fail state".into()))?,
        }
    }

    pub(super) fn parse_whitespace(
        &self,
        iter: &mut Scan<impl Iterator<Item = Byte>>,
    ) -> Result<()> {
        loop {
            match iter.peak()? {
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
            value: Default::default(),
        }
    }
}

#[derive(Debug)]
pub(super) struct MapValue {
    indent_floor: u16,
    pub value: Vec<u8>,
}

impl MapValue {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = Byte>>) -> Result<Marker> {
        match iter.peak()? {
            Some(b'\n') | Some(b'\r') => Ok(Marker::LineEnd),
            Some(err) => Err(ErrorKind::InvalidChar.with_context(([b'\n', b'\r'], err)))?,
            None => Ok(Marker::Done),
        }
    }

    pub(super) fn parse_value(
        &mut self,
        iter: &mut Scan<impl Iterator<Item = Byte>>,
    ) -> Result<()> {
        make_local!(iter);

        loop {
            match iter.peak()? {
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

fn is_whitespace(c: &u8) -> bool {
    *c == b'\t' || *c == b' '
}

// #[derive(Debug, Default)]
// pub(super) struct WhiteSpace;

// impl WhiteSpace {
//     pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = Byte>>) -> Result<Marker> {
//         match iter.peak()? {
//             Some(b'\n') | Some(b'\r') => Ok(Marker::LineEnd),
//             Some(_txt) => Ok(Marker::Ignore),
//             None => Ok(Marker::Done),
//         }
//     }

//     pub(super) fn skip_whitespace(
//         &self,
//         iter: &mut Scan<impl Iterator<Item = Byte>>,
//     ) -> Result<()> {
//         loop {
//             match iter.peak()? {
//                 Some(b' ') | Some(b'\t') => iter.discard(),
//                 _ => break Ok(()),
//             }
//         }
//     }
// }

// #[derive(Debug, Default)]
// pub(super) struct Ignore;

// impl Ignore {
//     pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = Byte>>) -> Result<Marker> {
//         match iter.peak()? {
//             Some(b'\n') | Some(b'\r') => Ok(Marker::LineEnd),
//             Some(b' ') | Some(b'\t') => Ok(Marker::WhiteSpace),
//             None => Ok(Marker::Done),
//             Some(err) => Err(ErrorKind::InvalidChar.with_context(err))?,
//         }
//     }

//     pub(super) fn skip_til_whitespace(
//         &self,
//         iter: &mut Scan<impl Iterator<Item = Byte>>,
//     ) -> Result<()> {
//         loop {
//             match iter.peak()? {
//                 Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => break Ok(()),
//                 None => break Ok(()),
//                 _ => iter.discard(),
//             }
//         }
//     }
// }
