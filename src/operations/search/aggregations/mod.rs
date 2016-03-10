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

pub mod metrics;
pub mod bucket;

use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

// TODO - deprecated
use rustc_serialize::json::{Json, ToJson};

use serde::de::Deserialize;
use serde::ser::{Serialize, Serializer};
use serde_json::{to_value, Value};

use error::EsError;
use query;
use units::{DistanceType, DistanceUnit, Duration, GeoBox, JsonVal, Location};

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
        use self::Aggregation::*;

        let m:BTreeMap<&'static str, Value> = match self {
            &Metrics(ref a) => a.to_map(),
            &Bucket(ref b, ref aggs) => {
                let mut m:BTreeMap<&'static str, Value> = b.to_map();
                match aggs {
                    &Some(ref a) => {
                        m.insert("aggs", to_value(a));
                    },
                    &None => ()
                }
                m
            }
        };
        m.serialize(serializer)
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

#[derive(Debug)]
pub struct MinResult {
    pub value: JsonVal
}

impl<'a> From<&'a Json> for MinResult {
    fn from(from: &'a Json) -> MinResult {
        MinResult {
            value: JsonVal::from(from.find("value").expect("No 'value' value"))
        }
    }
}

#[derive(Debug)]
pub struct MaxResult {
    pub value: JsonVal
}

impl<'a> From<&'a Json> for MaxResult {
    fn from(from: &'a Json) -> MaxResult {
        MaxResult {
            value: JsonVal::from(from.find("value").expect("No 'value' value"))
        }
    }
}

#[derive(Debug)]
pub struct SumResult {
    pub value: f64
}

impl<'a> From<&'a Json> for SumResult {
    fn from(from: &'a Json) -> SumResult {
        SumResult {
            value: get_json_f64!(from, "value")
        }
    }
}

#[derive(Debug)]
pub struct AvgResult {
    pub value: f64
}

impl<'a> From<&'a Json> for AvgResult {
    fn from(from: &'a Json) -> AvgResult {
        AvgResult {
            value: get_json_f64!(from, "value")
        }
    }
}

#[derive(Debug)]
pub struct StatsResult {
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub sum: f64
}

impl<'a> From<&'a Json> for StatsResult {
    fn from(from: &'a Json) -> StatsResult {
        StatsResult {
            count: get_json_u64!(from, "count"),
            min: get_json_f64!(from, "min"),
            max: get_json_f64!(from, "max"),
            avg: get_json_f64!(from, "avg"),
            sum: get_json_f64!(from, "sum")
        }
    }
}

/// Used by the `ExtendedStatsResult`
#[derive(Debug)]
pub struct Bounds {
    pub upper: f64,
    pub lower: f64
}

impl<'a> From<&'a Json> for Bounds {
    fn from(from: &'a Json) -> Bounds {
        Bounds {
            upper: get_json_f64!(from, "upper"),
            lower: get_json_f64!(from, "lower")
        }
    }
}

#[derive(Debug)]
pub struct ExtendedStatsResult {
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub sum: f64,
    pub sum_of_squares: f64,
    pub variance: f64,
    pub std_deviation: f64,
    pub std_deviation_bounds: Bounds
}

impl<'a> From<&'a Json> for ExtendedStatsResult {
    fn from(from: &'a Json) -> ExtendedStatsResult {
        ExtendedStatsResult {
            count: get_json_u64!(from, "count"),
            min: get_json_f64!(from, "min"),
            max: get_json_f64!(from, "max"),
            avg: get_json_f64!(from, "avg"),
            sum: get_json_f64!(from, "sum"),
            sum_of_squares: get_json_f64!(from, "sum_of_squares"),
            variance: get_json_f64!(from, "variance"),
            std_deviation: get_json_f64!(from, "std_deviation"),
            std_deviation_bounds: from.find("std_deviation_bounds")
                .expect("No 'std_deviation_bounds'")
                .into()
        }
    }
}

#[derive(Debug)]
pub struct ValueCountResult {
    pub value: u64
}

impl<'a> From<&'a Json> for ValueCountResult {
    fn from(from: &'a Json) -> ValueCountResult {
        ValueCountResult {
            value: get_json_u64!(from, "value")
        }
    }
}

#[derive(Debug)]
pub struct PercentilesResult {
    pub values: HashMap<String, f64>
}

impl<'a> From<&'a Json> for PercentilesResult {
    fn from(from: &'a Json) -> PercentilesResult {
        let val_obj = get_json_object!(from, "values");
        let mut vals = HashMap::with_capacity(val_obj.len());

        for (k, v) in val_obj.into_iter() {
            vals.insert(k.clone(), v.as_f64().expect("Not numeric value"));
        }

        PercentilesResult {
            values: vals
        }
    }
}

