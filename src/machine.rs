use super::{
    error::{Error, Result},
    event::Event,
    node::NodeKind,
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
    I: Iterator<Item = Byte>,
{
    pub(super) fn new(stream: I) -> Self {
        StateMachine {
            state: Default::default(),
            scan: Scan::new(stream),
        }
    }
}

impl<I> StateMachine<I, Done>
where
    I: Iterator<Item = Byte>,
{
    pub(super) fn done(&self) -> String {
        format!("Finished! Indent history: {:?}", self.scan.history())
    }

    pub(super) fn cycle(self, output: &mut Event) -> Self {
        *output = Some(().into());
        self
    }
}

impl<I> StateMachine<I, Failure>
where
    I: Iterator<Item = Byte>,
{
    pub(super) fn cycle(mut self, output: &mut Event) -> Self {
        *output = Some(self.state.error().into());
        self
    }
}

/* State Drivers */
/* ============= */
pub(super) trait Drive {
    type Event;

    fn drive(&mut self, _: &mut Self::Event) -> Result<Marker>;
}

impl<I> Drive for StateMachine<I>
where
    I: Iterator<Item = Byte>,
{
    type Event = Event;

    fn drive(&mut self, _: &mut Self::Event) -> Result<Marker> {
        self.state.find_next(&mut self.scan)
    }
}

impl<I> Drive for StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = Byte>,
{
    type Event = Event;

    fn drive(&mut self, _: &mut Self::Event) -> Result<Marker> {
        self.state
            .update_indent(&mut self.scan)
            .and(self.state.find_next(&mut self.scan))
    }
}

impl<I> Drive for StateMachine<I, LineEnd>
where
    I: Iterator<Item = Byte>,
{
    type Event = Event;

    fn drive(&mut self, _: &mut Self::Event) -> Result<Marker> {
        self.state
            .close_line(&mut self.scan)
            .and(self.state.find_next(&mut self.scan))
    }
}

/* Ambiguous Drivers */
impl<I> Drive for StateMachine<I, AmbiguousScalar>
where
    I: Iterator<Item = Byte>,
{
    type Event = Event;

    fn drive(&mut self, _: &mut Self::Event) -> Result<Marker> {
        self.state.find_next(&mut self.scan)
    }
}

impl<I> Drive for StateMachine<I, AmbiguousColon>
where
    I: Iterator<Item = Byte>,
{
    type Event = Event;

    fn drive(&mut self, _: &mut Self::Event) -> Result<Marker> {
        self.state.find_next(&mut self.scan)
    }
}

/* Scalar Drivers */
impl<I> Drive for StateMachine<I, ScalarLiteral>
where
    I: Iterator<Item = Byte>,
{
    type Event = Event;

    fn drive(&mut self, output: &mut Self::Event) -> Result<Marker> {
        self.state.find_next(&mut self.scan).and_then(|m| {
            *output = Some(NodeKind::ScalarPlain(take(&mut self.state.scalar)).into());
            Ok(m)
        })
    }
}

/* Map Drivers */
impl<I> Drive for StateMachine<I, MapStart>
where
    I: Iterator<Item = Byte>,
{
    type Event = Event;

    fn drive(&mut self, _: &mut Self::Event) -> Result<Marker> {
        self.state.find_next(&mut self.scan)
    }
}

impl<I> Drive for StateMachine<I, MapVerifyKey>
where
    I: Iterator<Item = Byte>,
{
    type Event = Event;

    fn drive(&mut self, output: &mut Self::Event) -> Result<Marker> {
        self.state
            .parse_key(&mut self.scan)
            .and_then(|v| {
                *output = Some(NodeKind::Key(take(&mut self.state.key)).into());
                Ok(v)
            })
            .and(self.state.find_next(&mut self.scan))
    }
}

impl<I> Drive for StateMachine<I, MapWhiteSpace>
where
    I: Iterator<Item = Byte>,
{
    type Event = Event;

    fn drive(&mut self, _: &mut Self::Event) -> Result<Marker> {
        self.state
            .parse_whitespace(&mut self.scan)
            .and(self.state.find_next(&mut self.scan))
    }
}

impl<I> Drive for StateMachine<I, MapValue>
where
    I: Iterator<Item = Byte>,
{
    type Event = Event;

    fn drive(&mut self, output: &mut Self::Event) -> Result<Marker> {
        self.state
            .parse_value(&mut self.scan)
            .and_then(|v| {
                *output = Some(NodeKind::ScalarPlain(take(&mut self.state.value)).into());
                Ok(v)
            })
            .and(self.state.find_next(&mut self.scan))
    }
}

/* Legal state transitions */
/* ======================= */
impl<I> From<StateMachine<I, Start>> for StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = Byte>,
{
    fn from(prev: StateMachine<I, Start>) -> Self {
        Self {
            state: Default::default(),
            scan: prev.scan.activate(),
        }
    }
}

// macro_rules! from_start {
//     ( $($type:ident),* ) => {
//         $(
//             impl<I: Iterator<Item = Byte>> From<StateMachine<I, Start>> for StateMachine<I, $type> {

//                 fn from(prev: StateMachine<I, Start>) -> Self {
//                     Self {
//                         state: Default::default(),
//                         scan: prev.scan,
//                     }
//                 }
//             }
//         )*
//     };
// }

//from_start!();

macro_rules! from_linestart {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = Byte>> From<StateMachine<I, LineStart, Active>> for StateMachine<I, $type> {


                fn from(prev: StateMachine<I, LineStart, Active>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan.deactivate(),
                    }
                }
            }
        )*
    };
}

