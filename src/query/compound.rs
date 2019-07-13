/*
 * Copyright 2016-2018 Ben Ashford
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

use serde::{Serialize, Serializer};

use crate::{json::ShouldSkip, units::OneOrMany};

use super::{functions::Function, MinimumShouldMatch, Query, ScoreMode};

/// BoostMode
#[derive(Debug, Copy, Clone)]
pub enum BoostMode {
    Multiply,
    Replace,
    Sum,
    Avg,
    Max,
    Min,
}

impl Serialize for BoostMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BoostMode::Multiply => "multiply",
            BoostMode::Replace => "replace",
            BoostMode::Sum => "sum",
            BoostMode::Avg => "avg",
            BoostMode::Max => "max",
            BoostMode::Min => "min",
        }
        .serialize(serializer)
    }
}

/// Constant score query
#[derive(Debug, Default, Serialize)]
pub struct ConstantScoreQuery {
    query: Query,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    boost: Option<f64>,
}

impl Query {
    pub fn build_constant_score<A>(query: A) -> ConstantScoreQuery
    where
        A: Into<Query>,
    {
        ConstantScoreQuery {
            query: query.into(),
            ..Default::default()
        }
    }
}

impl ConstantScoreQuery {
    add_field!(with_boost, boost, f64);

    build!(ConstantScore);
}

/// Bool query
#[derive(Debug, Default, Serialize)]
pub struct BoolQuery {
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    must: Option<OneOrMany<Query>>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    filter: Option<Query>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    should: Option<OneOrMany<Query>>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    must_not: Option<OneOrMany<Query>>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    minimum_should_match: Option<MinimumShouldMatch>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    boost: Option<f64>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    disable_coord: Option<bool>,
}

impl Query {
    pub fn build_bool() -> BoolQuery {
        Default::default()
    }
}

impl BoolQuery {
    add_field!(with_must, must, OneOrMany<Query>);
    add_field!(with_filter, filter, Query);
    add_field!(with_should, should, OneOrMany<Query>);
    add_field!(with_must_not, must_not, OneOrMany<Query>);
    add_field!(
        with_minimum_should_match,
        minimum_should_match,
        MinimumShouldMatch
    );
    add_field!(with_boost, boost, f64);
    add_field!(with_disable_coord, disable_coord, bool);

    build!(Bool);
}

/// DisMax query
#[derive(Debug, Default, Serialize)]
pub struct DisMaxQuery {
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    tie_breaker: Option<f64>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    boost: Option<f64>,
    queries: Vec<Query>,
}

impl Query {
    pub fn build_dis_max<A>(queries: A) -> DisMaxQuery
    where
        A: Into<Vec<Query>>,
    {
        DisMaxQuery {
            queries: queries.into(),
            ..Default::default()
        }
    }
}

impl DisMaxQuery {
    add_field!(with_tie_breaker, tie_breaker, f64);
    add_field!(with_boost, boost, f64);

    build!(DisMax);
}

/// Function Score query
#[derive(Debug, Default, Serialize)]
pub struct FunctionScoreQuery {
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    query: Option<Query>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    boost: Option<f64>,
    functions: Vec<Function>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    max_boost: Option<f64>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    score_mode: Option<ScoreMode>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    boost_mode: Option<BoostMode>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    min_score: Option<f64>,
}

impl Query {
    pub fn build_function_score() -> FunctionScoreQuery {
        Default::default()
    }
}

impl FunctionScoreQuery {
    add_field!(with_query, query, Query);
    add_field!(with_boost, boost, f64);
    add_field!(with_max_boost, max_boost, f64);
    add_field!(with_score_mode, score_mode, ScoreMode);
    add_field!(with_boost_mode, boost_mode, BoostMode);
    add_field!(with_min_score, min_score, f64);

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

/// Boosting query
#[derive(Debug, Default, Serialize)]
pub struct BoostingQuery {
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    positive: Option<Query>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    negative: Option<Query>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    negative_boost: Option<f64>,
}

impl Query {
    pub fn build_boosting() -> BoostingQuery {
        Default::default()
    }
}

impl BoostingQuery {
    add_field!(with_positive, positive, Query);
    add_field!(with_negative, negative, Query);
    add_field!(with_negative_boost, negative_boost, f64);

    build!(Boosting);
}

/// Indices query
#[derive(Debug, Default, Serialize)]
pub struct IndicesQuery {
    indices: OneOrMany<String>,
    query: Query,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    no_match_query: Option<NoMatchQuery>,
}

impl Query {
    pub fn build_indices<A, B>(indices: A, query: B) -> IndicesQuery
    where
        A: Into<OneOrMany<String>>,
        B: Into<Query>,
    {
        IndicesQuery {
            indices: indices.into(),
            query: query.into(),
            ..Default::default()
        }
    }
}

impl IndicesQuery {
    add_field!(with_no_match_query, no_match_query, NoMatchQuery);

    build!(Indices);
}

/// Options for the `no_match_query` option of IndicesQuery
#[derive(Debug)]
pub enum NoMatchQuery {
    None,
    All,
    Query(Query),
}

from_exp!(Query, NoMatchQuery, from, NoMatchQuery::Query(from));

impl Serialize for NoMatchQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use self::NoMatchQuery::*;
        match self {
            None => "none".serialize(serializer),
            All => "all".serialize(serializer),
            Query(ref q) => q.serialize(serializer),
        }
    }
}
