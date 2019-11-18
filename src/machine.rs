use {
    super::{scanner::*, states::*},
    std::collections::VecDeque,
};

pub(super) struct StateMachine<I, S = Start, INDENT = Inactive> {
    // Current state
    state: S,

    // Byte iter
    scan: Scan<I, INDENT>,

    // Node storage
    store: VecDeque<Vec<u8>>,
}

impl<I> StateMachine<I>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn new(stream: I) -> Self {
        StateMachine {
            state: Default::default(),
            scan: Scan::new(stream),
            store: Default::default(),
        }
    }

    pub(super) fn cycle(&mut self) -> Marker {
        self.state.find_next(&mut self.scan)
    }
}

impl<I> StateMachine<I, WhiteSpace>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn cycle(&mut self) -> Marker {
        self.state.skip_whitespace(&mut self.scan);

        self.state.find_next(&mut self.scan)
    }
}

impl<I> StateMachine<I, Ignore>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn cycle(&mut self) -> Marker {
        self.state.skip_til_whitespace(&mut self.scan);

        self.state.find_next(&mut self.scan)
    }
}

impl<I> StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn cycle(&mut self) -> Marker {
        self.state.update_indent(&mut self.scan);

        self.state.find_next(&mut self.scan)
    }
}

impl<I> StateMachine<I, LineEnd>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn cycle(&mut self) -> Marker {
        self.state.close_line(&mut self.scan);

        self.state.find_next(&mut self.scan)
    }
}

impl<I> StateMachine<I, MapKey>
where
    I: Iterator<Item = u8>,
{
    // pub(super) fn cycle(&mut self) -> Marker {
    //     self.state.parse_key(&mut self.scan);

    //     Marker::Failure
    // }
}

/* Exit States */
impl<I> StateMachine<I, Done>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn done(&self) -> String {
        format!("Finished! Indent history: {:?}", self.scan.history())
    }
}

impl<I> StateMachine<I, Failure>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn error(self) -> String {
        self.state.into_inner()
    }
}

/* Legal state transitions */
impl<I> From<StateMachine<I, Start>> for StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = u8>,
{
    fn from(prev: StateMachine<I, Start>) -> Self {
        Self {
            state: Default::default(),
            scan: prev.scan.activate(),
            store: Default::default(),
        }
    }
}

macro_rules! from_start {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = u8>> From<StateMachine<I, Start>> for StateMachine<I, $type> {
                fn from(prev: StateMachine<I, Start>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan,
                        store: prev.store,
                    }
                }
            }
        )*
    };
}

from_start!(Done);

macro_rules! from_whitespace {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = u8>> From<StateMachine<I, WhiteSpace>> for StateMachine<I, $type> {
                fn from(prev: StateMachine<I, WhiteSpace>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan,
                        store: prev.store,
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
            impl<I: Iterator<Item = u8>> From<StateMachine<I, Ignore>> for StateMachine<I, $type> {
                fn from(prev: StateMachine<I, Ignore>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan,
                        store: prev.store,
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
            impl<I: Iterator<Item = u8>> From<StateMachine<I, LineStart, Active>> for StateMachine<I, $type> {
                fn from(prev: StateMachine<I, LineStart, Active>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan.deactivate(),
                        store: prev.store,
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
            impl<I: Iterator<Item = u8>> From<StateMachine<I, LineEnd>> for StateMachine<I, $type> {
                fn from(prev: StateMachine<I, LineEnd>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan,
                        store: prev.store,
                    }
                }
            }
        )*
    };
}

from_lineend!(Done);

impl<I> From<StateMachine<I, LineEnd>> for StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = u8>,
{
    fn from(prev: StateMachine<I, LineEnd>) -> Self {
        Self {
            state: Default::default(),
            scan: prev.scan.activate(),
            store: prev.store,
        }
    }
}

impl<T, I, S> From<(T, StateMachine<I, S>)> for StateMachine<I, Failure>
where
    T: ToString,
    I: Iterator<Item = u8>,
{
    fn from((msg, prev): (T, StateMachine<I, S>)) -> Self {
        StateMachine {
            state: Failure::new(msg),
            scan: prev.scan,
            store: prev.store,
        }
    }
}

impl<T, I, S> From<(T, StateMachine<I, S, Active>)> for StateMachine<I, Failure>
where
    T: ToString,
    I: Iterator<Item = u8>,
{
    fn from((msg, prev): (T, StateMachine<I, S, Active>)) -> Self {
        StateMachine {
            state: Failure::new(msg),
            scan: prev.scan.deactivate(),
            store: prev.store,
        }
    }
}
