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

//! Compound queries

use std::collections::BTreeMap;

use rustc_serialize::json::{Json, ToJson};

use ::units::{JsonVal, OneOrMany};

use super::{MinimumShouldMatch, ScoreMode, Query};
use super::functions::Function;

/// BoostMode
#[derive(Debug)]
pub enum BoostMode {
    Multiply,
    Replace,
    Sum,
    Avg,
    Max,
    Min
}

impl ToJson for BoostMode {
    fn to_json(&self) -> Json {
        match self {
            &BoostMode::Multiply => "multiply",
            &BoostMode::Replace => "replace",
            &BoostMode::Sum => "sum",
            &BoostMode::Avg => "avg",
            &BoostMode::Max => "max",
            &BoostMode::Min => "min"
        }.to_json()
    }
}

/// Constant score query
#[derive(Debug, Default)]
pub struct ConstantScoreQuery {
    query: Query,
    boost: Option<f64>
}

impl Query {
    pub fn build_constant_score<A>(query: A) -> ConstantScoreQuery
        where A: Into<Query> {

        ConstantScoreQuery {
            query: query.into(),
            ..Default::default()
        }
    }
}

impl ConstantScoreQuery {
    add_option!(with_boost, boost, f64);

    build!(ConstantScore);
}

impl ToJson for ConstantScoreQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("query".to_owned(), self.query.to_json());
        optional_add!(self, d, boost);
        Json::Object(d)
    }
}

/// Bool query
#[derive(Debug, Default)]
pub struct BoolQuery {
    must: Option<OneOrMany<Query>>,
    filter: Option<Query>,
    should: Option<OneOrMany<Query>>,
    must_not: Option<OneOrMany<Query>>,
    minimum_should_match: Option<MinimumShouldMatch>,
    boost: Option<f64>,
    disable_coord: Option<bool>
}

impl Query {
    pub fn build_bool() -> BoolQuery {
        Default::default()
    }
}

impl BoolQuery {
    add_option!(with_must, must, OneOrMany<Query>);
    add_option!(with_filter, filter, Query);
    add_option!(with_should, should, OneOrMany<Query>);
    add_option!(with_must_not, must_not, OneOrMany<Query>);
    add_option!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_option!(with_boost, boost, f64);
    add_option!(with_disable_coord, disable_coord, bool);

    build!(Bool);
}

impl ToJson for BoolQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        optional_add!(self, d, must);
        optional_add!(self, d, filter);
        optional_add!(self, d, should);
        optional_add!(self, d, must_not);
        optional_add!(self, d, minimum_should_match);
        optional_add!(self, d, boost);
        optional_add!(self, d, disable_coord);
        Json::Object(d)
    }
}

/// DisMax query
#[derive(Debug, Default)]
pub struct DisMaxQuery {
    tie_breaker: Option<f64>,
    boost: Option<f64>,
    queries: Vec<Query>
}

impl Query {
    pub fn build_dis_max<A>(queries: A) -> DisMaxQuery
        where A: Into<Vec<Query>> {

        DisMaxQuery {
            queries: queries.into(),
            ..Default::default()
        }
    }
}

impl DisMaxQuery {
    add_option!(with_tie_breaker, tie_breaker, f64);
    add_option!(with_boost, boost, f64);

    build!(DisMax);
}

impl ToJson for DisMaxQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("queries".to_owned(), self.queries.to_json());
        optional_add!(self, d, tie_breaker);
        optional_add!(self, d, boost);
        Json::Object(d)
    }
}

/// Function Score query
#[derive(Debug, Default)]
pub struct FunctionScoreQuery {
    query: Option<Query>,
    boost: Option<f64>,
    functions: Vec<Function>,
    max_boost: Option<f64>,
    score_mode: Option<ScoreMode>,
    boost_mode: Option<BoostMode>,
    min_score: Option<f64>
}

impl Query {
    pub fn build_function_score() -> FunctionScoreQuery {
        Default::default()
    }
}

impl FunctionScoreQuery {
    add_option!(with_query, query, Query);
    add_option!(with_boost, boost, f64);
    add_option!(with_max_boost, max_boost, f64);
    add_option!(with_score_mode, score_mode, ScoreMode);
    add_option!(with_boost_mode, boost_mode, BoostMode);
    add_option!(with_min_score, min_score, f64);

    pub fn with_functions<A: Into<Vec<Function>>>(mut self, functions: A) -> Self {
        self.functions = functions.into();
        self
    }

    pub fn with_function<A: Into<Function>>(mut self, function: A) -> Self {
        self.functions = vec![function.into()];
        self
    }

    build!(FunctionScore);
}

impl ToJson for FunctionScoreQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("functions".to_owned(), self.functions.to_json());
        optional_add!(self, d, query);
        optional_add!(self, d, boost);
        optional_add!(self, d, max_boost);
        optional_add!(self, d, score_mode);
        optional_add!(self, d, boost_mode);
        optional_add!(self, d, min_score);
        Json::Object(d)
    }
}

/// Boosting query
#[derive(Debug, Default)]
pub struct BoostingQuery {
    positive: Option<Query>,
    negative: Option<Query>,
    negative_boost: Option<f64>
}

impl Query {
    pub fn build_boosting() -> BoostingQuery {
        Default::default()
    }
}

impl BoostingQuery {
    add_option!(with_positive, positive, Query);
    add_option!(with_negative, negative, Query);
    add_option!(with_negative_boost, negative_boost, f64);

    build!(Boosting);
}

impl ToJson for BoostingQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        optional_add!(self, d, positive);
        optional_add!(self, d, negative);
        optional_add!(self, d, negative_boost);
        Json::Object(d)
    }
}

/// Indices query
#[derive(Debug, Default)]
pub struct IndicesQuery {
    indices: OneOrMany<String>,
    query: Query,
    no_match_query: Option<NoMatchQuery>
}

impl Query {
    pub fn build_indices<A, B>(indices: A, query: B) -> IndicesQuery
        where A: Into<OneOrMany<String>>,
              B: Into<Query> {
        IndicesQuery {
            indices: indices.into(),
            query: query.into(),
            ..Default::default()
        }
    }
}

impl IndicesQuery {
    add_option!(with_no_match_query, no_match_query, NoMatchQuery);

    build!(Indices);
}

impl ToJson for IndicesQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("indices".to_owned(), self.indices.to_json());
        d.insert("query".to_owned(), self.query.to_json());
        optional_add!(self, d, no_match_query);
        Json::Object(d)
    }
}

/// Options for the `no_match_query` option of IndicesQuery
#[derive(Debug)]
pub enum NoMatchQuery {
    None,
    All,
    Query(Query)
}

from_exp!(Query, NoMatchQuery, from, NoMatchQuery::Query(from));

impl ToJson for NoMatchQuery {
    fn to_json(&self) -> Json {
        match self {
            &NoMatchQuery::None => "none".to_json(),
            &NoMatchQuery::All => "all".to_json(),
            &NoMatchQuery::Query(ref q) => q.to_json()
        }
    }
}
