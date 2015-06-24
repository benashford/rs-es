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

//! Implementation of ElasticSearch [aggregations](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations.html)

use std::collections::{BTreeMap, HashMap};

use rustc_serialize::json::{Json, ToJson};

use error::EsError;
use units::JsonVal;

/// Script attributes for various attributes
pub struct Script<'a> {
    field:  &'a str,
    script: &'a str,
    params: Option<Json>
}

impl<'a> Script<'a> {
    fn add_to_object(&self, obj: &mut BTreeMap<String, Json>) {
        obj.insert("field".to_owned(), self.field.to_json());
        obj.insert("script".to_owned(), self.script.to_json());
        match self.params {
            Some(ref json) => {
                obj.insert("params".to_owned(), json.clone());
            },
            None           => ()
        }
    }

    pub fn with_params(mut self, params: Json) -> Self {
        self.params = Some(params);
        self
    }
}

/// Min aggregation
pub enum Min<'a> {
    /// Field
    Field(&'a str),

    /// By Script
    Script(Script<'a>)
}

impl<'a> From<Min<'a>> for Aggregation<'a> {
    fn from(from: Min<'a>) -> Aggregation<'a> {
        Aggregation::Metrics(MetricsAggregation::Min(from))
    }
}

impl<'a> ToJson for Min<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        match self {
            &Min::Field(field) => {
                d.insert("field".to_string(), field.to_json());
            },
            &Min::Script(ref script) => {
                script.add_to_object(&mut d);
            }
        }
        Json::Object(d)
    }
}

/// Individual aggregations and their options
enum MetricsAggregation<'a> {
    Min(Min<'a>)
}

impl<'a> ToJson for MetricsAggregation<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        match self {
            &MetricsAggregation::Min(ref min_agg) => {
                d.insert("min".to_owned(), min_agg.to_json());
            }
        }
        Json::Object(d)
    }
}

/// Order - used for some bucketing aggregations to determine the order of
/// buckets
pub enum OrderKey<'a> {
    Count,
    Term,
    Expr(&'a str)
}

impl<'a> From<&'a str> for OrderKey<'a> {
    fn from(from: &'a str) -> OrderKey<'a> {
        OrderKey::Expr(from)
    }
}

impl<'a> ToString for OrderKey<'a> {
    fn to_string(&self) -> String {
        match *self {
            OrderKey::Count   => "_count".to_owned(),
            OrderKey::Term    => "_term".to_owned(),
            OrderKey::Expr(e) => e.to_owned()
        }
    }
}

pub struct Order<'a>(OrderKey<'a>, super::Order);

impl<'a> Order<'a> {
    fn asc<O: Into<OrderKey<'a>>>(key: O) -> Order<'a> {
        Order(key.into(), super::Order::Asc)
    }

    fn desc<O: Into<OrderKey<'a>>>(key: O) -> Order<'a> {
        Order(key.into(), super::Order::Desc)
    }
}

impl<'a> ToJson for Order<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert(self.0.to_string(), self.1.to_json());
        Json::Object(d)
    }
}

/// Add to JSON trait
trait AddToJson {
    fn add_to_json(&self, &mut BTreeMap<String, Json>);
}

/// Terms aggregation
pub struct Terms<'a> {
    field:      &'a str,
    size:       Option<u64>,
    shard_size: Option<u64>,
    order:      Option<Order<'a>>
}

