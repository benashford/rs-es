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

//! Implementation of ElasticSearch [aggregations](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations.html)

#[macro_use]
mod common;

pub mod metrics;
pub mod bucket;

use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

// TODO - deprecated
use rustc_serialize::json::{Json, ToJson};

use serde::de::Deserialize;
use serde::ser;
use serde::ser::{Serialize, Serializer};
use serde_json::{to_value, Value};

use ::error::EsError;
use ::query;

use self::bucket::BucketAggregationResult;
use self::metrics::MetricsAggregationResult;

// TODO - deprecated
#[derive(Debug)]
pub enum Scripts<'a> {
    Inline(&'a str, Option<&'a str>),
    Id(&'a str)
}

// TODO - deprecated
/// Script attributes for various attributes
#[derive(Debug)]
pub struct Script<'a> {
    script: Scripts<'a>,
    params: Option<Json>
}

// TODO - deprecated
// impl<'a> Script<'a> {
//     pub fn id(script_id: &'a str) -> Script<'a> {
//         Script {
//             script: Scripts::Id(script_id),
//             params: None
//         }
//     }

//     pub fn script(script: &'a str) -> Script<'a> {
//         Script {
//             script: Scripts::Inline(script, None),
//             params: None
//         }
//     }

//     pub fn script_and_field(script: &'a str, field: &'a str) -> Script<'a> {
//         Script {
//             script: Scripts::Inline(script, Some(field)),
//             params: None
//         }
//     }

//     fn add_to_object(&self, obj: &mut BTreeMap<&'static str, Value>) {
//         match self.script {
//             Scripts::Inline(script, field) => {
//                 obj.insert("script", to_value(script));
//                 match field {
//                     Some(f) => {
//                         obj.insert("field", to_value(f));
//                     },
//                     None    => ()
//                 }
//             },
//             Scripts::Id(script_id) => {
//                 obj.insert("script_id", to_value(script_id));
//             }
//         };
//         // TODO - fix this
//         // match self.params {
//         //     Some(ref json) => {
//         //         obj.insert("params".to_owned(), json.clone());
//         //     },
//         //     None           => ()
//         // };
//     }

//     pub fn with_params(mut self, params: Json) -> Self {
//         self.params = Some(params);
//         self
//     }
// }

/// A common pattern is for an aggregation to accept a field or a script
#[derive(Debug)]
// TODO - deprecated
pub enum FieldOrScript<'a> {
    Field(&'a str),
    Script(Script<'a>)
}

impl<'a> FieldOrScript<'a> {
    // TODO - deprecated
    // fn add_to_object(&self, obj: &mut BTreeMap<String, Json>) {
    //     match self {
    //         &FieldOrScript::Field(field) => {
    //             obj.insert("field".to_owned(), field.to_json());
    //         },
    //         &FieldOrScript::Script(ref script) => {
    //             script.add_to_object(obj);
    //         }
    //     }
    // }
}

// impl<'a> From<&'a str> for FieldOrScript<'a> {
//     fn from(from: &'a str) -> FieldOrScript<'a> {
//         FieldOrScript::Field(from)
//     }
// }

// impl<'a> From<Script<'a>> for FieldOrScript<'a> {
//     fn from(from: Script<'a>) -> FieldOrScript<'a> {
//         FieldOrScript::Script(from)
//     }
// }

// /// Macros to build simple `FieldOrScript` based aggregations
// macro_rules! field_or_script_new {
//     ($t:ident) => {
//         impl<'a> $t<'a> {
//             pub fn new<FOS: Into<FieldOrScript<'a>>>(fos: FOS) -> $t<'a> {
//                 $t(fos.into())
//             }
//         }
//     }
// }

// // TODO - deprecated
// macro_rules! field_or_script_to_json {
//     ($t:ident) => {
//         impl<'a> ToJson for $t<'a> {
//             fn to_json(&self) -> Json {
//                 // DEPRECATED
//                 // let mut d = BTreeMap::new();
//                 // self.0.add_to_object(&mut d);
//                 // Json::Object(d)
//                 unimplemented!()
//             }
//         }
//     }
// }

/// Aggregations are either metrics or bucket-based aggregations
#[derive(Debug)]
pub enum Aggregation<'a> {
    /// A metric aggregation (e.g. min)
    Metrics(metrics::MetricsAggregation<'a>),

    /// A bucket aggregation, groups data into buckets and optionally applies
    /// sub-aggregations
    Bucket(bucket::BucketAggregation<'a>, Option<Aggregations<'a>>)
}

impl<'a> Serialize for Aggregation<'a> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        serializer.serialize_struct("Aggregation", AggregationVisitor {
            agg: self,
            state: 0
        })
    }
}

struct AggregationVisitor<'a> {
    agg: &'a Aggregation<'a>,
    state: u8
}

impl<'a> ser::MapVisitor for AggregationVisitor<'a> {
    fn visit<S>(&mut self, serializer: &mut S) -> Result<Option<()>, S::Error>
        where S: Serializer {
        use self::Aggregation::*;

        self.state += 1;
        match self.state {
            1 => match self.agg {
                &Metrics(ref metric_agg) => {
                    let agg_name = metric_agg.details();
                    Ok(Some(try!(serializer.serialize_map_elt(agg_name, metric_agg))))
                },
                &Bucket(ref bucket_agg, _) => {
                    let agg_name = bucket_agg.details();
                    Ok(Some(try!(serializer.serialize_map_elt(agg_name, bucket_agg))))
                }
            },
            2 => match self.agg {
                &Metrics(_) => Ok(Some(())),
                &Bucket(_, ref opt_aggs) => {
                    match opt_aggs {
                        &Some(ref other_aggs) => {
                            Ok(Some(try!(serializer.serialize_map_elt("aggregations",
                                                                      other_aggs))))
                        },
                        &None => Ok(Some(()))
                    }
                }
            },
            _ => Ok(None)
        }
    }
}

