#![allow(dead_code)]

//mod assets;
mod machine;
mod scanner;
mod states;

pub(self) use crate::{machine::IndentMachine, scanner::*, states::*};

const SAMPLE: &str = r#"zero
    one
        two 
            three
    one
zero
"#;

enum State<I> {
    Start(IndentMachine<I, Start>),
    WhiteSpace(IndentMachine<I, WhiteSpace>),
    Ignore(IndentMachine<I, Ignore>),
    LineStart(IndentMachine<I, LineStart, Active>),
    LineEnd(IndentMachine<I, LineEnd>),
    Done(IndentMachine<I, Done>),
    Failure(IndentMachine<I, Failure>),
}

impl<I> State<I>
where
    I: Iterator<Item = u8>,
{
    fn new(stream: I) -> Self {
        Self::Start(IndentMachine::new(stream))
    }

    fn process(self) -> Result<String, String> {
        let mut check = 0;
        let mut bind = self;

        loop {
            match bind {
                Self::Done(dn) => return Ok(dn.done()),
                Self::Failure(fail) => return Err(fail.error()),
                _ if check > 100 => return Err(format!("SOmething is wrong in step")),
                _ => {
                    check += 1;
                    bind = bind.step()
                }
            }
        }
    }

    fn step(self) -> Self {
        match self {
            Self::Start(mut st) => match st.cycle() {
                Marker::LineStart => Self::LineStart(st.into()),
                Marker::Ignore => Self::Ignore(st.into()),
                Marker::LineEnd => Self::LineEnd(st.into()),
                Marker::Done => Self::Done(st.into()),
                _ => Self::Failure((format!("Invalid transition"), st).into()),
            },
            Self::WhiteSpace(mut st) => match st.cycle() {
                Marker::LineEnd => Self::LineEnd(st.into()),
                Marker::Ignore => Self::Ignore(st.into()),
                Marker::Done => Self::Done(st.into()),
                _ => Self::Failure((format!("Invalid transition"), st).into()),
            },
            Self::Ignore(mut st) => match st.cycle() {
                Marker::LineEnd => Self::LineEnd(st.into()),
                Marker::WhiteSpace => Self::WhiteSpace(st.into()),
                Marker::Done => Self::Done(st.into()),
                Marker::Failure => Self::Failure((format!("State violation (Ignore)"), st).into()),
                _ => Self::Failure((format!("Invalid transition"), st).into()),
            },
            Self::LineStart(mut st) => match st.cycle() {
                Marker::LineEnd => Self::LineEnd(st.into()),
                Marker::Ignore => Self::Ignore(st.into()),
                Marker::Done => Self::Done(st.into()),
                _ => Self::Failure((format!("Invalid transition"), st).into()),
            },
            Self::LineEnd(mut st) => match st.cycle() {
                Marker::LineStart => Self::LineStart(st.into()),
                Marker::Done => Self::Done(st.into()),
                Marker::Failure => Self::Failure((format!("State violation (LineEnd)"), st).into()),
                _ => Self::Failure((format!("Invalid transition"), st).into()),
            },
            st @ Self::Done(_) => st,
            st @ Self::Failure(_) => st,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_state_machine() {
        let data = SAMPLE;
        let result = State::new(data.bytes()).process();

        println!("State machine says: {:?}", result);

        panic!();
    }
}
