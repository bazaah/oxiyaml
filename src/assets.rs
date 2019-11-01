use std::cmp::Ordering;

#[derive(Debug, Default)]
pub(super) struct Indent {
    floor: u16,
    previous: u16,
    current: u16,
}

impl Indent {
    pub(super) fn new() -> Self {
        Default::default()
    }

    pub(super) fn current(&self) -> u16 {
        self.current
    }

    pub(super) fn previous(&self) -> u16 {
        self.previous
    }

    pub(super) fn floor(&self) -> u16 {
        self.floor
    }

    pub(super) fn insert(&mut self, idt: u16) -> Ordering {
        self.previous = self.current;
        self.current = idt;
        idt.cmp(&self.floor)
    }

    pub(super) fn adjust_floor(&mut self, new: u16) {
        self.floor = new;
    }
}
