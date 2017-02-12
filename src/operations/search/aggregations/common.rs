/*
 * Copyright 2016 Ben Ashford
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

//! Features that are common to all aggregations

use std::collections::HashMap;

use serde::ser::{Serialize, Serializer, SerializeMap};

use ::json::{MergeSerialize, serialize_map_optional_kv};
use ::units::JsonVal;

macro_rules! agg {
    ($b:ident) => {
        impl<'a> $b<'a> {
            pub fn field(field: &'a str) -> Self {
                $b(Agg {
                    field: Some(field),
                    ..Default::default()
                })
            }

            pub fn script<S: Into<Script<'a>>>(script: S) -> Self {
                $b(Agg {
                    script: script.into(),
                    ..Default::default()
                })
            }

            pub fn with_script<S: Into<Script<'a>>>(mut self, script: S) -> Self {
                self.0.script = script.into();
                self
            }

            pub fn with_missing<J: Into<JsonVal>>(mut self, missing: J) -> Self {
                self.0.missing = Some(missing.into());
                self
            }
        }

        impl<'a> Serialize for $b<'a> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where S: Serializer {

                self.0.serialize(serializer)
            }
        }
    }
}

/// Scripts used in aggregations
#[derive(Debug, Default)]
pub struct Script<'a> {
    pub inline: Option<&'a str>,
    pub file: Option<&'a str>,
    pub id: Option<&'a str>,
    pub params: Option<HashMap<&'a str, JsonVal>>
}

/// Base of all Metrics aggregations
#[derive(Debug, Default)]
pub struct Agg<'a, E>
    where E: MergeSerialize {

    pub field: Option<&'a str>,
    pub script: Script<'a>,
    pub missing: Option<JsonVal>,
    pub extra: E
}

macro_rules! add_extra_option {
    ($n:ident, $e:ident, $t:ty) => {
        pub fn $n<T: Into<$t>>(mut self, val: T) -> Self {
            self.0.extra.$e = Some(val.into());
            self
        }
    }
}

impl<'a, E> Serialize for Agg<'a, E>
    where E: MergeSerialize {

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {

        let mut map = try!(serializer.serialize_map(None));

        try!(serialize_map_optional_kv(&mut map, "field", &self.field));
        try!(serialize_map_optional_kv(&mut map, "inline", &self.script.inline));
        try!(serialize_map_optional_kv(&mut map, "file", &self.script.file));
        try!(serialize_map_optional_kv(&mut map, "id", &self.script.id));
        try!(serialize_map_optional_kv(&mut map, "params", &self.script.params));
        try!(serialize_map_optional_kv(&mut map, "missing", &self.missing));
        try!(self.extra.merge_serialize(&mut map));

        map.end()
    }
}

// Useful for results

/// Macro to implement the various as... functions that return the details of an
/// aggregation for that particular type
macro_rules! agg_as {
    ($n:ident,$st:ident,$tp:ident,$t:ident,$rt:ty) => {
        pub fn $n<'a>(&'a self) -> Result<&'a $rt, EsError> {
            match self {
                &AggregationResult::$st(ref res) => {
                    match res {
                        &$tp::$t(ref res) => Ok(res),
                        _ => Err(EsError::EsError(format!("Wrong type: {:?}", self)))
                    }
                },
                _ => Err(EsError::EsError(format!("Wrong type: {:?}", self)))
            }
        }
    }
}
