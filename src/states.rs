use {super::*, std::cell::Cell};

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
    Start,
    WhiteSpace,
    Ignore,
    LineEnd,
    LineStart,
    Done,
    Failure,
}

#[derive(Debug, Default)]
pub(super) struct Start;

impl Start {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Marker {
        match iter.peak() {
            Some(b' ') | Some(b'\t') => Marker::LineStart,
            Some(b'\n') | Some(b'\r') => Marker::LineEnd,
            Some(_txt) => Marker::Ignore,
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
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Marker {
        match iter.peak() {
            Some(b'\n') | Some(b'\r') => Marker::LineEnd,
            Some(b' ') | Some(b'\t') => Marker::WhiteSpace,
            None => Marker::Done,
            _ => Marker::Failure,
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
pub(super) struct LineEnd(Cell<bool>);

impl LineEnd {
    pub(super) fn find_next(&self, iter: &mut Scan<impl Iterator<Item = u8>>) -> Marker {
        match iter.peak() {
            _ if self.0.get() => Marker::Failure,
            Some(b' ') | Some(b'\t') | Some(b'a'..=b'z') | Some(b'A'..=b'Z') => Marker::LineStart,
            Some(_) => Marker::Failure,
            None => Marker::Done,
        }
    }

    pub(super) fn close_line(&self, iter: &mut Scan<impl Iterator<Item = u8>>) {
        make_local!(iter);
        loop {
            match iter.peak() {
                Some(b'\n') => discard_and!(break),
                Some(b'\r') => discard_and!(match iter.peak() {
                    Some(b'\n') => discard_and!(break),
                    _ => break self.0.set(true),
                }),
                None => break self.0.set(true),
                _ => iter.discard(),
            }
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Done;

#[derive(Debug)]
pub(super) struct Failure {
    msg: String,
}

impl Failure {
    pub(super) fn new<T: ToString>(s: T) -> Self {
        Self { msg: s.to_string() }
    }

    pub(super) fn into_inner(self) -> String {
        self.msg
    }
}
