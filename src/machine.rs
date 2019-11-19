use super::{
    error::{Error, Result},
    node::*,
    scanner::*,
    states::*,
};

pub(super) struct StateMachine<I, S = Start, INDENT = Inactive> {
    // Current state
    state: S,

    // Byte iter
    scan: Scan<I, INDENT>,
}

impl<I> StateMachine<I>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn new(stream: I) -> Self {
        StateMachine {
            state: Default::default(),
            scan: Scan::new(stream),
        }
    }

    pub(super) fn marker(&mut self) -> Result<Marker> {
        Ok(self.state.find_next(&mut self.scan))
    }
}

impl<I> StateMachine<I, WhiteSpace>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn marker(&mut self) -> Result<Marker> {
        self.state.skip_whitespace(&mut self.scan);

        Ok(self.state.find_next(&mut self.scan))
    }
}

impl<I> StateMachine<I, Ignore>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn marker(&mut self) -> Result<Marker> {
        self.state.skip_til_whitespace(&mut self.scan);

        self.state.find_next(&mut self.scan)
    }
}

impl<I> StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn marker(&mut self) -> Result<Marker> {
        self.state.update_indent(&mut self.scan);

        Ok(self.state.find_next(&mut self.scan))
    }
}

impl<I> StateMachine<I, LineEnd>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn marker(&mut self) -> Result<Marker> {
        self.state.close_line(&mut self.scan)?;

        self.state.find_next(&mut self.scan)
    }
}

impl<I> StateMachine<I, MapKey>
where
    I: Iterator<Item = u8>,
{
    // pub(super) fn marker(&mut self) -> Marker {
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

pub(super) trait Transition<T>: Sized {
    type Output;

    fn transition(_: T, _: &mut Self::Output) -> Self;
}

pub(super) trait TransitionInto<T>: Sized {
    type Output;

    fn transform(self, _: &mut Self::Output) -> T;
}

impl<T, U> TransitionInto<U> for T
where
    U: Transition<T>,
{
    type Output = U::Output;
    fn transform(self, o: &mut Self::Output) -> U {
        U::transition(self, o)
    }
}

/* Legal state transitions */
impl<I> Transition<StateMachine<I, Start>> for StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = u8>,
{
    type Output = Option<NodeKind>;

    fn transition(prev: StateMachine<I, Start>, _: &mut Self::Output) -> Self {
        Self {
            state: Default::default(),
            scan: prev.scan.activate(),
        }
    }
}

macro_rules! from_start {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = u8>> Transition<StateMachine<I, Start>> for StateMachine<I, $type> {
                type Output = Option<NodeKind>;

                fn transition(prev: StateMachine<I, Start>, _: &mut Self::Output) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan,
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
            impl<I: Iterator<Item = u8>> Transition<StateMachine<I, WhiteSpace>> for StateMachine<I, $type> {
                type Output = Option<NodeKind>;

                fn transition(prev: StateMachine<I, WhiteSpace>, _: &mut Self::Output) -> Self {
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
            impl<I: Iterator<Item = u8>> Transition<StateMachine<I, Ignore>> for StateMachine<I, $type> {
                type Output = Option<NodeKind>;

                fn transition(prev: StateMachine<I, Ignore>, _: &mut Self::Output) -> Self {
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
            impl<I: Iterator<Item = u8>> Transition<StateMachine<I, LineStart, Active>> for StateMachine<I, $type> {
                type Output = Option<NodeKind>;

                fn transition(prev: StateMachine<I, LineStart, Active>, _: &mut Self::Output) -> Self {
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
            impl<I: Iterator<Item = u8>> Transition<StateMachine<I, LineEnd>> for StateMachine<I, $type> {
                type Output = Option<NodeKind>;

                fn transition(prev: StateMachine<I, LineEnd>, _: &mut Self::Output) -> Self {
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

impl<I> Transition<StateMachine<I, LineEnd>> for StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = u8>,
{
    type Output = Option<NodeKind>;

    fn transition(prev: StateMachine<I, LineEnd>, _: &mut Self::Output) -> Self {
        Self {
            state: Default::default(),
            scan: prev.scan.activate(),
        }
    }
}

impl<I, S> Transition<(Error, StateMachine<I, S>)> for StateMachine<I, Failure>
where
    I: Iterator<Item = u8>,
{
    type Output = Option<NodeKind>;

    fn transition((err, prev): (Error, StateMachine<I, S>), output: &mut Self::Output) -> Self {
        *output = Some(NodeKind::Failure(err));

        StateMachine {
            state: Default::default(),
            scan: prev.scan,
        }
    }
}

impl<I, S> Transition<(Error, StateMachine<I, S, Active>)> for StateMachine<I, Failure>
where
    I: Iterator<Item = u8>,
{
    type Output = Option<NodeKind>;

    fn transition(
        (err, prev): (Error, StateMachine<I, S, Active>),
        output: &mut Self::Output,
    ) -> Self {
        *output = Some(NodeKind::Failure(err));

        StateMachine {
            state: Default::default(),
            scan: prev.scan.deactivate(),
        }
    }
}
