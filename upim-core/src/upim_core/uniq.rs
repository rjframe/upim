//! Uniq iterator
//!
//! Filters out duplicate elements of an iterator.

use std::collections::BTreeSet;


pub(crate) struct UniqIterator<I, T> {
    source: I,
    seen: BTreeSet<T>,
}

impl<I, T> Iterator for UniqIterator<I, T>
    where I: Iterator + Iterator<Item = T>,
          T: Copy + Ord,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let seen = &mut self.seen;

        match self.source.find(|i| { ! seen.contains(i) }) {
            Some(i) => {
                seen.insert(i);
                Some(i)
            },
            None => None,
        }
    }
}

pub(crate) trait Uniq<I, T>: Iterator {
    fn uniq(self) -> UniqIterator<Self, T>
        where Self: Sized + Iterator<Item = T>,
              T: Ord,
    {
        UniqIterator {
            source: self,
            seen: BTreeSet::new(),
        }
    }
}

impl<I, T> Uniq<I, T> for I where I: Iterator {}
