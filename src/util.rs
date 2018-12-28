/*
 * Copyright 2015-2016 Ben Ashford
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Miscellaneous code used in numerous places

use std::iter::Iterator;

/// A custom String-join trait as the stdlib one is currently marked as unstable.
pub trait StrJoin {
    /// Join an iterator of things that can be referenced as strings into a
    /// single owned-string by the given joining string
    ///
    /// # Example
    ///
    /// ```
    /// use rs_es::util::StrJoin;
    ///
    /// let data = vec!["a", "b", "c", "d"];
    /// assert_eq!("a-b-c-d", data.iter().join("-"));
    /// ```
    ///
    /// This will print: `a-b-c-d`
    ///
    fn join(self, join: &str) -> String;
}

impl<I, S> StrJoin for I
where
    S: AsRef<str>,
    I: Iterator<Item = S>,
{
    fn join(self, join: &str) -> String {
        let mut s = String::new();
        for f in self {
            s.push_str(f.as_ref());
            s.push_str(join);
        }
        s.pop();
        s
    }
}

/// Useful macro for adding a function to supply a value to an optional field
macro_rules! add_field {
    ($n:ident, $f:ident, $t:ty) => (
        pub fn $n<T: Into<$t>>(mut self, val: T) -> Self {
            self.$f = Some(val.into());
            self
        }
    );
}

/// Useful macros for implementing `From` traits
///
/// TODO: this may only be useful for Query DSL, in which case should be moved
/// to that module
macro_rules! from_exp {
    ($ft:ty, $dt:ident, $pi:ident, $ex:expr) => {
        impl From<$ft> for $dt {
            fn from($pi: $ft) -> $dt {
                $ex
            }
        }
    };
}

macro_rules! from {
    ($ft:ty, $dt:ident, $ev:ident, $pi:ident) => {
        from_exp!($ft, $dt, $pi, $dt::$ev($pi));
    };
    ($ft:ty, $dt:ident, $ev:ident) => {
        from!($ft, $dt, $ev, from);
    };
}
