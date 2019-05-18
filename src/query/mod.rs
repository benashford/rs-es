/*
 * Copyright 2015-2019 Ben Ashford
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

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

use crate::{json::ShouldSkip, util::StrJoin};

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
    second: MinimumShouldMatch,
}

impl CombinationMinimumShouldMatch {
    pub fn new<A, B>(first: A, second: B) -> CombinationMinimumShouldMatch
    where
        A: Into<MinimumShouldMatch>,
        B: Into<MinimumShouldMatch>,
    {
        CombinationMinimumShouldMatch {
            first: first.into(),
            second: second.into(),
        }
    }
}

impl ToString for CombinationMinimumShouldMatch {
    fn to_string(&self) -> String {
        format!("{}<{}", self.first.to_string(), self.second.to_string())
    }
}

impl Serialize for CombinationMinimumShouldMatch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

#[derive(Debug)]
pub enum MinimumShouldMatch {
    Integer(i64),
    Percentage(f64),
    Combination(Box<CombinationMinimumShouldMatch>),
    MultipleCombination(Vec<CombinationMinimumShouldMatch>),
    LowHigh(i64, i64),
}

from!(i64, MinimumShouldMatch, Integer);
from!(f64, MinimumShouldMatch, Percentage);
from_exp!(
    CombinationMinimumShouldMatch,
    MinimumShouldMatch,
    from,
    MinimumShouldMatch::Combination(Box::new(from))
);
from!(
    Vec<CombinationMinimumShouldMatch>,
    MinimumShouldMatch,
    MultipleCombination
);
from_exp!(
    (i64, i64),
    MinimumShouldMatch,
    from,
    MinimumShouldMatch::LowHigh(from.0, from.1)
);

impl ToString for MinimumShouldMatch {
    fn to_string(&self) -> String {
        match self {
            MinimumShouldMatch::Integer(val) => val.to_string(),
            MinimumShouldMatch::Percentage(val) => format!("{}%", val),
            _ => panic!("Can't convert {:?} to String", self),
        }
    }
}

impl Serialize for MinimumShouldMatch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            MinimumShouldMatch::Integer(val) => val.serialize(serializer),
            MinimumShouldMatch::Percentage(_) => self.to_string().serialize(serializer),
            MinimumShouldMatch::Combination(ref comb) => comb.serialize(serializer),
            MinimumShouldMatch::MultipleCombination(ref combs) => combs
                .iter()
                .map(ToString::to_string)
                .join(" ")
                .serialize(serializer),
            MinimumShouldMatch::LowHigh(low, high) => {
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
    Proportionate(f64),
}

from!(i64, Fuzziness, LevenshteinDistance);
from!(f64, Fuzziness, Proportionate);

impl Serialize for Fuzziness {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use self::Fuzziness::*;
        match self {
            Auto => "auto".serialize(serializer),
            LevenshteinDistance(dist) => dist.serialize(serializer),
            Proportionate(p) => p.serialize(serializer),
        }
    }
}

// Flags

/// Flags - multiple operations can take a set of flags, each set is dependent
/// on the operation in question, but they're all formatted to a similar looking
/// String
#[derive(Debug)]
pub struct Flags<A>(Vec<A>)
where
    A: AsRef<str>;

impl<A> Serialize for Flags<A>
where
    A: AsRef<str>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.iter().join("|").serialize(serializer)
    }
}

impl<A> From<Vec<A>> for Flags<A>
where
    A: AsRef<str>,
{
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
    Min,
}

impl Serialize for ScoreMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ScoreMode::Multiply => "multiply".serialize(serializer),
            ScoreMode::Sum => "sum".serialize(serializer),
            ScoreMode::Avg => "avg".serialize(serializer),
            ScoreMode::First => "first".serialize(serializer),
            ScoreMode::Max => "max".serialize(serializer),
            ScoreMode::Min => "min".serialize(serializer),
        }
    }
}

/// Query represents all available queries
///
/// Each value is boxed as Queries can be recursive, they also vary
/// significantly in size

// TODO: Filters and Queries are merged, ensure all filters are included in this enum
#[derive(Debug)]
pub enum Query {
    MatchAll(Box<MatchAllQuery>),

    // Full-text queries
    Match(Box<full_text::MatchQuery>),
    MultiMatch(Box<full_text::MultiMatchQuery>),
    Common(Box<full_text::CommonQuery>),
    QueryString(Box<full_text::QueryStringQuery>),
    SimpleQueryString(Box<full_text::SimpleQueryStringQuery>),

    // Term level queries
    Term(Box<term::TermQuery>),
    Terms(Box<term::TermsQuery>),
    Range(Box<term::RangeQuery>),
    Exists(Box<term::ExistsQuery>),
    // Not implementing the Missing query, as it's deprecated, use `must_not` and `Exists`
    // instead
    Prefix(Box<term::PrefixQuery>),
    Wildcard(Box<term::WildcardQuery>),
    Regexp(Box<term::RegexpQuery>),
    Fuzzy(Box<term::FuzzyQuery>),
    Type(Box<term::TypeQuery>),
    Ids(Box<term::IdsQuery>),

    // Compound queries
    ConstantScore(Box<compound::ConstantScoreQuery>),
    Bool(Box<compound::BoolQuery>),
    DisMax(Box<compound::DisMaxQuery>),
    FunctionScore(Box<compound::FunctionScoreQuery>),
    Boosting(Box<compound::BoostingQuery>),
    Indices(Box<compound::IndicesQuery>),
    // Not implementing the And query, as it's deprecated, use `bool` instead.
    // Not implementing the Not query, as it's deprecated
    // Not implementing the Or query, as it's deprecated, use `bool` instead.
    // Not implementing the Filtered query, as it's deprecated.
    // Not implementing the Limit query, as it's deprecated.

    // Joining queries
    Nested(Box<joining::NestedQuery>),
    HasChild(Box<joining::HasChildQuery>),
    HasParent(Box<joining::HasParentQuery>),

    // Geo queries
    GeoShape(Box<geo::GeoShapeQuery>),
    GeoBoundingBox(Box<geo::GeoBoundingBoxQuery>),
    GeoDistance(Box<geo::GeoDistanceQuery>),
    // TODO: implement me - pending changes to range query
    //GeoDistanceRange(Box<geo::GeoDistanceRangeQuery>)
    GeoPolygon(Box<geo::GeoPolygonQuery>),
    GeohashCell(Box<geo::GeohashCellQuery>),

    // Specialized queries
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

impl Serialize for Query {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use self::Query::*;

        let mut map_ser = serializer.serialize_map(Some(1))?;
        (match self {
            // All
            MatchAll(ref q) => map_ser.serialize_entry("match_all", q),

            // Full-text
            Match(ref q) => map_ser.serialize_entry("match", q),
            MultiMatch(ref q) => map_ser.serialize_entry("multi_match", q),
            Common(ref q) => map_ser.serialize_entry("common", q),
            QueryString(ref q) => map_ser.serialize_entry("query_string", q),
            SimpleQueryString(ref q) => map_ser.serialize_entry("simple_query_string", q),

            // Term
            Term(ref q) => map_ser.serialize_entry("term", q),
            Terms(ref q) => map_ser.serialize_entry("terms", q),
            Range(ref q) => map_ser.serialize_entry("range", q),
            Exists(ref q) => map_ser.serialize_entry("exists", q),
            Prefix(ref q) => map_ser.serialize_entry("prefix", q),
            Wildcard(ref q) => map_ser.serialize_entry("wildcard", q),
            Regexp(ref q) => map_ser.serialize_entry("regexp", q),
            Fuzzy(ref q) => map_ser.serialize_entry("fuzzy", q),
            Type(ref q) => map_ser.serialize_entry("type", q),
            Ids(ref q) => map_ser.serialize_entry("ids", q),

            // Compound
            ConstantScore(ref q) => map_ser.serialize_entry("constant_score", q),
            Bool(ref q) => map_ser.serialize_entry("bool", q),
            DisMax(ref q) => map_ser.serialize_entry("dis_max", q),
            FunctionScore(ref q) => map_ser.serialize_entry("function_score", q),
            Boosting(ref q) => map_ser.serialize_entry("boosting", q),
            Indices(ref q) => map_ser.serialize_entry("indices", q),

            // Joining
            Nested(ref q) => map_ser.serialize_entry("nested", q),
            HasChild(ref q) => map_ser.serialize_entry("has_child", q),
            HasParent(ref q) => map_ser.serialize_entry("has_parent", q),

            // Geo
            GeoShape(ref q) => map_ser.serialize_entry("geo_shape", q),
            GeoBoundingBox(ref q) => map_ser.serialize_entry("geo_bounding_box", q),
            GeoDistance(ref q) => map_ser.serialize_entry("geo_distance", q),
            GeoPolygon(ref q) => map_ser.serialize_entry("geo_polygon", q),
            GeohashCell(ref q) => map_ser.serialize_entry("geohash_cell", q),

            // Specialized
            MoreLikeThis(ref q) => map_ser.serialize_entry("more_like_this", q),
        })?;
        map_ser.end()
    }
}

// Specific query types go here

/// Match all query

#[derive(Debug, Default, Serialize)]
pub struct MatchAllQuery {
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    boost: Option<f64>,
}

impl Query {
    pub fn build_match_all() -> MatchAllQuery {
        MatchAllQuery::default()
    }
}

impl MatchAllQuery {
    add_field!(with_boost, boost, f64);

    build!(MatchAll);
}

#[cfg(test)]
mod tests {
    extern crate serde_json;

    use super::full_text::SimpleQueryStringFlags;
    use super::functions::Function;
    use super::term::TermsQueryLookup;
    use super::{Flags, Query};

    #[test]
    fn test_simple_query_string_flags() {
        let opts = vec![SimpleQueryStringFlags::And, SimpleQueryStringFlags::Not];
        let flags: Flags<SimpleQueryStringFlags> = opts.into();
        let json = serde_json::to_string(&flags);
        assert_eq!("\"AND|NOT\"", json.unwrap());
    }

    #[test]
    fn test_terms_query() {
        let terms_query = Query::build_terms("field_name")
            .with_values(vec!["a", "b", "c"])
            .build();
        assert_eq!(
            "{\"terms\":{\"field_name\":[\"a\",\"b\",\"c\"]}}",
            serde_json::to_string(&terms_query).unwrap()
        );

        let terms_query_2 = Query::build_terms("field_name")
            .with_values(["a", "b", "c"].as_ref())
            .build();
        assert_eq!(
            "{\"terms\":{\"field_name\":[\"a\",\"b\",\"c\"]}}",
            serde_json::to_string(&terms_query_2).unwrap()
        );

        let terms_query_3 = Query::build_terms("field_name")
            .with_values(TermsQueryLookup::new(123, "blah.de.blah").with_index("other"))
            .build();
        assert_eq!("{\"terms\":{\"field_name\":{\"id\":123,\"index\":\"other\",\"path\":\"blah.de.blah\"}}}",
                   serde_json::to_string(&terms_query_3).unwrap());
    }

    #[test]
    fn test_function_score_query() {
        let function_score_query = Query::build_function_score()
            .with_function(
                Function::build_script_score("this_is_a_script")
                    .with_lang("made_up")
                    .add_param("A", 12)
                    .build(),
            )
            .build();
        assert_eq!("{\"function_score\":{\"functions\":[{\"script_score\":{\"lang\":\"made_up\",\"params\":{\"A\":12},\"inline\":\"this_is_a_script\"}}]}}",
                   serde_json::to_string(&function_score_query).unwrap());
    }

    #[test]
    fn test_exists_query() {
        let exists_query = Query::build_exists("name").build();
        assert_eq!(
            "{\"exists\":{\"field\":\"name\"}}",
            serde_json::to_string(&exists_query).unwrap()
        );
    }
}
