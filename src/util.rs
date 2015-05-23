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

// Miscellaneous code used in numerous places

use std::iter::Iterator;

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

// A custom String-join trait as the stdlib one is currently marked as unstable.
pub trait StrJoin {
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
