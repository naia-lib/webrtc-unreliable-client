use std::fmt;

#[cfg(test)]
mod direction_test;

/// Direction is a marker for transmission direction of an endpoint
#[derive(Debug, PartialEq, Clone)]
pub enum Direction {
    Unspecified = 0,
}

const DIRECTION_UNSPECIFIED_STR: &str = "Unspecified";

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            _ => DIRECTION_UNSPECIFIED_STR,
        };
        write!(f, "{}", s)
    }
}

impl Default for Direction {
    fn default() -> Direction {
        Direction::Unspecified
    }
}
