/*
 * Copyright 2015 Ben Ashford
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

// Macro to add an Optional to a BTreeMap if it's set
macro_rules! optional_add {
    ($map:ident, $sn:expr, $field:expr, $val: ident, $ex:expr) => {
        match $sn {
            Some(ref $val) => { $map.insert($field.to_string(), $ex); }
            _              => ()
        }
    };
    ($map:ident, $sn:expr, $field:expr) => {
        optional_add!($map, $sn, $field, value, value.to_json());
    };
}

// Macros to read values from Json structs
macro_rules! get_json_thing {
    ($r:ident,$f:expr,$t:ident) => {
        $r.find($f).unwrap().$t().unwrap()
    }
}

macro_rules! get_json_string {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_string).to_string()
    }
}

macro_rules! get_json_i64 {
    ($r:ident,$f:expr) => {
        get_json_thing!($r,$f,as_i64)
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

/// A custom String-join trait as the stdlib one is currently marked as unstable.
pub trait StrJoin {
    /// Join an iterator of things that can be referenced as strings into a
    /// single owned-string by the given joining string
    ///
    /// # Example
    ///
    /// ```
    /// use util::StrJoin;
    ///
    /// let data = vec!["a", "b", "c", "d"];
    /// println!("Joined: {}", data.iter().join("-"));
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
