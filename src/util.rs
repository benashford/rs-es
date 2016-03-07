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

// Macro to add an Optional from a struct to a BTreeMap if it's set.
//
// This is a recurring pattern when creating JSON.
macro_rules! optional_add {
    ($slf:expr, $map:ident, $sn:ident, $field:expr, $val: ident, $ex:expr) => {
        match $slf.$sn {
            Some(ref $val) => { $map.insert($field.to_owned(), $ex); }
            _              => ()
        }
    };
    ($slf:expr, $map:ident, $sn:ident, $field:expr) => {
        optional_add!($slf, $map, $sn, $field, value, value.to_json());
    };
    ($slf:expr, $map:ident, $sn:ident) => {
        optional_add!($slf, $map, $sn, stringify!($sn));
    };
}

// Macros to read values from Json structs
macro_rules! get_json_thing {
    ($r:ident,$f:expr,$t:ident) => {
        $r.find($f)
            .expect(concat!("No field '", stringify!($f), "'"))
            .$t()
            .expect(concat!("Field '",
                            stringify!($f),
                            "' is not of type ",
                            stringify!($t)))
    }
}

macro_rules! get_json_object {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_object)
    }
}

macro_rules! get_json_array {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_array)
    }
}

macro_rules! get_json_string {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_string).to_owned()
    }
}

macro_rules! get_json_i64 {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_i64)
    }
}

macro_rules! get_json_u64 {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_u64)
    }
}

macro_rules! get_json_bool {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_boolean)
    }
}

macro_rules! get_json_f64 {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_f64)
    }
}

/// Optional extraction, this is opposed to the `get_json_*` macros that will
/// panic.
macro_rules! optional_json_thing {
    ($r:ident,$f:expr,$t:ident) => {
        $r.find($f).and_then(|v| {
            v.$t()
        })
    }
}

macro_rules! optional_json_string {
    ($r:ident,$f:expr) => {
        optional_json_thing!($r, $f, as_string).and_then(|str| {
            Some(str.to_owned())
        })
    }
}

macro_rules! optional_json_f64 {
    ($r:ident,$f:expr) => {
        optional_json_thing!($r, $f, as_f64).and_then(|f| Some(f))
    }
}

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

impl<I, S> StrJoin for I where
    S: AsRef<str>,
    I: Iterator<Item=S> {
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
    }
}

macro_rules! from {
    ($ft:ty, $dt:ident, $ev:ident, $pi:ident) => {
        from_exp!($ft, $dt, $pi, $dt::$ev($pi));
    };
    ($ft:ty, $dt:ident, $ev:ident) => {
        from!($ft, $dt, $ev, from);
    };
}