#[derive(Debug)]
pub struct PercentileRanksResult {
    pub values: HashMap<String, f64>
}

impl<'a> From<&'a Json> for PercentileRanksResult {
    fn from(from: &'a Json) -> PercentileRanksResult {
        let val_obj = get_json_object!(from, "values");
        let mut vals = HashMap::with_capacity(val_obj.len());

        for (k, v) in val_obj.into_iter() {
            vals.insert(k.clone(), v.as_f64().expect("Not numeric value"));
        }

        PercentileRanksResult {
            values: vals
        }
    }
}

#[derive(Debug)]
pub struct CardinalityResult {
    pub value: u64
}

impl<'a> From<&'a Json> for CardinalityResult {
    fn from(from: &'a Json) -> CardinalityResult {
        CardinalityResult {
            value: get_json_u64!(from, "value")
        }
    }
}

#[derive(Debug)]
pub struct GeoBoundsResult {
    pub bounds: GeoBox
}

impl<'a> From<&'a Json> for GeoBoundsResult {
    fn from(from: &'a Json) -> GeoBoundsResult {
        GeoBoundsResult {
            bounds: GeoBox::from(from.find("bounds").expect("No 'bounds' field"))
        }
    }
}

#[derive(Debug)]
pub struct ScriptedMetricResult {
    pub value: JsonVal
}

impl<'a> From<&'a Json> for ScriptedMetricResult {
    fn from(from: &'a Json) -> ScriptedMetricResult {
        ScriptedMetricResult {
            value: JsonVal::from(from.find("value").expect("No 'value' field"))
        }
    }
}

// Buckets result

/// Macros for buckets to return a reference to the sub-aggregations
macro_rules! add_aggs_ref {
    () => {
        pub fn aggs_ref<'a>(&'a self) -> Option<&'a AggregationsResult> {
            self.aggs.as_ref()
        }
    }
}

