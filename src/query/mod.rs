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

//! Implementation of the ElasticSearch Query DSL.
//!
//! ElasticSearch offers a
//! [rich DSL for searches](https://www.elastic.co/guide/en/elasticsearch/reference/1.x/query-dsl.html).
//! It is JSON based, and therefore very easy to use and composable if using from a
//! dynamic language (e.g.
//! [Ruby](https://github.com/elastic/elasticsearch-ruby/tree/master/elasticsearch-dsl#features-overview));
//! but Rust, being a staticly-typed language, things are different.  The `rs_es::query`
//! module defines a set of builder objects which can be similarly composed to the same
//! ends.
//!
//! For example:
//!
//! ```rust
//! use rs_es::query::Query;
//!
//! let query = Query::build_bool()
//!     .with_must(vec![Query::build_term("field_a",
//!                                       "value").build(),
//!                     Query::build_range("field_b")
//!                           .with_gte(5)
//!                           .with_lt(10)
//!                           .build()])
//!     .build();
//! ```

use std::collections::BTreeMap;

use serde::{Serialize, Serializer};

use ::json::ShouldSkip;
use ::util::StrJoin;

#[macro_use]
mod common;

pub mod compound;
pub mod full_text;
pub mod functions;
pub mod geo;
pub mod joining;
pub mod specialized;
pub mod term;

// Miscellaneous types required by queries go here

// Enums

/// Minimum should match - used in numerous queries
/// TODO: should go somewhere specific
#[derive(Debug)]
pub struct CombinationMinimumShouldMatch {
    first: MinimumShouldMatch,
    second: MinimumShouldMatch
}

impl CombinationMinimumShouldMatch {
    pub fn new<A, B>(first: A, second: B) -> CombinationMinimumShouldMatch
        where A: Into<MinimumShouldMatch>,
              B: Into<MinimumShouldMatch>
    {
        CombinationMinimumShouldMatch {
            first:  first.into(),
            second: second.into()
        }
    }
}

impl ToString for CombinationMinimumShouldMatch {
    fn to_string(&self) -> String {
        format!("{}<{}", self.first.to_string(), self.second.to_string())
    }
}

impl Serialize for CombinationMinimumShouldMatch {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        self.to_string().serialize(serializer)
    }
}

#[derive(Debug)]
pub enum MinimumShouldMatch {
    Integer(i64),
    Percentage(f64),
    Combination(Box<CombinationMinimumShouldMatch>),
    MultipleCombination(Vec<CombinationMinimumShouldMatch>),
    LowHigh(i64, i64)
}

from!(i64, MinimumShouldMatch, Integer);
from!(f64, MinimumShouldMatch, Percentage);
from_exp!(CombinationMinimumShouldMatch,
          MinimumShouldMatch,
          from,
          MinimumShouldMatch::Combination(Box::new(from)));
from!(Vec<CombinationMinimumShouldMatch>, MinimumShouldMatch, MultipleCombination);
from_exp!((i64, i64),
          MinimumShouldMatch,
          from,
          MinimumShouldMatch::LowHigh(from.0, from.1));

impl ToString for MinimumShouldMatch {
    fn to_string(&self) -> String {
        match self {
            &MinimumShouldMatch::Integer(val) => val.to_string(),
            &MinimumShouldMatch::Percentage(val) => {
                format!("{}%", val)
            },
            _ => panic!("Can't convert {:?} to String", self)
        }
    }
}

impl Serialize for MinimumShouldMatch {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        match self {
            &MinimumShouldMatch::Integer(val) => val.serialize(serializer),
            &MinimumShouldMatch::Percentage(_) => self.to_string().serialize(serializer),
            &MinimumShouldMatch::Combination(ref comb) => comb.serialize(serializer),
            &MinimumShouldMatch::MultipleCombination(ref combs) => {
                combs.iter().map(|c| c.to_string()).join(" ").serialize(serializer)
            },
            &MinimumShouldMatch::LowHigh(low, high) => {
                let mut d = BTreeMap::new();
                d.insert("low_freq", low);
                d.insert("high_freq", high);
                d.serialize(serializer)
            }
        }
    }
}

/// Fuzziness
#[derive(Debug)]
pub enum Fuzziness {
    Auto,
    LevenshteinDistance(i64),
    Proportionate(f64)
}

