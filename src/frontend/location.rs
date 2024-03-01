use std::fmt::Debug;
use std::ops::Add;

// (line, column)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location(pub u16, pub u16);

impl Debug for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.0 + 1, self.1)
    }
}

/// [from, to], both inclusive
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Range(pub Location, pub Location);

impl Range {
    #[must_use]
    pub const fn single_char(location: Location) -> Self {
        Self(location, location)
    }
}

impl Add for Range {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.min(rhs.0), self.1.max(rhs.1))
    }
}

impl Default for Range {
    fn default() -> Self {
        Self(Location(0, 0), Location(0, 0))
    }
}

impl Debug for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //         line matches         column matche
        match (self.0 .0 == self.1 .0, self.0 .1 == self.1 .1) {
            (true, true) => write!(f, "{:?}", self.0),
            (true, false) => write!(f, "{:?}-{}", self.0, self.1 .1),
            (false, _) => write!(f, "{:?}-{:?}", self.0, self.1),
        }
    }
}
