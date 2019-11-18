#[derive(Debug)]
pub(super) struct Scan<I, S = Inactive> {
    ch: Option<u8>,
    iter: I,
    indent: IndentTrack<S>,
}

impl<I, S> Scan<I, S>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn next(&mut self) -> Option<u8> {
        match self.ch.take() {
            ch @ Some(_) => ch,
            None => match self.iter.next() {
                ch @ Some(_) => ch,
                None => None,
            },
        }
    }

    pub(super) fn peak(&mut self) -> Option<u8> {
        match self.ch {
            ch @ Some(_) => ch,
            None => match self.iter.next() {
                ch @ Some(_) => {
                    self.ch = ch;
                    self.ch
                }
                None => None,
            },
        }
    }

    pub(super) fn discard(&mut self) {
        self.ch = None;
    }

    pub(super) fn current(&self) -> u16 {
        self.indent.current()
    }

    pub(super) fn previous(&self) -> u16 {
        self.indent.previous()
    }

    pub(super) fn floor(&self) -> u16 {
        self.indent.floor()
    }

    pub(super) fn history(&self) -> &[u16] {
        self.indent.history()
    }

    pub(super) fn clamp(&mut self) {
        self.indent.clamp()
    }

    pub(super) fn unclamp(&mut self) {
        self.indent.unclamp()
    }
}

impl<I> Scan<I, Active>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn update_indent(&mut self, new: u16) {
        self.indent.update(new);
    }

    pub(super) fn deactivate(self) -> Scan<I, Inactive> {
        Scan {
            ch: self.ch,
            iter: self.iter,
            indent: self.indent.deactivate(),
        }
    }
}

impl<I> Scan<I, Inactive>
where
    I: Iterator<Item = u8>,
{
    pub(super) fn new(iter: I) -> Self {
        Self {
            ch: None,
            iter,
            indent: Default::default(),
        }
    }

    pub(super) fn activate(self) -> Scan<I, Active> {
        Scan {
            ch: self.ch,
            iter: self.iter,
            indent: self.indent.activate(),
        }
    }
}

#[derive(Debug)]
struct IndentTrack<S = Inactive> {
    //Flags
    state: S,
    clamp_floor: bool,

    // Values
    floor: u16,
    previous: u16,
    current: u16,

    // Misc
    history: Vec<u16>,
}

impl<S> IndentTrack<S> {
    fn current(&self) -> u16 {
        self.current
    }

    fn previous(&self) -> u16 {
        self.previous
    }

    fn floor(&self) -> u16 {
        self.floor
    }

    fn history(&self) -> &[u16] {
        self.history.as_ref()
    }

    fn clamp(&mut self) {
        self.clamp_floor = true;
    }

    fn unclamp(&mut self) {
        self.clamp_floor = false;
    }
}

impl IndentTrack<Active> {
    fn update(&mut self, new: u16) {
        if !self.clamp_floor && new > self.floor {
            self.floor = new;
        }
        self.previous = self.current;
        self.current = new;
        self.history.push(new);
    }

    fn deactivate(self) -> IndentTrack<Inactive> {
        IndentTrack {
            state: Default::default(),
            clamp_floor: self.clamp_floor,
            floor: self.floor,
            previous: self.previous,
            current: self.current,
            history: self.history,
        }
    }
}

impl IndentTrack<Inactive> {
    fn activate(self) -> IndentTrack<Active> {
        IndentTrack {
            state: Default::default(),
            clamp_floor: self.clamp_floor,
            floor: self.floor,
            previous: self.previous,
            current: self.current,
            history: self.history,
        }
    }
}

impl Default for IndentTrack {
    fn default() -> Self {
        Self {
            state: Default::default(),
            clamp_floor: true,
            floor: 0,
            previous: 0,
            current: 0,
            history: Default::default(),
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct Active;

#[derive(Debug, Default)]
pub(super) struct Inactive;

// #[cfg(test)]
// mod tests {
//     use super::*;

// }