from!(i64, Fuzziness, LevenshteinDistance);
from!(f64, Fuzziness, Proportionate);

impl Serialize for Fuzziness {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        use self::Fuzziness::*;
        match self {
            &Auto => "auto".serialize(serializer),
            &LevenshteinDistance(dist) => dist.serialize(serializer),
            &Proportionate(p) => p.serialize(serializer)
        }
    }
}

// Flags

/// Flags - multiple operations can take a set of flags, each set is dependent
/// on the operation in question, but they're all formatted to a similar looking
/// String
#[derive(Debug)]
pub struct Flags<A>(Vec<A>)
    where A: AsRef<str>;

impl<A> Serialize for Flags<A>
    where A: AsRef<str> {

    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        self.0.iter().join("|").serialize(serializer)
    }
}

impl<A> From<Vec<A>> for Flags<A>
    where A: AsRef<str> {

    fn from(from: Vec<A>) -> Self {
        Flags(from)
    }
}

/// ScoreMode
#[derive(Debug)]
pub enum ScoreMode {
    Multiply,
    Sum,
    Avg,
    First,
    Max,
    Min
}

impl Serialize for ScoreMode {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        match self {
            &ScoreMode::Multiply => "multiply".serialize(serializer),
            &ScoreMode::Sum => "sum".serialize(serializer),
            &ScoreMode::Avg => "avg".serialize(serializer),
            &ScoreMode::First => "first".serialize(serializer),
            &ScoreMode::Max => "max".serialize(serializer),
            &ScoreMode::Min => "min".serialize(serializer)
        }
    }
}

/// Query represents all available queries
///
/// Each value is boxed as Queries can be recursive, they also vary
/// significantly in size

// TODO: Filters and Queries are merged, ensure all filters are included in this enum
#[derive(Debug, Serialize)]
pub enum Query {
    // TODO - uncomment one-level to re-enable after Serdeification
    #[serde(rename="match_all")]
    MatchAll(Box<MatchAllQuery>),

    // Full-text queries
    #[serde(rename="match")]
    Match(Box<full_text::MatchQuery>),
    #[serde(rename="multi_match")]
    MultiMatch(Box<full_text::MultiMatchQuery>),
    #[serde(rename="common")]
    Common(Box<full_text::CommonQuery>),
    #[serde(rename="query_string")]
    QueryString(Box<full_text::QueryStringQuery>),
    #[serde(rename="simple_query_string")]
    SimpleQueryString(Box<full_text::SimpleQueryStringQuery>),

    // // Term level queries
    #[serde(rename="term")]
    Term(Box<term::TermQuery>),
    #[serde(rename="terms")]
    Terms(Box<term::TermsQuery>),
    #[serde(rename="range")]
    Range(Box<term::RangeQuery>),
    #[serde(rename="exists")]
    Exists(Box<term::ExistsQuery>),
    // // Not implementing the Missing query, as it's deprecated, use `must_not` and `Exists`
    // // instead
    #[serde(rename="prefix")]
    Prefix(Box<term::PrefixQuery>),
    #[serde(rename="wildcard")]
    Wildcard(Box<term::WildcardQuery>),
    #[serde(rename="regexp")]
    Regexp(Box<term::RegexpQuery>),
    #[serde(rename="fuzzy")]
    Fuzzy(Box<term::FuzzyQuery>),
    #[serde(rename="type")]
    Type(Box<term::TypeQuery>),
    #[serde(rename="ids")]
    Ids(Box<term::IdsQuery>),

    // Compound queries
    #[serde(rename="constant_score")]
    ConstantScore(Box<compound::ConstantScoreQuery>),
    #[serde(rename="bool")]
    Bool(Box<compound::BoolQuery>),
    #[serde(rename="dis_max")]
    DisMax(Box<compound::DisMaxQuery>),
    #[serde(rename="function_score")]
    FunctionScore(Box<compound::FunctionScoreQuery>),
    #[serde(rename="boosting")]
    Boosting(Box<compound::BoostingQuery>),
    #[serde(rename="indices")]
    Indices(Box<compound::IndicesQuery>),
    // // Not implementing the And query, as it's deprecated, use `bool` instead.
    // // Not implementing the Not query, as it's deprecated
    // // Not implementing the Or query, as it's deprecated, use `bool` instead.
    // // Not implementing the Filtered query, as it's deprecated.
    // // Not implementing the Limit query, as it's deprecated.