/// Macro to extract sub-aggregations for a bucket aggregation
macro_rules! extract_aggs {
    ($f:ident, $a:ident) => {
        // match $a {
        //     &Some(ref agg) => {
        //         Some(object_to_result(agg, $f.as_object().expect("Not an object")))
        //     },
        //     &None          => None
        // }
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct GlobalResult {
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl GlobalResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> GlobalResult {
        GlobalResult {
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct FilterResult {
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl FilterResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> FilterResult {
        FilterResult {
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct FiltersBucketResult {
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl FiltersBucketResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> FiltersBucketResult {
        FiltersBucketResult {
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct FiltersResult {
    pub buckets: HashMap<String, FiltersBucketResult>
}

impl FiltersResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> FiltersResult {
        FiltersResult {
            buckets: from.find("buckets").expect("No buckets")
                .as_object().expect("Not an object")
                .into_iter().map(|(k, v)| {
                    (k.clone(), FiltersBucketResult::from(v, aggs))
                }).collect()
        }
    }
}

#[derive(Debug)]
pub struct MissingResult {
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl MissingResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> MissingResult {
        MissingResult {
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct NestedResult {
    pub aggs: Option<AggregationsResult>
}

impl NestedResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> NestedResult {
        NestedResult {
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct ReverseNestedResult {
    pub aggs: Option<AggregationsResult>
}

impl ReverseNestedResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> ReverseNestedResult {
        ReverseNestedResult {
            aggs: extract_aggs!(from, aggs)
        }
    }
}

#[derive(Debug)]
pub struct ChildrenResult {
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl ChildrenResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> ChildrenResult {
        ChildrenResult {
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct TermsBucketResult {
    pub key: JsonVal,
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl TermsBucketResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> TermsBucketResult {
        info!("Creating TermsBucketResult from: {:?} with {:?}", from, aggs);

        TermsBucketResult {
            key: JsonVal::from(from.find("key").expect("No 'key' value")),
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct TermsResult {
    pub doc_count_error_upper_bound: u64,
    pub sum_other_doc_count: u64,
    pub buckets: Vec<TermsBucketResult>
}

impl TermsResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> TermsResult {
        TermsResult {
            doc_count_error_upper_bound: get_json_u64!(from, "doc_count_error_upper_bound"),
            sum_other_doc_count: get_json_u64!(from, "sum_other_doc_count"),
            buckets: from.find("buckets").expect("No buckets")
                .as_array().expect("Not an array")
                .iter().map(|bucket| {
                    TermsBucketResult::from(bucket, aggs)
                }).collect()
        }
    }
}

// Range result objects

#[derive(Debug)]
pub struct RangeBucketResult {
    pub from:      Option<JsonVal>,
    pub to:        Option<JsonVal>,
    pub doc_count: u64,
    pub aggs:      Option<AggregationsResult>
}

impl RangeBucketResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> RangeBucketResult {
        RangeBucketResult {
            from:      from.find("from").and_then(|from| Some(from.into())),
            to:        from.find("to").and_then(|to| Some(to.into())),
            doc_count: get_json_u64!(from, "doc_count"),
            aggs:      extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct RangeResult {
    pub buckets: HashMap<String, RangeBucketResult>,
}

impl RangeResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> RangeResult {
        let bucket_obj = get_json_object!(from, "buckets");
        let mut buckets = HashMap::with_capacity(bucket_obj.len());

        for (k, v) in bucket_obj.into_iter() {
            buckets.insert(k.clone(), RangeBucketResult::from(v, aggs));
        }

        RangeResult {
            buckets: buckets
        }
    }
}

// Date range result objects

#[derive(Debug)]
pub struct DateRangeBucketResult {
    pub from:           Option<f64>,
    pub from_as_string: Option<String>,
    pub to:             Option<f64>,
    pub to_as_string:   Option<String>,
    pub doc_count:      u64,
    pub aggs:           Option<AggregationsResult>
}

impl DateRangeBucketResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> DateRangeBucketResult {
        DateRangeBucketResult {
            from:           optional_json_f64!(from, "from"),
            from_as_string: optional_json_string!(from, "from_as_string"),
            to:             optional_json_f64!(from, "to"),
            to_as_string:   optional_json_string!(from, "to_as_string"),
            doc_count:      get_json_u64!(from, "doc_count"),
            aggs:           extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct DateRangeResult {
    pub buckets: Vec<DateRangeBucketResult>
}

impl DateRangeResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> DateRangeResult {
        DateRangeResult {
            buckets: from.find("buckets").expect("No buckets")
                .as_array().expect("Not an array")
                .iter().map(|bucket| {
                    DateRangeBucketResult::from(bucket, aggs)
                }).collect()
        }
    }
}

/// Used for histogram results
#[derive(Debug)]
pub struct HistogramBucketResult {
    pub key: String,
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl HistogramBucketResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> HistogramBucketResult {
        HistogramBucketResult {
            key: get_json_string!(from, "key"),
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct HistogramResult {
    pub buckets: Vec<HistogramBucketResult>
}

impl HistogramResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> HistogramResult {
        HistogramResult {
            buckets: from.find("buckets").expect("No buckets")
                .as_array().expect("Not an array")
                .iter().map(|bucket| HistogramBucketResult::from(bucket, aggs))
                .collect()
        }
    }
}

// Date histogram results
#[derive(Debug)]
pub struct DateHistogramBucketResult {
    pub key_as_string: String,
    pub key: u64,
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl DateHistogramBucketResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> DateHistogramBucketResult {
        DateHistogramBucketResult {
            key_as_string: get_json_string!(from, "key_as_string"),
            key: get_json_u64!(from, "key"),
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct DateHistogramResult {
    pub buckets: Vec<DateHistogramBucketResult>
}

impl DateHistogramResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> DateHistogramResult {
        DateHistogramResult {
            buckets: from.find("buckets").expect("No buckets")
                .as_array().expect("Not an array")
                .iter().map(|bucket| DateHistogramBucketResult::from(bucket, aggs))
                .collect()
        }
    }
}

// GeoDistance results
#[derive(Debug)]
pub struct GeoDistanceBucketResult {
    pub key: String,
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl GeoDistanceBucketResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> GeoDistanceBucketResult {
        GeoDistanceBucketResult {
            key: get_json_string!(from, "key"),
            from: optional_json_f64!(from, "from"),
            to: optional_json_f64!(from, "to"),
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct GeoDistanceResult {
    pub buckets: Vec<GeoDistanceBucketResult>
}

impl GeoDistanceResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> GeoDistanceResult {
        GeoDistanceResult {
            buckets: from.find("buckets").expect("No buckets")
                .as_array().expect("Not an array")
                .iter().map(|bucket| GeoDistanceBucketResult::from(bucket, aggs))
                .collect()
        }
    }
}

#[derive(Debug)]
pub struct GeoHashBucketResult {
    pub key:       String,
    pub doc_count: u64,
    pub aggs:      Option<AggregationsResult>
}

impl GeoHashBucketResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> GeoHashBucketResult {
        GeoHashBucketResult {
            key: get_json_string!(from, "key"),
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: extract_aggs!(from, aggs)
        }
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct GeoHashResult {
    pub buckets: Vec<GeoHashBucketResult>
}

impl GeoHashResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> GeoHashResult {
        GeoHashResult {
            buckets: from.find("buckets").expect("No buckets")
                .as_array().expect("Not an array")
                .iter().map(|bucket| GeoHashBucketResult::from(bucket, aggs))
                .collect()
        }
    }
}

/// The result of one specific aggregation
///
/// The data returned varies depending on aggregation type
#[derive(Debug, Deserialize)]
pub enum AggregationResult {
    // TODO - disabled during refactoring
    // // Metrics
    // Min(MinResult),
    // Max(MaxResult),
    // Sum(SumResult),
    // Avg(AvgResult),
    // Stats(StatsResult),
    // ExtendedStats(ExtendedStatsResult),
    // ValueCount(ValueCountResult),
    // Percentiles(PercentilesResult),
    // PercentileRanks(PercentileRanksResult),
    // Cardinality(CardinalityResult),
    // GeoBounds(GeoBoundsResult),
    // ScriptedMetric(ScriptedMetricResult),

    // // Buckets
    // Global(GlobalResult),
    // Filter(FilterResult),
    // Filters(FiltersResult),
    // Missing(MissingResult),
    // Nested(NestedResult),
    // ReverseNested(ReverseNestedResult),
    // Children(ChildrenResult),
    // Terms(TermsResult),
    // Range(RangeResult),
    // DateRange(DateRangeResult),
    // Histogram(HistogramResult),
    // DateHistogram(DateHistogramResult),
    // GeoDistance(GeoDistanceResult),
    // GeoHash(GeoHashResult)
}

/// Macro to implement the various as... functions that return the details of an
/// aggregation for that particular type
macro_rules! agg_as {
    ($n:ident,$t:ident,$rt:ty) => {
        pub fn $n<'a>(&'a self) -> Result<&'a $rt, EsError> {
            // TODO - re-enable
            // match self {
            //     &AggregationResult::$t(ref res) => Ok(res),
            //     _                               => {
            //         Err(EsError::EsError(format!("Wrong type: {:?}", self)))
            //     }
            // }
            unimplemented!()
        }
    }
}

impl AggregationResult {
    // Metrics
    agg_as!(as_min, Min, MinResult);
    agg_as!(as_max, Max, MaxResult);
    agg_as!(as_sum, Sum, SumResult);
    agg_as!(as_avg, Avg, AvgResult);
    agg_as!(as_stats, Stats, StatsResult);
    agg_as!(as_extended_stats, ExtendedStats, ExtendedStatsResult);
    agg_as!(as_value_count, ValueCount, ValueCountResult);
    agg_as!(as_percentiles, Percentiles, PercentilesResult);
    agg_as!(as_percentile_ranks, PercentileRanks, PercentileRanksResult);
    agg_as!(as_cardinality, Cardinality, CardinalityResult);
    agg_as!(as_geo_bounds, GeoBounds, GeoBoundsResult);
    agg_as!(as_scripted_metric, ScriptedMetric, ScriptedMetricResult);

    // buckets
    agg_as!(as_global, Global, GlobalResult);
    agg_as!(as_filter, Filter, FilterResult);
    agg_as!(as_filters, Filters, FiltersResult);
    agg_as!(as_missing, Missing, MissingResult);
    agg_as!(as_nested, Nested, NestedResult);
    agg_as!(as_reverse_nested, ReverseNested, ReverseNestedResult);
    agg_as!(as_children, Children, ChildrenResult);
    agg_as!(as_terms, Terms, TermsResult);
    agg_as!(as_range, Range, RangeResult);
    agg_as!(as_date_range, DateRange, DateRangeResult);
    agg_as!(as_histogram, Histogram, HistogramResult);
    agg_as!(as_date_histogram, DateHistogram, DateHistogramResult);
    agg_as!(as_geo_distance, GeoDistance, GeoDistanceResult);
    agg_as!(as_geo_hash, GeoHash, GeoHashResult);
}

#[derive(Debug)]
pub struct AggregationsResult(HashMap<String, AggregationResult>);

/// Loads a Json object of aggregation results into an `AggregationsResult`.
fn object_to_result(aggs: &Aggregations,
                    object: &BTreeMap<String, Value>) -> AggregationsResult {
    // let mut ar_map = HashMap::new();

    // for (key, val) in aggs.0.iter() {
    //     let owned_key = (*key).to_owned();
    //     let json = object.get(&owned_key).expect(&format!("No key: {}", &owned_key));
    //     ar_map.insert(owned_key, match val {
    //         &Aggregation::Metrics(ref ma) => {
    //             match ma {
    //                 &MetricsAggregation::Min(_) => {
    //                     AggregationResult::Min(MinResult::from(json))
    //                 },
    //                 &MetricsAggregation::Max(_) => {
    //                     AggregationResult::Max(MaxResult::from(json))
    //                 },
    //                 &MetricsAggregation::Sum(_) => {
    //                     AggregationResult::Sum(SumResult::from(json))
    //                 },
    //                 &MetricsAggregation::Avg(_) => {
    //                     AggregationResult::Avg(AvgResult::from(json))
    //                 },
    //                 &MetricsAggregation::Stats(_) => {
    //                     AggregationResult::Stats(StatsResult::from(json))
    //                 },
    //                 &MetricsAggregation::ExtendedStats(_) => {
    //                     AggregationResult::ExtendedStats(ExtendedStatsResult::from(json))
    //                 },
    //                 &MetricsAggregation::ValueCount(_) => {
    //                     AggregationResult::ValueCount(ValueCountResult::from(json))
    //                 }
    //                 &MetricsAggregation::Percentiles(_) => {
    //                     AggregationResult::Percentiles(PercentilesResult::from(json))
    //                 },
    //                 &MetricsAggregation::PercentileRanks(_) => {
    //                     AggregationResult::PercentileRanks(PercentileRanksResult::from(json))
    //                 },
    //                 &MetricsAggregation::Cardinality(_) => {
    //                     AggregationResult::Cardinality(CardinalityResult::from(json))
    //                 },
    //                 &MetricsAggregation::GeoBounds(_) => {
    //                     AggregationResult::GeoBounds(GeoBoundsResult::from(json))
    //                 },
    //                 &MetricsAggregation::ScriptedMetric(_) => {
    //                     AggregationResult::ScriptedMetric(ScriptedMetricResult::from(json))
    //                 }
    //             }
    //         },
    //         &Aggregation::Bucket(ref ba, ref aggs) => {
    //             match ba {
    //                 &BucketAggregation::Global(_) => {
    //                     AggregationResult::Global(GlobalResult::from(json, aggs))
    //                 },
    //                 &BucketAggregation::Filter(_) => {
    //                     AggregationResult::Filter(FilterResult::from(json, aggs))
    //                 },
    //                 &BucketAggregation::Filters(_) => {
    //                     AggregationResult::Filters(FiltersResult::from(json, aggs))
    //                 },
    //                 &BucketAggregation::Missing(_) => {
    //                     AggregationResult::Missing(MissingResult::from(json, aggs))
    //                 },
    //                 &BucketAggregation::Nested(_) => {
    //                     AggregationResult::Nested(NestedResult::from(json, aggs))
    //                 },
    //                 &BucketAggregation::ReverseNested(_) => {
    //                     AggregationResult::ReverseNested(ReverseNestedResult::from(json,
    //                                                                                aggs))
    //                 },
    //                 &BucketAggregation::Children(_) => {
    //                     AggregationResult::Children(ChildrenResult::from(json, aggs))
    //                 },
    //                 &BucketAggregation::Terms(_) => {
    //                     AggregationResult::Terms(TermsResult::from(json, aggs))
    //                 },
    //                 &BucketAggregation::Range(_) => {
    //                     AggregationResult::Range(RangeResult::from(json, aggs))
    //                 },
    //                 &BucketAggregation::DateRange(_) => {
    //                     AggregationResult::DateRange(DateRangeResult::from(json, aggs))
    //                 },
    //                 &BucketAggregation::Histogram(_) => {
    //                     AggregationResult::Histogram(HistogramResult::from(json, aggs))
    //                 },
    //                 &BucketAggregation::DateHistogram(_) => {
    //                     AggregationResult::DateHistogram(DateHistogramResult::from(json,
    //                                                                                aggs))
    //                 },
    //                 &BucketAggregation::GeoDistance(_) => {
    //                     AggregationResult::GeoDistance(GeoDistanceResult::from(json,
    //                                                                            aggs))
    //                 },
    //                 &BucketAggregation::GeoHash(_) => {
    //                     AggregationResult::GeoHash(GeoHashResult::from(json, aggs))
    //                 }
    //             }
    //         }
    //     });
    // }

    // info!("Processed aggs - From: {:?}. To: {:?}", object, ar_map);

    // AggregationsResult(ar_map)
    unimplemented!()
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
        let object = match json.as_object() {
            Some(o) => o,
            None    => return Err(EsError::EsError("Aggregations is not an object".to_owned()))
        };
        Ok(object_to_result(aggs, object))
    }
}
