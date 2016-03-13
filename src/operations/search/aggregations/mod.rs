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
use ::units::{DistanceType, DistanceUnit, Duration, GeoBox, JsonVal, Location};

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

// TODO - deprecated
// impl<'a> ToJson for Aggregations<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         for (k, ref v) in self.0.iter() {
//             d.insert((*k).to_owned(), v.to_json());
//         }
//         Json::Object(d)
//     }
// }

// Result objects

// Metrics result

// #[derive(Debug)]
// pub struct SumResult {
//     pub value: f64
// }

// impl<'a> From<&'a Json> for SumResult {
//     fn from(from: &'a Json) -> SumResult {
//         SumResult {
//             value: get_json_f64!(from, "value")
//         }
//     }
// }

// #[derive(Debug)]
// pub struct AvgResult {
//     pub value: f64
// }

// impl<'a> From<&'a Json> for AvgResult {
//     fn from(from: &'a Json) -> AvgResult {
//         AvgResult {
//             value: get_json_f64!(from, "value")
//         }
//     }
// }

// #[derive(Debug)]
// pub struct StatsResult {
//     pub count: u64,
//     pub min: f64,
//     pub max: f64,
//     pub avg: f64,
//     pub sum: f64
// }

// impl<'a> From<&'a Json> for StatsResult {
//     fn from(from: &'a Json) -> StatsResult {
//         StatsResult {
//             count: get_json_u64!(from, "count"),
//             min: get_json_f64!(from, "min"),
//             max: get_json_f64!(from, "max"),
//             avg: get_json_f64!(from, "avg"),
//             sum: get_json_f64!(from, "sum")
//         }
//     }
// }

// /// Used by the `ExtendedStatsResult`
// #[derive(Debug)]
// pub struct Bounds {
//     pub upper: f64,
//     pub lower: f64
// }

// impl<'a> From<&'a Json> for Bounds {
//     fn from(from: &'a Json) -> Bounds {
//         Bounds {
//             upper: get_json_f64!(from, "upper"),
//             lower: get_json_f64!(from, "lower")
//         }
//     }
// }

// #[derive(Debug)]
// pub struct ExtendedStatsResult {
//     pub count: u64,
//     pub min: f64,
//     pub max: f64,
//     pub avg: f64,
//     pub sum: f64,
//     pub sum_of_squares: f64,
//     pub variance: f64,
//     pub std_deviation: f64,
//     pub std_deviation_bounds: Bounds
// }

// impl<'a> From<&'a Json> for ExtendedStatsResult {
//     fn from(from: &'a Json) -> ExtendedStatsResult {
//         ExtendedStatsResult {
//             count: get_json_u64!(from, "count"),
//             min: get_json_f64!(from, "min"),
//             max: get_json_f64!(from, "max"),
//             avg: get_json_f64!(from, "avg"),
//             sum: get_json_f64!(from, "sum"),
//             sum_of_squares: get_json_f64!(from, "sum_of_squares"),
//             variance: get_json_f64!(from, "variance"),
//             std_deviation: get_json_f64!(from, "std_deviation"),
//             std_deviation_bounds: from.find("std_deviation_bounds")
//                 .expect("No 'std_deviation_bounds'")
//                 .into()
//         }
//     }
// }

// #[derive(Debug)]
// pub struct ValueCountResult {
//     pub value: u64
// }

// impl<'a> From<&'a Json> for ValueCountResult {
//     fn from(from: &'a Json) -> ValueCountResult {
//         ValueCountResult {
//             value: get_json_u64!(from, "value")
//         }
//     }
// }

// #[derive(Debug)]
// pub struct PercentilesResult {
//     pub values: HashMap<String, f64>
// }

// impl<'a> From<&'a Json> for PercentilesResult {
//     fn from(from: &'a Json) -> PercentilesResult {
//         let val_obj = get_json_object!(from, "values");
//         let mut vals = HashMap::with_capacity(val_obj.len());

//         for (k, v) in val_obj.into_iter() {
//             vals.insert(k.clone(), v.as_f64().expect("Not numeric value"));
//         }

//         PercentilesResult {
//             values: vals
//         }
//     }
// }

// #[derive(Debug)]
// pub struct PercentileRanksResult {
//     pub values: HashMap<String, f64>
// }

// impl<'a> From<&'a Json> for PercentileRanksResult {
//     fn from(from: &'a Json) -> PercentileRanksResult {
//         let val_obj = get_json_object!(from, "values");
//         let mut vals = HashMap::with_capacity(val_obj.len());

//         for (k, v) in val_obj.into_iter() {
//             vals.insert(k.clone(), v.as_f64().expect("Not numeric value"));
//         }

//         PercentileRanksResult {
//             values: vals
//         }
//     }
// }

// #[derive(Debug)]
// pub struct CardinalityResult {
//     pub value: u64
// }

// impl<'a> From<&'a Json> for CardinalityResult {
//     fn from(from: &'a Json) -> CardinalityResult {
//         CardinalityResult {
//             value: get_json_u64!(from, "value")
//         }
//     }
// }

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

// // Date range result objects

// #[derive(Debug)]
// pub struct DateRangeBucketResult {
//     pub from:           Option<f64>,
//     pub from_as_string: Option<String>,
//     pub to:             Option<f64>,
//     pub to_as_string:   Option<String>,
//     pub doc_count:      u64,
//     pub aggs:           Option<AggregationsResult>
// }

// impl DateRangeBucketResult {
//     fn from(from: &Json, aggs: &Option<Aggregations>) -> DateRangeBucketResult {
//         DateRangeBucketResult {
//             from:           optional_json_f64!(from, "from"),
//             from_as_string: optional_json_string!(from, "from_as_string"),
//             to:             optional_json_f64!(from, "to"),
//             to_as_string:   optional_json_string!(from, "to_as_string"),
//             doc_count:      get_json_u64!(from, "doc_count"),
//             aggs:           extract_aggs!(from, aggs)
//         }
//     }

