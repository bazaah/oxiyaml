use {
    super::{
        error::{Error, Result},
        event::{Event, Transition},
        scanner::*,
        states::*,
    },
    std::io,
};

pub(super) struct StateMachine<I, S = Start, INDENT = Inactive> {
    // Current state
    state: S,

    // Byte iter
    scan: Scan<I, INDENT>,
}

impl<I> StateMachine<I>
where
    I: Iterator<Item = Byte>,
{
    pub(super) fn new(stream: I) -> Self {
        StateMachine {
            state: Default::default(),
            scan: Scan::new(stream),
        }
    }

    pub(super) fn marker(&mut self) -> Result<Marker> {
        self.state.find_next(&mut self.scan)
    }
}

// impl<I> StateMachine<I, WhiteSpace>
// where
//     I: Iterator<Item = Byte>,
// {
//     pub(super) fn marker(&mut self) -> Result<Marker> {
//         self.state.skip_whitespace(&mut self.scan)?;

//         self.state.find_next(&mut self.scan)
//     }
// }

// impl<I> StateMachine<I, Ignore>
// where
//     I: Iterator<Item = Byte>,
// {
//     pub(super) fn marker(&mut self) -> Result<Marker> {
//         self.state.skip_til_whitespace(&mut self.scan)?;

//         self.state.find_next(&mut self.scan)
//     }
// }

impl<I> StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = Byte>,
{
    pub(super) fn marker(&mut self) -> Result<Marker> {
        self.state.update_indent(&mut self.scan)?;

        self.state.find_next(&mut self.scan)
    }
}

impl<I> StateMachine<I, LineEnd>
where
    I: Iterator<Item = Byte>,
{
    pub(super) fn marker(&mut self) -> Result<Marker> {
        self.state.close_line(&mut self.scan)?;

        self.state.find_next(&mut self.scan)
    }
}

impl<I> StateMachine<I, Done>
where
    I: Iterator<Item = Byte>,
{
    pub(super) fn done(&self) -> String {
        format!("Finished! Indent history: {:?}", self.scan.history())
    }
}

/* Legal state transitions */
/* ======================= */

impl<I> Transition<StateMachine<I, Start>> for StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = Byte>,
{
    type Output = Event;

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
            impl<I: Iterator<Item = Byte>> Transition<StateMachine<I, Start>> for StateMachine<I, $type> {
                type Output = Event;

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
            impl<I: Iterator<Item = Byte>> Transition<StateMachine<I, WhiteSpace>> for StateMachine<I, $type> {
                type Output = Event;

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
            impl<I: Iterator<Item = Byte>> Transition<StateMachine<I, Ignore>> for StateMachine<I, $type> {
                type Output = Event;

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
            impl<I: Iterator<Item = Byte>> Transition<StateMachine<I, LineStart, Active>> for StateMachine<I, $type> {
                type Output = Event;

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
            impl<I: Iterator<Item = Byte>> Transition<StateMachine<I, LineEnd>> for StateMachine<I, $type> {
                type Output = Event;

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
    I: Iterator<Item = Byte>,
{
    type Output = Event;

    fn transition(prev: StateMachine<I, LineEnd>, _: &mut Self::Output) -> Self {
        Self {
            state: Default::default(),
            scan: prev.scan.activate(),
        }
    }
}

/* Into Failure */
impl<I, S> Transition<(Error, StateMachine<I, S>)> for StateMachine<I, Failure>
where
    I: Iterator<Item = Byte>,
{
    type Output = Event;

    fn transition((err, prev): (Error, StateMachine<I, S>), _: &mut Self::Output) -> Self {
        StateMachine {
            state: err.into(),
            scan: prev.scan,
        }
    }
}

impl<I, S> Transition<(Error, StateMachine<I, S, Active>)> for StateMachine<I, Failure>
where
    I: Iterator<Item = Byte>,
{
    type Output = Event;

    fn transition((err, prev): (Error, StateMachine<I, S, Active>), _: &mut Self::Output) -> Self {
        StateMachine {
            state: err.into(),
            scan: prev.scan.deactivate(),
        }
    }
}

/* Exit States */
impl<I> Transition<StateMachine<I, Failure>> for StateMachine<I, Failure>
where
    I: Iterator<Item = Byte>,
{
    type Output = Event;

    fn transition(mut prev: StateMachine<I, Failure>, output: &mut Self::Output) -> Self {
        *output = Some(prev.state.error().into());

        prev
    }
}

impl<I> Transition<StateMachine<I, Done>> for StateMachine<I, Done>
where
    I: Iterator<Item = Byte>,
{
    type Output = Event;

    fn transition(prev: StateMachine<I, Done>, output: &mut Self::Output) -> Self {
        *output = Some(().into());

        prev
    }
}