// TODO - deprecated
// impl<'a> ToJson for Aggregation<'a> {
//     fn to_json(&self) -> Json {
//         match self {
//             &Aggregation::Metrics(ref ma)          => {
//                 ma.to_json()
//             },
//             &Aggregation::Bucket(ref ba, ref aggs) => {
//                 let mut d = BTreeMap::new();
//                 ba.add_to_object(&mut d);
//                 match aggs {
//                     &Some(ref a) => {
//                         d.insert("aggs".to_owned(), a.to_json());
//                     },
//                     &None        => ()
//                 }
//                 Json::Object(d)
//             }
//         }
//     }
// }

/// The set of aggregations
///
/// There are many ways of creating aggregations, either standalone or via a
/// conversion trait
#[derive(Debug, Serialize)]
pub struct Aggregations<'a>(HashMap<&'a str, Aggregation<'a>>);

impl<'a> Aggregations<'a> {
    /// Create an empty-set of aggregations, individual aggregations should be
    /// added via the `add` method
    ///
    /// # Examples
    ///
    /// ```
    /// use rs_es::operations::search::aggregations::Aggregations;
    /// use rs_es::operations::search::aggregations::metrics::Min;
    ///
    /// let mut aggs = Aggregations::new();
    /// aggs.add("agg_name", Min::field("field_name"));
    /// ```
    pub fn new() -> Aggregations<'a> {
        Aggregations(HashMap::new())
    }

    /// Add an aggregation to the set of aggregations
    pub fn add<A: Into<Aggregation<'a>>>(&mut self, key: &'a str, val: A) {
        self.0.insert(key, val.into());
    }
}

impl<'b> From<Vec<(&'b str, Aggregation<'b>)>> for Aggregations<'b> {
    fn from(from: Vec<(&'b str, Aggregation<'b>)>) -> Aggregations<'b> {
        let mut aggs = Aggregations::new();
        for (name, agg) in from {
            aggs.add(name, agg);
        }
        aggs
    }
}

impl <'a, A: Into<Aggregation<'a>>> From<(&'a str, A)> for Aggregations<'a> {
    fn from(from: (&'a str, A)) -> Aggregations<'a> {
        let mut aggs = Aggregations::new();
        aggs.add(from.0, from.1.into());
        aggs
    }
}

// Result objects

// Metrics result

// #[derive(Debug)]
// pub struct GeoBoundsResult {
//     pub bounds: GeoBox
// }

// impl<'a> From<&'a Json> for GeoBoundsResult {
//     fn from(from: &'a Json) -> GeoBoundsResult {
//         GeoBoundsResult {
//             bounds: GeoBox::from(from.find("bounds").expect("No 'bounds' field"))
//         }
//     }
// }

// #[derive(Debug)]
// pub struct ScriptedMetricResult {
//     pub value: JsonVal
// }

// impl<'a> From<&'a Json> for ScriptedMetricResult {
//     fn from(from: &'a Json) -> ScriptedMetricResult {
//         ScriptedMetricResult {
//             value: JsonVal::from(from.find("value").expect("No 'value' field"))
//         }
//     }
// }

/// The result of one specific aggregation
///
/// The data returned varies depending on aggregation type
#[derive(Debug)]
pub enum AggregationResult {
    /// Results of metrics aggregations
    Metrics(MetricsAggregationResult),

    /// Result of a bucket aggregation
    Bucket(BucketAggregationResult)
}

#[derive(Debug)]
pub struct AggregationsResult(HashMap<String, AggregationResult>);

/// Loads a Json object of aggregation results into an `AggregationsResult`.
fn object_to_result(aggs: &Aggregations,
                    object: &BTreeMap<String, Value>) -> Result<AggregationsResult, EsError> {
    use self::Aggregation::*;

    let mut ar_map = HashMap::new();
    for (&key, val) in aggs.0.iter() {
        let owned_key = key.to_owned();
        let json = match object.get(&owned_key) {
            Some(json) => json,
            None => return Err(EsError::EsError(format!("No key: {}", &owned_key)))
        };
        ar_map.insert(owned_key, match val {
            &Metrics(ref ma) => {
                AggregationResult::Metrics(try!(MetricsAggregationResult::from(ma, json)))
            },
            &Aggregation::Bucket(ref ba, ref aggs) => {
                AggregationResult::Bucket(try!(BucketAggregationResult::from(ba,
                                                                             json,
                                                                             aggs)))
            }
        });
    }

    info!("Processed aggs - From: {:?}. To: {:?}", object, ar_map);

    Ok(AggregationsResult(ar_map))
}

impl AggregationsResult {
    pub fn get<'a>(&'a self, key: &str) -> Result<&'a AggregationResult, EsError> {
        match self.0.get(key) {
            Some(ref agg_res) => Ok(agg_res),
            None              => Err(EsError::EsError(format!("No agg for key: {}",
                                                              key)))
        }
    }

    pub fn from(aggs: &Aggregations,
                json: &Value) -> Result<AggregationsResult, EsError> {
        println!("Parsing aggregations {:?}", json);
        let object = match json.as_object() {
            Some(o) => o,
            None    => return Err(EsError::EsError("Aggregations is not an object".to_owned()))
        };
        object_to_result(aggs, object)
    }
}
