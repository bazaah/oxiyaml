use std::{convert::TryFrom, fmt, marker::PhantomData};

struct CountMachine<'state, S = Start>
where
    S: Describe + 'state,
{
    data: Box<dyn Iterator<Item = u8> + 'state>,
    buffer: Vec<u8>,
    state: S,

    t_words: usize,
    t_letters: usize,

    mkr: PhantomData<&'state S>,
}

impl<'state> CountMachine<'state> {
    fn new(data: impl Iterator<Item = u8> + 'state) -> Self {
        CountMachine {
            data: Box::new(data),
            buffer: Default::default(),
            state: Start,
            t_words: 0,
            t_letters: 0,
            mkr: PhantomData,
        }
    }
}

impl<'state> CountMachine<'state, Word> {
    fn next_word(&mut self) {
        self.state.get_word(&mut self.buffer, &mut self.data);
        if !self.buffer.is_empty() {
            self.t_words += 1;
        }
    }
}

impl<'state> CountMachine<'state, Letters> {
    fn count_letters(&mut self) {
        self.t_letters += self.state.count_letters(&self.buffer);
    }
}

impl<'state> CountMachine<'state, Done> {
    fn done(self) -> (usize, usize) {
        self.into()
    }
}

impl<'state> From<CountMachine<'state>> for CountMachine<'state, Word> {
    fn from(prev: CountMachine<'state>) -> Self {
        CountMachine {
            data: prev.data,
            buffer: prev.buffer,
            state: Word::new(),
            t_words: prev.t_words,
            t_letters: prev.t_letters,
            mkr: PhantomData,
        }
    }
}

impl<'state> TryFrom<CountMachine<'state, Word>> for CountMachine<'state, Letters> {
    type Error = CountMachine<'state, Done>;

    fn try_from(mut prev: CountMachine<'state, Word>) -> Result<Self, Self::Error> {
        prev.next_word();
        match prev.buffer.is_empty() {
            true => Err(CountMachine {
                data: prev.data,
                buffer: prev.buffer,
                state: Done,
                t_words: prev.t_words,
                t_letters: prev.t_letters,
                mkr: PhantomData,
            }),
            false => Ok(CountMachine {
                data: prev.data,
                buffer: prev.buffer,
                state: Letters::new(),
                t_words: prev.t_words,
                t_letters: prev.t_letters,
                mkr: PhantomData,
            }),
        }
    }
}

impl<'state> From<CountMachine<'state, Letters>> for CountMachine<'state, Word> {
    fn from(mut prev: CountMachine<'state, Letters>) -> Self {
        prev.count_letters();
        prev.buffer.clear();
        CountMachine {
            data: prev.data,
            buffer: prev.buffer,
            state: Word::new(),
            t_words: prev.t_words,
            t_letters: prev.t_letters,
            mkr: PhantomData,
        }
    }
}

impl<'state> From<CountMachine<'state, Done>> for (usize, usize) {
    fn from(prev: CountMachine<'state, Done>) -> Self {
        (prev.t_words, prev.t_letters)
    }
}

struct Start;

impl Describe for Start {}

struct Word {
    pristine: bool,
}

impl Word {
    fn new() -> Self {
        Self { pristine: true }
    }

    fn get_word(&mut self, buffer: &mut Vec<u8>, source: &mut impl Iterator<Item = u8>) {
        while let Some(letter) = source.next() {
            match letter {
                b' ' | b'\n' | b'\t' => break,
                _ => buffer.push(letter),
            }
            self.pristine = false;
        }
    }
}

impl Describe for Word {
    fn describe(&self) -> &dyn fmt::Display {
        if self.pristine {
            &"no word yet"
        } else {
            &"have a word"
        }
    }
}

struct Letters;

impl Letters {
    fn new() -> Self {
        Self
    }

    fn count_letters(&self, word: &[u8]) -> usize {
        word.len()
    }
}

impl Describe for Letters {
    fn describe(&self) -> &dyn fmt::Display {
        &"some letters here"
    }
}

struct Done;

impl Describe for Done {}

trait Describe {
    fn describe(&self) -> &dyn fmt::Display {
        &"nothing here"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn check_state() {
        let data = "my very energetic mother jumped straight under nine planets";
        let mut machine: CountMachine<Word> = CountMachine::new(data.bytes()).into();

        let (words, letters): (usize, usize) = loop {
            match <CountMachine<Word> as TryInto<CountMachine<Letters>>>::try_into(machine) {
                Ok(words) => machine = words.into(),
                Err(done) => break done.done(),
            }
        };

        println!("total words: {} | total letters: {}", words, letters,);
        panic!()
    }

    fn different_types() {}
}