//     add_aggs_ref!();
// }

// #[derive(Debug)]
// pub struct DateRangeResult {
//     pub buckets: Vec<DateRangeBucketResult>
// }

// impl DateRangeResult {
//     fn from(from: &Json, aggs: &Option<Aggregations>) -> DateRangeResult {
//         DateRangeResult {
//             buckets: from.find("buckets").expect("No buckets")
//                 .as_array().expect("Not an array")
//                 .iter().map(|bucket| {
//                     DateRangeBucketResult::from(bucket, aggs)
//                 }).collect()
//         }
//     }
// }

// /// Used for histogram results
// #[derive(Debug)]
// pub struct HistogramBucketResult {
//     pub key: String,
//     pub doc_count: u64,
//     pub aggs: Option<AggregationsResult>
// }

// impl HistogramBucketResult {
//     fn from(from: &Json, aggs: &Option<Aggregations>) -> HistogramBucketResult {
//         HistogramBucketResult {
//             key: get_json_string!(from, "key"),
//             doc_count: get_json_u64!(from, "doc_count"),
//             aggs: extract_aggs!(from, aggs)
//         }
//     }

//     add_aggs_ref!();
// }

// #[derive(Debug)]
// pub struct HistogramResult {
//     pub buckets: Vec<HistogramBucketResult>
// }

// impl HistogramResult {
//     fn from(from: &Json, aggs: &Option<Aggregations>) -> HistogramResult {
//         HistogramResult {
//             buckets: from.find("buckets").expect("No buckets")
//                 .as_array().expect("Not an array")
//                 .iter().map(|bucket| HistogramBucketResult::from(bucket, aggs))
//                 .collect()
//         }
//     }
// }

// // Date histogram results
// #[derive(Debug)]
// pub struct DateHistogramBucketResult {
//     pub key_as_string: String,
//     pub key: u64,
//     pub doc_count: u64,
//     pub aggs: Option<AggregationsResult>
// }

// impl DateHistogramBucketResult {
//     fn from(from: &Json, aggs: &Option<Aggregations>) -> DateHistogramBucketResult {
//         DateHistogramBucketResult {
//             key_as_string: get_json_string!(from, "key_as_string"),
//             key: get_json_u64!(from, "key"),
//             doc_count: get_json_u64!(from, "doc_count"),
//             aggs: extract_aggs!(from, aggs)
//         }
//     }

//     add_aggs_ref!();
// }

// #[derive(Debug)]
// pub struct DateHistogramResult {
//     pub buckets: Vec<DateHistogramBucketResult>
// }

// impl DateHistogramResult {
//     fn from(from: &Json, aggs: &Option<Aggregations>) -> DateHistogramResult {
//         DateHistogramResult {
//             buckets: from.find("buckets").expect("No buckets")
//                 .as_array().expect("Not an array")
//                 .iter().map(|bucket| DateHistogramBucketResult::from(bucket, aggs))
//                 .collect()
//         }
//     }
// }

// // GeoDistance results
// #[derive(Debug)]
// pub struct GeoDistanceBucketResult {
//     pub key: String,
//     pub from: Option<f64>,
//     pub to: Option<f64>,
//     pub doc_count: u64,
//     pub aggs: Option<AggregationsResult>
// }

// impl GeoDistanceBucketResult {
//     fn from(from: &Json, aggs: &Option<Aggregations>) -> GeoDistanceBucketResult {
//         GeoDistanceBucketResult {
//             key: get_json_string!(from, "key"),
//             from: optional_json_f64!(from, "from"),
//             to: optional_json_f64!(from, "to"),
//             doc_count: get_json_u64!(from, "doc_count"),
//             aggs: extract_aggs!(from, aggs)
//         }
//     }

//     add_aggs_ref!();
// }

// #[derive(Debug)]
// pub struct GeoDistanceResult {
//     pub buckets: Vec<GeoDistanceBucketResult>
// }

// impl GeoDistanceResult {
//     fn from(from: &Json, aggs: &Option<Aggregations>) -> GeoDistanceResult {
//         GeoDistanceResult {
//             buckets: from.find("buckets").expect("No buckets")
//                 .as_array().expect("Not an array")
//                 .iter().map(|bucket| GeoDistanceBucketResult::from(bucket, aggs))
//                 .collect()
//         }
//     }
// }

// #[derive(Debug)]
// pub struct GeoHashBucketResult {
//     pub key:       String,
//     pub doc_count: u64,
//     pub aggs:      Option<AggregationsResult>
// }

// impl GeoHashBucketResult {
//     fn from(from: &Json, aggs: &Option<Aggregations>) -> GeoHashBucketResult {
//         GeoHashBucketResult {
//             key: get_json_string!(from, "key"),
//             doc_count: get_json_u64!(from, "doc_count"),
//             aggs: extract_aggs!(from, aggs)
//         }
//     }

//     add_aggs_ref!();
// }

// #[derive(Debug)]
// pub struct GeoHashResult {
//     pub buckets: Vec<GeoHashBucketResult>
// }

// impl GeoHashResult {
//     fn from(from: &Json, aggs: &Option<Aggregations>) -> GeoHashResult {
//         GeoHashResult {
//             buckets: from.find("buckets").expect("No buckets")
//                 .as_array().expect("Not an array")
//                 .iter().map(|bucket| GeoHashBucketResult::from(bucket, aggs))
//                 .collect()
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
                use self::bucket::BucketAggregation::*;
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
