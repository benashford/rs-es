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
        Aggregation::Min(from)
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
enum Aggregation<'a> {
    Min(Min<'a>)
}

impl<'a> ToJson for Aggregation<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        match self {
            &Aggregation::Min(ref min_agg) => {
                d.insert("min".to_owned(), min_agg.to_json());
            }
        }
        Json::Object(d)
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

/// The result of one specific aggregation
///
/// The data returned varies depending on aggregation type
#[derive(Debug)]
pub enum AggregationResult {
    Min(MinResult)
}

impl AggregationResult {
    pub fn as_min<'a>(&'a self) -> Result<&'a MinResult, EsError> {
        use self::AggregationResult::*;
        match self {
            &Min(ref res) => Ok(res)//,
            //_          => Err(EsError::EsError(format!("Wrong type: {:?}", self)))
        }
    }
}

pub struct AggregationsResult(HashMap<String, AggregationResult>);

/// Loads a Json object of aggregation results into an `AggregationsResult`.
fn object_to_result(aggs: &Aggregations, object: &BTreeMap<String, Json>) -> AggregationsResult {
    let mut ar_map = HashMap::new();

    for (key, val) in aggs.0.iter() {
        let owned_key = (*key).to_owned();
        let json = object.get(&owned_key).expect(&format!("No key: {}", &owned_key));
        ar_map.insert(owned_key, match val {
            &Aggregation::Min(_) => AggregationResult::Min(MinResult::from(json))
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
