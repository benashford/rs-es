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

//! Features common to all operations

use std::fmt;

use serde::ser::{Serialize, Serializer};

use util::StrJoin;

/// A newtype for the value of a URI option, this is to allow conversion traits
/// to be implemented for it
pub struct OptionVal(pub String);

/// Conversion from `&str` to `OptionVal`
impl<'a> From<&'a str> for OptionVal {
    fn from(from: &'a str) -> OptionVal {
        OptionVal(from.to_owned())
    }
}

/// Basic types have conversions to `OptionVal`
from_exp!(String, OptionVal, from, OptionVal(from));
from_exp!(i32, OptionVal, from, OptionVal(from.to_string()));
from_exp!(i64, OptionVal, from, OptionVal(from.to_string()));
from_exp!(u32, OptionVal, from, OptionVal(from.to_string()));
from_exp!(u64, OptionVal, from, OptionVal(from.to_string()));
from_exp!(bool, OptionVal, from, OptionVal(from.to_string()));

/// Every ES operation has a set of options
pub struct Options<'a>(pub Vec<(&'a str, OptionVal)>);

impl<'a> Options<'a> {
    pub fn new() -> Options<'a> {
        Options(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Add a value
    ///
    /// ```
    /// use rs_es::operations::common::Options;
    /// let mut options = Options::new();
    /// options.push("a", 1);
    /// options.push("b", "2");
    /// ```
    pub fn push<O: Into<OptionVal>>(&mut self, key: &'a str, val: O) {
        self.0.push((key, val.into()));
    }
}

impl<'a> fmt::Display for Options<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if !self.is_empty() {
            try!(formatter.write_str("?"));
            try!(formatter.write_str(&self.0.iter().map(|&(ref k, ref v)| {
                format!("{}={}", k, v.0)
            }).join("&")));
        }
        Ok(())
    }
}

/// Adds a function to an operation to add specific query-string options to that
/// operations builder interface.
macro_rules! add_option {
    ($n:ident, $e:expr) => (
        pub fn $n<T: Into<OptionVal>>(&'a mut self, val: T) -> &'a mut Self {
            self.options.push($e, val);
            self
        }
    )
}

/// The [`version_type` field](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-index_.html#index-versioning)
pub enum VersionType {
    Internal,
    External,
    ExternalGt,
    ExternalGte,
    Force
}

impl Serialize for VersionType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {

        self.to_string().serialize(serializer)
    }
}

impl ToString for VersionType {
    fn to_string(&self) -> String {
        match *self {
            VersionType::Internal => "internal",
            VersionType::External => "external",
            VersionType::ExternalGt => "external_gt",
            VersionType::ExternalGte => "external_gte",
            VersionType::Force => "force"
        }.to_owned()
    }
}

from_exp!(VersionType, OptionVal, from, OptionVal(from.to_string()));

/// The consistency query parameter
pub enum Consistency {
    One,
    Quorum,
    All
}

impl From<Consistency> for OptionVal {
    fn from(from: Consistency) -> OptionVal {
        OptionVal(match from {
            Consistency::One => "one",
            Consistency::Quorum => "quorum",
            Consistency::All => "all"
        }.to_owned())
    }
}

/// Values for `default_operator` query parameters
pub enum DefaultOperator {
    And,
    Or
}

impl From<DefaultOperator> for OptionVal {
    fn from(from: DefaultOperator) -> OptionVal {
        OptionVal(match from {
            DefaultOperator::And => "and",
            DefaultOperator::Or => "or"
        }.to_owned())
    }
}
