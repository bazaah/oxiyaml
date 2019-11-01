use super::{scanner::*, states::*};

pub(super) struct IndentMachine<I, S = Start, M = Inactive> {
    state: S,

    scan: Scan<I, M>,
}

impl<I> IndentMachine<I>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn new(stream: I) -> Self {
        IndentMachine {
            state: Default::default(),
            scan: Scan::new(stream),
        }
    }

    pub(super) fn cycle(&mut self) -> Marker {
        self.state.find_next(&mut self.scan)
    }
}

impl<I> IndentMachine<I, WhiteSpace>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn cycle(&mut self) -> Marker {
        self.state.skip_whitespace(&mut self.scan);

        self.state.find_next(&mut self.scan)
    }
}

impl<I> IndentMachine<I, Ignore>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn cycle(&mut self) -> Marker {
        self.state.skip_til_whitespace(&mut self.scan);

        self.state.find_next(&mut self.scan)
    }
}

impl<I> IndentMachine<I, LineStart, Active>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn cycle(&mut self) -> Marker {
        self.state.update_indent(&mut self.scan);

        self.state.find_next(&mut self.scan)
    }
}

impl<I> IndentMachine<I, LineEnd>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn cycle(&mut self) -> Marker {
        self.state.close_line(&mut self.scan);

        self.state.find_next(&mut self.scan)
    }
}

impl<I> IndentMachine<I, Done>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn done(&self) -> String {
        format!("Finished! Indent history: {:?}", self.scan.history())
    }
}

impl<I> IndentMachine<I, Failure>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn error(self) -> String {
        self.state.into_inner()
    }
}

impl<I> From<IndentMachine<I, Start>> for IndentMachine<I, LineStart, Active>
where
    I: Iterator<Item = u8>,
{
    fn from(prev: IndentMachine<I, Start>) -> Self {
        Self {
            state: Default::default(),
            scan: prev.scan.activate(),
        }
    }
}

macro_rules! from_start {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = u8>> From<IndentMachine<I, Start>> for IndentMachine<I, $type> {
                fn from(prev: IndentMachine<I, Start>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan,
                    }
                }
            }
        )*
    };
}

from_start!(Ignore, LineEnd, Done);

macro_rules! from_whitespace {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = u8>> From<IndentMachine<I, WhiteSpace>> for IndentMachine<I, $type> {
                fn from(prev: IndentMachine<I, WhiteSpace>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan,
                    }
                }
            }
        )*
    };
}

from_whitespace!(LineEnd, Ignore, Done);

macro_rules! from_ignore {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = u8>> From<IndentMachine<I, Ignore>> for IndentMachine<I, $type> {
                fn from(prev: IndentMachine<I, Ignore>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan,
                    }
                }
            }
        )*
    };
}

from_ignore!(LineEnd, WhiteSpace, Done);

macro_rules! from_linestart {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = u8>> From<IndentMachine<I, LineStart, Active>> for IndentMachine<I, $type> {
                fn from(prev: IndentMachine<I, LineStart, Active>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan.deactivate(),
                    }
                }
            }
        )*
    };
}

from_linestart!(LineEnd, Ignore, Done);

macro_rules! from_lineend {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = u8>> From<IndentMachine<I, LineEnd>> for IndentMachine<I, $type> {
                fn from(prev: IndentMachine<I, LineEnd>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan,
                    }
                }
            }
        )*
    };
}

from_lineend!(Done);

impl<I> From<IndentMachine<I, LineEnd>> for IndentMachine<I, LineStart, Active>
where
    I: Iterator<Item = u8>,
{
    fn from(prev: IndentMachine<I, LineEnd>) -> Self {
        Self {
            state: Default::default(),
            scan: prev.scan.activate(),
        }
    }
}

impl<T, I, S> From<(T, IndentMachine<I, S>)> for IndentMachine<I, Failure>
where
    T: ToString,
    I: Iterator<Item = u8>,
{
    fn from((msg, prev): (T, IndentMachine<I, S>)) -> Self {
        IndentMachine {
            state: Failure::new(msg),
            scan: prev.scan,
        }
    }
}

impl<T, I, S> From<(T, IndentMachine<I, S, Active>)> for IndentMachine<I, Failure>
where
    T: ToString,
    I: Iterator<Item = u8>,
{
    fn from((msg, prev): (T, IndentMachine<I, S, Active>)) -> Self {
        IndentMachine {
            state: Failure::new(msg),
            scan: prev.scan.deactivate(),
        }
    }
}

// trait NextState {
//     fn next_state(&mut self) -> Marker;
// }

// macro_rules! impl_next_state {
//     ( $($type:ident),* ) => {
//         $(
//             impl<I: Iterator<Item = u8>> NextState for IndentMachine<I, $type> {
//                 fn next_state(&mut self) -> Marker {
//                     self.state.find_next(&mut self.scan)
//                 }
//             }
//         )*
//     };
// }

// impl_next_state!(Start, WhiteSpace, Ignore, LineEnd);

// impl<I: Iterator<Item = u8>> NextState for IndentMachine<I, LineStart, Active> {
//     fn next_state(&mut self) -> Marker {
//         self.state.find_next(&mut self.scan)
//     }
// }
