use std::marker::PhantomData;

struct StateMachine<'state, S = StateA<'state>>
where
    S: Lifetime<'state> + 'state,
{
    data: String,
    state: S,
    mkr: PhantomData<&'state S>,
}

impl<'state> StateMachine<'state> {
    fn with_words(words: &'state str) -> Self {
        StateMachine {
            data: format!("dank memes"),
            state: StateA { value: words },
            mkr: PhantomData,
        }
    }
}

impl<'state> Default for StateMachine<'state> {
    fn default() -> Self {
        Self {
            data: format!("dank memes"),
            state: Default::default(),
            mkr: PhantomData,
        }
    }
}

impl<'state, S> Lifetime<'state> for StateMachine<'state, S>
where
    S: Lifetime<'state> + 'state,
{
    fn value(&'state self) -> &'state str {
        self.state.value()
    }
}

impl<'state> From<StateMachine<'state, StateA<'state>>> for StateMachine<'state, StateB<'state>> {
    fn from(s: StateMachine<'state, StateA<'state>>) -> Self {
        StateMachine {
            data: s.data,
            state: StateB::new(s.state.value),
            mkr: PhantomData,
        }
    }
}

impl<'state> From<StateMachine<'state, StateB<'state>>> for StateMachine<'state, StateC> {
    fn from(s: StateMachine<'state, StateB<'state>>) -> Self {
        StateMachine {
            data: s.data,
            state: StateC::new(&s.state.split),
            mkr: PhantomData,
        }
    }
}

trait Lifetime<'state> {
    fn value(&'state self) -> &'state str;
}

#[derive(Default)]
struct StateA<'state> {
    value: &'state str,
}

impl<'state> Lifetime<'state> for StateA<'state> {
    fn value(&'state self) -> &'state str {
        self.value
    }
}

struct StateB<'state> {
    split: Vec<&'state str>,
}

impl<'state> StateB<'state> {
    fn new(src: &'state str) -> Self {
        Self {
            split: src.split(' ').collect(),
        }
    }
}

impl<'state> Lifetime<'state> for StateB<'state> {
    fn value(&'state self) -> &'state str {
        self.split[0]
    }
}

struct StateC {
    last: String,
}

impl StateC {
    fn new<S: AsRef<str>>(words: &[S]) -> Self {
        let last =
            words
                .iter()
                .map(|s| s.as_ref())
                .rev()
                .fold(String::default(), |mut last, word| {
                    last += word;
                    last
                });
        StateC { last }
    }
}

impl<'state> Lifetime<'state> for StateC {
    fn value(&'state self) -> &'state str {
        self.last.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check() {
        let state = StateMachine::with_words("dank memes boi");
        assert_eq!(state.value(), "dank memes boi");

        let state: StateMachine<StateB> = state.into();
        assert_eq!(state.value(), "dank");

        let state: StateMachine<StateC> = state.into();
        assert_eq!(state.value(), "boimemesdank")
    }
}
