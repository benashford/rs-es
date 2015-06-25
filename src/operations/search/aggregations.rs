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

#[derive(Debug)]
pub enum Scripts<'a> {
    Inline(&'a str, Option<&'a str>),
    Id(&'a str)
}

/// Script attributes for various attributes
#[derive(Debug)]
pub struct Script<'a> {
    script: Scripts<'a>,
    params: Option<Json>
}

impl<'a> Script<'a> {
    pub fn id(script_id: &'a str) -> Script<'a> {
        Script {
            script: Scripts::Id(script_id),
            params: None
        }
    }

    pub fn script(script: &'a str) -> Script<'a> {
        Script {
            script: Scripts::Inline(script, None),
            params: None
        }
    }

    pub fn script_and_field(script: &'a str, field: &'a str) -> Script<'a> {
        Script {
            script: Scripts::Inline(script, Some(field)),
            params: None
        }
    }

    fn add_to_object(&self, obj: &mut BTreeMap<String, Json>) {
        match self.script {
            Scripts::Inline(script, field) => {
                obj.insert("script".to_owned(), script.to_json());
                match field {
                    Some(f) => {
                        obj.insert("field".to_owned(), f.to_json());
                    },
                    None    => ()
                }
            },
            Scripts::Id(script_id) => {
                obj.insert("script_id".to_owned(), script_id.to_json());
            }
        };
        match self.params {
            Some(ref json) => {
                obj.insert("params".to_owned(), json.clone());
            },
            None           => ()
        };
    }

    pub fn with_params(mut self, params: Json) -> Self {
        self.params = Some(params);
        self
    }
}

macro_rules! metrics_agg {
    ($b:ident) => {
        impl<'a> From<$b<'a>> for Aggregation<'a> {
            fn from(from: $b<'a>) -> Aggregation<'a> {
                Aggregation::Metrics(MetricsAggregation::$b(from))
            }
        }
    }
}

/// A common pattern is for an aggregation to accept a field or a script
#[derive(Debug)]
pub enum FieldOrScript<'a> {
    Field(&'a str),
    Script(Script<'a>)
}

impl<'a> FieldOrScript<'a> {
    fn add_to_object(&self, obj: &mut BTreeMap<String, Json>) {
        match self {
            &FieldOrScript::Field(field) => {
                obj.insert("field".to_owned(), field.to_json());
            },
            &FieldOrScript::Script(ref script) => {
                script.add_to_object(obj);
            }
        }
    }
}

impl<'a> From<&'a str> for FieldOrScript<'a> {
    fn from(from: &'a str) -> FieldOrScript<'a> {
        FieldOrScript::Field(from)
    }
}

impl<'a> From<Script<'a>> for FieldOrScript<'a> {
    fn from(from: Script<'a>) -> FieldOrScript<'a> {
        FieldOrScript::Script(from)
    }
}

/// Macros to build simple `FieldOrScript` based aggregations
macro_rules! field_or_script_new {
    ($t:ident) => {
        impl<'a> $t<'a> {
            pub fn new<FOS: Into<FieldOrScript<'a>>>(fos: FOS) -> $t<'a> {
                $t(fos.into())
            }
        }
    }
}

macro_rules! field_or_script_to_json {
    ($t:ident) => {
        impl<'a> ToJson for $t<'a> {
            fn to_json(&self) -> Json {
                let mut d = BTreeMap::new();
                self.0.add_to_object(&mut d);
                Json::Object(d)
            }
        }
    }
}

/// Min aggregation
#[derive(Debug)]
pub struct Min<'a>(FieldOrScript<'a>);

field_or_script_new!(Min);
field_or_script_to_json!(Min);
metrics_agg!(Min);

/// Max aggregation
#[derive(Debug)]
pub struct Max<'a>(FieldOrScript<'a>);

field_or_script_new!(Max);
field_or_script_to_json!(Max);
metrics_agg!(Max);

/// Sum aggregation
#[derive(Debug)]
pub struct Sum<'a>(FieldOrScript<'a>);

field_or_script_new!(Sum);
field_or_script_to_json!(Sum);
metrics_agg!(Sum);

/// Avg aggregation
#[derive(Debug)]
pub struct Avg<'a>(FieldOrScript<'a>);

field_or_script_new!(Avg);
field_or_script_to_json!(Avg);
metrics_agg!(Avg);

/// Individual aggregations and their options
#[derive(Debug)]
pub enum MetricsAggregation<'a> {
    Min(Min<'a>),
    Max(Max<'a>),
    Sum(Sum<'a>),
    Avg(Avg<'a>)
}

impl<'a> ToJson for MetricsAggregation<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        match self {
            &MetricsAggregation::Min(ref min_agg) => {
                d.insert("min".to_owned(), min_agg.to_json());
            },
            &MetricsAggregation::Max(ref max_agg) => {
                d.insert("max".to_owned(), max_agg.to_json());
            },
            &MetricsAggregation::Sum(ref sum_agg) => {
                d.insert("sum".to_owned(), sum_agg.to_json());
            },
            &MetricsAggregation::Avg(ref avg_agg) => {
                d.insert("avg".to_owned(), avg_agg.to_json());
            }
        }
        Json::Object(d)
    }
}

/// Order - used for some bucketing aggregations to determine the order of
/// buckets
#[derive(Debug)]
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

