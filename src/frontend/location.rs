use std::ops::Add;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Location(pub u16, pub u16);

/// [from, to], both inclusive
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