    // Joining queries
    #[serde(rename="nested")]
    Nested(Box<joining::NestedQuery>),
    #[serde(rename="has_child")]
    HasChild(Box<joining::HasChildQuery>),
    #[serde(rename="has_parent")]
    HasParent(Box<joining::HasParentQuery>),

    // Geo queries
    #[serde(rename="geo_shape")]
    GeoShape(Box<geo::GeoShapeQuery>),
    #[serde(rename="geo_bounding_box")]
    GeoBoundingBox(Box<geo::GeoBoundingBoxQuery>),
    #[serde(rename="geo_distance")]
    GeoDistance(Box<geo::GeoDistanceQuery>),
    // TODO: implement me - pending changes to range query
    //GeoDistanceRange(Box<geo::GeoDistanceRangeQuery>)
    #[serde(rename="geo_polygon")]
    GeoPolygon(Box<geo::GeoPolygonQuery>),
    #[serde(rename="geohash_cell")]
    GeohashCell(Box<geo::GeohashCellQuery>),

    // Specialized queries
    #[serde(rename="more_like_this")]
    MoreLikeThis(Box<specialized::MoreLikeThisQuery>),
    // TODO: template queries
    // TODO: Search by script

    // Span queries
    // TODO: SpanTerm(Box<term::TermQuery>),
    // TODO: Span multi term query
    // TODO: Span first query
    // TODO: Span near query
    // TODO: Span or query
    // TODO: Span not query
    // TODO: Span containing query
    // TODO: Span within query
}

impl Default for Query {
    fn default() -> Query {
        Query::MatchAll(Default::default())
    }
}

// Specific query types go here

/// Match all query

#[derive(Debug, Default, Serialize)]
pub struct MatchAllQuery {
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    boost: Option<f64>,
}

impl Query {
    pub fn build_match_all() -> MatchAllQuery {
        MatchAllQuery::default()
    }
}

impl MatchAllQuery {
    add_option!(with_boost, boost, f64);

    build!(MatchAll);
}

#[cfg(test)]
mod tests {
    extern crate serde_json;

    use super::{Flags, Query};
    use super::full_text::SimpleQueryStringFlags;
    use super::functions::Function;
    use super::term::TermsQueryLookup;

    #[test]
    fn test_simple_query_string_flags() {
        let opts = vec![SimpleQueryStringFlags::And, SimpleQueryStringFlags::Not];
        let flags:Flags<SimpleQueryStringFlags> = opts.into();
        let json = serde_json::to_string(&flags);
        assert_eq!("\"AND|NOT\"", json.unwrap());
    }

    #[test]
    fn test_terms_query() {
        let terms_query = Query::build_terms("field_name")
            .with_values(vec!["a", "b", "c"])
            .build();
        assert_eq!("{\"terms\":{\"field_name\":[\"a\",\"b\",\"c\"]}}",
                   serde_json::to_string(&terms_query).unwrap());

        let terms_query_2 = Query::build_terms("field_name")
            .with_values(["a", "b", "c"].as_ref())
            .build();
        assert_eq!("{\"terms\":{\"field_name\":[\"a\",\"b\",\"c\"]}}",
                   serde_json::to_string(&terms_query_2).unwrap());

        let terms_query_3 = Query::build_terms("field_name")
            .with_values(TermsQueryLookup::new(123, "blah.de.blah").with_index("other"))
            .build();
        assert_eq!("{\"terms\":{\"field_name\":{\"id\":123,\"index\":\"other\",\"path\":\"blah.de.blah\"}}}",
                   serde_json::to_string(&terms_query_3).unwrap());
    }

    // TODO - re-enable
    // #[test]
    // fn test_function_score_query() {
    //     let function_score_query = Query::build_function_score()
    //         .with_function(Function::build_script_score("this_is_a_script")
    //                        .with_lang("made_up")
    //                        .add_param("A", 12)
    //                        .build())
    //         .build();
    //     assert_eq!("{\"function_score\":{\"functions\":[{\"script_score\":{\"inline\":\"this_is_a_script\",\"lang\":\"made_up\",\"params\":{\"A\":12}}}]}}",
    //                function_score_query.to_json().to_string());
    // }
}
