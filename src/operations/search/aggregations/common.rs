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

use serde::ser;
use serde::ser::{Serialize, Serializer};

use ::json::MergeSerializer;
use ::units::JsonVal;

use super::Aggregation;

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
            fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
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
pub struct Agg<'a, E> {
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
    where E: Serialize {

    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        serializer.serialize_struct("Agg", AggVisitor {
            ma: self,
            state: 0
        })
    }
}

struct AggVisitor<'a, E: 'a> {
    ma: &'a Agg<'a, E>,
    state: u8
}

fn visit_field<S, T>(field: Option<T>,
                     field_name: &str,
                     serializer: &mut S) -> Result<Option<()>, S::Error>
    where S: Serializer,
          T: Serialize {

    match field {
        Some(value) => {
            Ok(Some(try!(serializer.serialize_map_elt(field_name, value))))
        },
        None => Ok(Some(()))
    }
}

impl<'a, E> ser::MapVisitor for AggVisitor<'a, E>
    where E: Serialize {

    fn visit<S>(&mut self, serializer: &mut S) -> Result<Option<()>, S::Error>
        where S: Serializer {

        self.state += 1;
        match self.state {
            1 => visit_field(self.ma.field, "field", serializer),
            2 => visit_field(self.ma.script.inline, "inline", serializer),
            3 => visit_field(self.ma.script.file, "file", serializer),
            4 => visit_field(self.ma.script.id, "id", serializer),
            5 => visit_field(self.ma.script.params.as_ref(), "params", serializer),
            6 => visit_field(self.ma.missing.as_ref(), "missing", serializer),
            7 => {
                let mut merge_serializer = MergeSerializer::new(serializer);
                Ok(Some(try!(self.ma.extra.serialize(&mut merge_serializer))))
            },
            _ => Ok(None)
        }
    }
}