impl<'a> Terms<'a> {
    pub fn new(field: &'a str) -> Terms<'a> {
        Terms {
            field:      field,
            size:       None,
            shard_size: None,
            order:      None
        }
    }

    add_field!(with_size, size, u64);
    add_field!(with_shard_size, shard_size, u64);
    add_field!(with_order, order, Order<'a>);
}

impl<'a> From<(Terms<'a>, Aggregations<'a>)> for Aggregation<'a> {
    fn from(from: (Terms<'a>, Aggregations<'a>)) -> Aggregation<'a> {
        Aggregation::Bucket(BucketAggregation::Terms(from.0), Some(from.1))
    }
}

impl<'a> From<Terms<'a>> for Aggregation<'a> {
    fn from(from: Terms<'a>) -> Aggregation<'a> {
        Aggregation::Bucket(BucketAggregation::Terms(from), None)
    }
}

impl<'a> ToJson for Terms<'a> {
    fn to_json(&self) -> Json {
        let mut json = BTreeMap::new();
        json.insert("field".to_owned(), Json::String(self.field.to_owned()));
        optional_add!(json, self.size, "size");
        optional_add!(json, self.shard_size, "shard_size");
        optional_add!(json, self.order, "order");
        Json::Object(json)
    }
}

/// The set of bucket aggregations
enum BucketAggregation<'a> {
    Terms(Terms<'a>)
}

impl<'a> AddToJson for BucketAggregation<'a> {
    fn add_to_json(&self, json: &mut BTreeMap<String, Json>) {
        match self {
            &BucketAggregation::Terms(ref terms) => {
                json.insert("terms".to_owned(), terms.to_json());
            }
        }
    }
}

/// Aggregations are either metrics or bucket-based aggregations
enum Aggregation<'a> {
    /// A metric aggregation (e.g. min)
    Metrics(MetricsAggregation<'a>),

    /// A bucket aggregation, groups data into buckets and optionally applies
    /// sub-aggregations
    Bucket(BucketAggregation<'a>, Option<Aggregations<'a>>)
}

impl<'a> ToJson for Aggregation<'a> {
    fn to_json(&self) -> Json {
        match self {
            &Aggregation::Metrics(ref ma)          => {
                ma.to_json()
            },
            &Aggregation::Bucket(ref ba, ref aggs) => {
                let mut d = BTreeMap::new();
                ba.add_to_json(&mut d);
                match aggs {
                    &Some(ref a) => {
                        d.insert("aggs".to_owned(), a.to_json());
                    },
                    &None        => ()
                }
                Json::Object(d)
            }
        }
    }
}

/// The set of aggregations
pub struct Aggregations<'a>(HashMap<&'a str, Aggregation<'a>>);

impl<'a> Aggregations<'a> {
    pub fn new() -> Aggregations<'a> {
        Aggregations(HashMap::new())
    }

    pub fn insert<A: Into<Aggregation<'a>>>(&mut self, key: &'a str, val: A) {
        self.0.insert(key, val.into());
    }
}

impl<'a> ToJson for Aggregations<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        for (k, ref v) in self.0.iter() {
            d.insert((*k).to_owned(), v.to_json());
        }
        Json::Object(d)
    }
}

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

// Buckets result

#[derive(Debug)]
pub struct TermsBucketResult {
    pub key: JsonVal,
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl TermsBucketResult {
    fn from(from: &Json, aggs: &Option<Aggregations>) -> TermsBucketResult {
        TermsBucketResult {
            key: JsonVal::from(from.find("key").expect("No 'key' value")),
            doc_count: get_json_u64!(from, "doc_count"),
            aggs: match aggs {
                &Some(ref agg) => {
                    Some(object_to_result(agg,
                                          get_json_object!(from, "aggregations")))
                },
                &None          => None
            }
        }
    }
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

/// The result of one specific aggregation
///
/// The data returned varies depending on aggregation type
#[derive(Debug)]
pub enum AggregationResult {
    // Metrics
    Min(MinResult),

    // Buckets
    Terms(TermsResult)
}

impl AggregationResult {
    pub fn as_min<'a>(&'a self) -> Result<&'a MinResult, EsError> {
        use self::AggregationResult::*;
        match self {
            &Min(ref res) => Ok(res),
            _             => Err(EsError::EsError(format!("Wrong type: {:?}", self)))
        }
    }
}

#[derive(Debug)]
pub struct AggregationsResult(HashMap<String, AggregationResult>);

/// Loads a Json object of aggregation results into an `AggregationsResult`.
fn object_to_result(aggs: &Aggregations, object: &BTreeMap<String, Json>) -> AggregationsResult {
    let mut ar_map = HashMap::new();

    for (key, val) in aggs.0.iter() {
        let owned_key = (*key).to_owned();
        let json = object.get(&owned_key).expect(&format!("No key: {}", &owned_key));
        ar_map.insert(owned_key, match val {
            &Aggregation::Metrics(ref ma) => {
                match ma {
                    &MetricsAggregation::Min(_) => {
                        AggregationResult::Min(MinResult::from(json))
                    }
                }
            },
            &Aggregation::Bucket(ref ba, ref aggs) => {
                match ba {
                    &BucketAggregation::Terms(_) => {
                        AggregationResult::Terms(TermsResult::from(json, aggs))
                    }
                }
            }
        });
    }

    info!("Processed aggs - From: {:?}. To: {:?}", object, ar_map);

    AggregationsResult(ar_map)
}

impl AggregationsResult {
    pub fn get<'a>(&'a self, key: &str) -> Result<&'a AggregationResult, EsError> {
        match self.0.get(key) {
            Some(ref agg_res) => Ok(agg_res),
            None              => Err(EsError::EsError(format!("No agg for key: {}",
                                                              key)))
        }
    }

    pub fn from(aggs: &Aggregations, json: &Json) -> AggregationsResult {
        let object = json.find("aggregations")
            .expect("No aggregations")
            .as_object()
            .expect("No aggregations");

        object_to_result(aggs, object)
    }
}
