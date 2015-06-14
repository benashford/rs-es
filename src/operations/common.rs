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

//! Features common to all operations

use rustc_serialize::json::{Json, ToJson};

/// Every ES operation has a set of options
pub type Options<'a> = Vec<(&'a str, String)>;

/// Adds a function to an operation to add specific query-string options to that
/// operations builder interface.
macro_rules! add_option {
    ($n:ident, $e:expr) => (
        pub fn $n<T: ToString>(&'a mut self, val: &T) -> &'a mut Self {
            self.options.push(($e, val.to_string()));
            self
        }
    )
}

/// Broadly similar to the `add_option` macro for query-string options, but for
/// specifying fields in the Bulk request
macro_rules! add_field {
    ($n:ident, $f:ident, $t:ty) => (
        pub fn $n<T: Into<$t>>(mut self, val: T) -> Self {
            self.$f = Some(val.into());
            self
        }
    )
}

/// The [`version_type` field](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-index_.html#index-versioning)
#[allow(dead_code)]
pub enum VersionType {
    Internal,
    External,
    ExternalGt,
    ExternalGte,
    Force
}

impl ToString for VersionType {
    fn to_string(&self) -> String {
        match *self {
            VersionType::Internal => "internal",
            VersionType::External => "external",
            VersionType::ExternalGt => "external_gt",
            VersionType::ExternalGte => "external_gte",
            VersionType::Force => "force"
        }.to_string()
    }
}

impl ToJson for VersionType {
    fn to_json(&self) -> Json {
        Json::String(self.to_string())
    }
}