/// Used to define the ordering of buckets in a some bucketted aggregations
///
/// # Examples
///
/// ```
/// use rs_es::operations::search::aggregations::{Order, OrderKey};
///
/// let order1 = Order::asc(OrderKey::Count);
/// let order2 = Order::desc("field_name");
/// ```
///
/// The first will produce a JSON fragment: `{"_count": "asc"}`; the second will
/// produce a JSON fragment: `{"field_name", "desc"}`
#[derive(Debug)]
pub struct Order<'a>(OrderKey<'a>, super::Order);

impl<'a> Order<'a> {
    /// Create an `Order` ascending
    pub fn asc<O: Into<OrderKey<'a>>>(key: O) -> Order<'a> {
        Order(key.into(), super::Order::Asc)
    }

    /// Create an `Order` descending
    pub fn desc<O: Into<OrderKey<'a>>>(key: O) -> Order<'a> {
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

/// Terms aggregation
#[derive(Debug)]
pub struct Terms<'a> {
    field:      FieldOrScript<'a>,
    size:       Option<u64>,
    shard_size: Option<u64>,
    order:      Option<Order<'a>>
}

impl<'a> Terms<'a> {
    pub fn new<FOS: Into<FieldOrScript<'a>>>(field: FOS) -> Terms<'a> {
        Terms {
            field:      field.into(),
            size:       None,
            shard_size: None,
            order:      None
        }
    }

    add_field!(with_size, size, u64);
    add_field!(with_shard_size, shard_size, u64);
    add_field!(with_order, order, Order<'a>);
}

macro_rules! bucket_agg {
    ($b:ident) => {
        impl<'a> From<($b<'a>, Aggregations<'a>)> for Aggregation<'a> {
            fn from(from: ($b<'a>, Aggregations<'a>)) -> Aggregation<'a> {
                Aggregation::Bucket(BucketAggregation::$b(from.0), Some(from.1))
            }
        }

        impl<'a> From<$b<'a>> for Aggregation<'a> {
            fn from(from: $b<'a>) -> Aggregation<'a> {
                Aggregation::Bucket(BucketAggregation::$b(from), None)
            }
        }
    }
}

bucket_agg!(Terms);

impl<'a> ToJson for Terms<'a> {
    fn to_json(&self) -> Json {
        let mut json = BTreeMap::new();
        self.field.add_to_object(&mut json);

        optional_add!(json, self.size, "size");
        optional_add!(json, self.shard_size, "shard_size");
        optional_add!(json, self.order, "order");

        Json::Object(json)
    }
}

/// The set of bucket aggregations
#[derive(Debug)]
pub enum BucketAggregation<'a> {
    Terms(Terms<'a>)
}

impl<'a> BucketAggregation<'a> {
    fn add_to_object(&self, json: &mut BTreeMap<String, Json>) {
        match self {
            &BucketAggregation::Terms(ref terms) => {
                json.insert("terms".to_owned(), terms.to_json());
            }
        }
    }
}

/// Aggregations are either metrics or bucket-based aggregations
#[derive(Debug)]
pub enum Aggregation<'a> {
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
                ba.add_to_object(&mut d);
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
///
/// There are many ways of creating aggregations, either standalone or via a
/// conversion trait
#[derive(Debug)]
pub struct Aggregations<'a>(HashMap<&'a str, Aggregation<'a>>);

impl<'a> Aggregations<'a> {
    /// Create an empty-set of aggregations, individual aggregations should be
    /// added via the `add` method
    ///
    /// # Examples
    ///
    /// ```
    /// use rs_es::operations::search::aggregations::{Aggregations, Min};
    ///
    /// let mut aggs = Aggregations::new();
    /// aggs.add("agg_name", Min::new("field_name"));
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

// Buckets result

/// Macros for buckets to return a reference to the sub-aggregations
macro_rules! add_aggs_ref {
    () => {
        pub fn aggs_ref<'a>(&'a self) -> Option<&'a AggregationsResult> {
            self.aggs.as_ref()
        }
    }
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
            aggs: match aggs {
                &Some(ref agg) => {
                    Some(object_to_result(agg, from.as_object().expect("Not an object")))
                },
                &None          => None
            }
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

/// The result of one specific aggregation
///
/// The data returned varies depending on aggregation type
#[derive(Debug)]
pub enum AggregationResult {
    // Metrics
    Min(MinResult),
    Max(MaxResult),
    Sum(SumResult),
    Avg(AvgResult),

    // Buckets
    Terms(TermsResult)
}

/// Macro to implement the various as... functions that return the details of an
/// aggregation for that particular type
macro_rules! agg_as {
    ($n:ident,$t:ident,$rt:ty) => {
        pub fn $n<'a>(&'a self) -> Result<&'a $rt, EsError> {
            match self {
                &AggregationResult::$t(ref res) => Ok(res),
                _                               => {
                    Err(EsError::EsError(format!("Wrong type: {:?}", self)))
                }
            }
        }
    }
}

impl AggregationResult {
    // Metrics
    agg_as!(as_min, Min, MinResult);
    agg_as!(as_max, Max, MaxResult);
    agg_as!(as_sum, Sum, SumResult);

    // buckets
    agg_as!(as_terms, Terms, TermsResult);
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
                    },
                    &MetricsAggregation::Max(_) => {
                        AggregationResult::Max(MaxResult::from(json))
                    },
                    &MetricsAggregation::Sum(_) => {
                        AggregationResult::Sum(SumResult::from(json))
                    },
                    &MetricsAggregation::Avg(_) => {
                        AggregationResult::Avg(AvgResult::from(json))
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
