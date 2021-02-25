//! Generic Either type

/// Generic "either one or the other" type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Either<T, U> {
    Left(T),
    Right(U),
}

impl<T, U> Either<T, U> {
    /// Check whether this container stores the Left type.
    pub fn is_left(&self) -> bool {
        matches!(self, Either::Left(_))
    }

    /// Check whether this container stores the Right type.
    pub fn is_right(&self) -> bool {
        matches!(self, Either::Right(_))
    }

    /// Return the stored item if it is in Left.
    pub fn left(&self) -> Option<&T> {
        match &self {
            Either::Left(ref v) => Some(v),
            _ => None,
        }
    }

    /// Return the stored item if it is in Right.
    pub fn right(&self) -> Option<&U> {
        match &self {
            Either::Right(ref v) => Some(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn either_is_methods() {
        let left = Either::<String, i32>::Left("s".to_owned());
        let right = Either::<String, i32>::Right(5);

        assert!(left.is_left());
        assert!(! left.is_right());
        assert!(right.is_right());
        assert!(! right.is_left());
    }

    #[test]
    fn either_get_left_right() {
        let left = Either::<String, i32>::Left("s".to_owned());
        let right = Either::<String, i32>::Right(5);

        assert_eq!(left.left(), Some(&"s".to_owned()));
        assert_eq!(left.right(), None);
        assert_eq!(right.right(), Some(&5));
        assert_eq!(right.left(), None);
    }
}