from_linestart!(LineEnd, AmbiguousScalar);

// macro_rules! from_lineend {
//     ( $($type:ident),* ) => {
//         $(
//             impl<I: Iterator<Item = Byte>> From<StateMachine<I, LineEnd>> for StateMachine<I, $type> {

//                 fn from(prev: StateMachine<I, LineEnd>) -> Self {
//                     Self {
//                         state: Default::default(),
//                         scan: prev.scan,
//                     }
//                 }
//             }
//         )*
//     };
// }

//from_lineend!();

impl<I> From<StateMachine<I, LineEnd>> for StateMachine<I, LineStart, Active>
where
    I: Iterator<Item = Byte>,
{
    fn from(prev: StateMachine<I, LineEnd>) -> Self {
        Self {
            state: Default::default(),
            scan: prev.scan.activate(),
        }
    }
}

/* Ambiguous */
macro_rules! from_ambi_scalar {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = Byte>> From<StateMachine<I, AmbiguousScalar>> for StateMachine<I, $type> {


                fn from(prev: StateMachine<I, AmbiguousScalar>) -> Self {
                    Self {
                        state: prev.state.into(),
                        scan: prev.scan,
                    }
                }
            }
        )*
    };
}

from_ambi_scalar!(AmbiguousColon, ScalarLiteral);

impl<I: Iterator<Item = Byte>> From<StateMachine<I, AmbiguousColon>>
    for StateMachine<I, AmbiguousScalar>
{
    fn from(prev: StateMachine<I, AmbiguousColon>) -> Self {
        Self {
            state: prev.state.into(),
            scan: prev.scan,
        }
    }
}

impl<I: Iterator<Item = Byte>> From<StateMachine<I, AmbiguousColon>> for StateMachine<I, MapStart> {
    fn from(prev: StateMachine<I, AmbiguousColon>) -> Self {
        Self {
            state: MapStart::extend_from(prev.scan.floor(), prev.state.scratch),
            scan: prev.scan,
        }
    }
}

/* Scalar */

/* Map */
impl<I: Iterator<Item = Byte>> From<StateMachine<I, MapStart>> for StateMachine<I, MapVerifyKey> {
    fn from(prev: StateMachine<I, MapStart>) -> Self {
        Self {
            state: prev.state.into(),
            scan: prev.scan,
        }
    }
}

impl<I: Iterator<Item = Byte>> From<StateMachine<I, MapVerifyKey>>
    for StateMachine<I, MapWhiteSpace>
{
    fn from(prev: StateMachine<I, MapVerifyKey>) -> Self {
        Self {
            state: prev.state.into(),
            scan: prev.scan,
        }
    }
}

impl<I: Iterator<Item = Byte>> From<StateMachine<I, MapWhiteSpace>> for StateMachine<I, MapValue> {
    fn from(prev: StateMachine<I, MapWhiteSpace>) -> Self {
        Self {
            state: prev.state.into(),
            scan: prev.scan,
        }
    }
}

impl<I: Iterator<Item = Byte>> From<StateMachine<I, MapValue>> for StateMachine<I, LineEnd> {
    fn from(prev: StateMachine<I, MapValue>) -> Self {
        Self {
            state: Default::default(),
            scan: prev.scan,
        }
    }
}

/* Into Failure */
impl<I, S> From<(Error, StateMachine<I, S>)> for StateMachine<I, Failure>
where
    I: Iterator<Item = Byte>,
{
    fn from((err, prev): (Error, StateMachine<I, S>)) -> Self {
        StateMachine {
            state: err.into(),
            scan: prev.scan,
        }
    }
}

impl<I, S> From<(Error, StateMachine<I, S, Active>)> for StateMachine<I, Failure>
where
    I: Iterator<Item = Byte>,
{
    fn from((err, prev): (Error, StateMachine<I, S, Active>)) -> Self {
        StateMachine {
            state: err.into(),
            scan: prev.scan.deactivate(),
        }
    }
}

/* Into Done */
macro_rules! to_done {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = Byte>> From<StateMachine<I, $type>> for StateMachine<I, Done> {


                fn from(prev: StateMachine<I, $type>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan,
                    }
                }
            }
        )*
    };
}

to_done!(Start, LineEnd, ScalarLiteral, MapValue);

macro_rules! to_done_deactivate {
    ( $($type:ident),* ) => {
        $(
            impl<I: Iterator<Item = Byte>> From<StateMachine<I, $type, Active>> for StateMachine<I, Done> {


                fn from(prev: StateMachine<I, $type, Active>) -> Self {
                    Self {
                        state: Default::default(),
                        scan: prev.scan.deactivate(),
                    }
                }
            }
        )*
    };
}

to_done_deactivate!(LineStart);

/* Helper */

fn take<T: Default>(dest: &mut T) -> T {
    std::mem::replace(dest, T::default())
}
