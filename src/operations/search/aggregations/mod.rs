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

use std::collections::HashMap;

use serde::ser::{Serialize, Serializer, SerializeMap};
use serde_json::{Value, Map};

use ::error::EsError;

use self::bucket::BucketAggregationResult;
use self::metrics::MetricsAggregationResult;

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
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        use self::Aggregation::*;
        let mut map = try!(serializer.serialize_map(Some(match self {
            &Metrics(_)              => 1,
            &Bucket(_, ref opt_aggs) => match opt_aggs {
                &Some(_) => 2,
                &None    => 1
            }
        })));
        match self {
            &Metrics(ref metric_agg) => {
                let agg_name = metric_agg.details();
                try!(map.serialize_entry(agg_name, metric_agg));
            },
            &Bucket(ref bucket_agg, ref opt_aggs) => {
                let agg_name = bucket_agg.details();
                try!(map.serialize_entry(agg_name, bucket_agg));
                match opt_aggs {
                    &Some(ref other_aggs) => {
                        try!(map.serialize_entry("aggregations", other_aggs));
                    }
                    &None => ()
                }
            }
        }
        map.end()
    }
}

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
                    object: &Map<String, Value>) -> Result<AggregationsResult, EsError> {
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
        let object = match json.as_object() {
            Some(o) => o,
            None    => return Err(EsError::EsError("Aggregations is not an object".to_owned()))
        };
        object_to_result(aggs, object)
    }
}
