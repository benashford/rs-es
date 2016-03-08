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

use rustc_serialize::json::{Json, ToJson};

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

impl ToJson for CombinationMinimumShouldMatch {
    fn to_json(&self) -> Json {
        Json::String(self.to_string())
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

impl ToJson for MinimumShouldMatch {
    fn to_json(&self) -> Json {
        match self {
            &MinimumShouldMatch::Integer(val) => val.to_json(),
            &MinimumShouldMatch::Percentage(_) => {
                self.to_string().to_json()
            },
            &MinimumShouldMatch::Combination(ref comb) => {
                comb.to_json()
            },
            &MinimumShouldMatch::MultipleCombination(ref combs) => {
                Json::String(combs.iter().map(|c| c.to_string()).join(" "))
            }
            &MinimumShouldMatch::LowHigh(low, high) => {
                let mut d = BTreeMap::new();
                d.insert("low_freq".to_owned(), low.to_json());
                d.insert("high_freq".to_owned(), high.to_json());
                Json::Object(d)
            }
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

impl ToJson for Fuzziness {
    fn to_json(&self) -> Json {
        use self::Fuzziness::{Auto, LevenshteinDistance, Proportionate};
        match self {
            &Auto                      => "auto".to_json(),
            &LevenshteinDistance(dist) => dist.to_json(),
            &Proportionate(prop)       => prop.to_json()
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

impl<A> ToJson for Flags<A>
where A: AsRef<str> {
    fn to_json(&self) -> Json {
        Json::String(self.0.iter().join("|"))
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

impl ToJson for ScoreMode {
    fn to_json(&self) -> Json {
        match self {
            &ScoreMode::Multiply => "multiply".to_json(),
            &ScoreMode::Sum => "sum".to_json(),
            &ScoreMode::Avg => "avg".to_json(),
            &ScoreMode::First => "first".to_json(),
            &ScoreMode::Max => "max".to_json(),
            &ScoreMode::Min => "min".to_json()
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

    // // Full-text queries
    // Match(Box<full_text::MatchQuery>),
    // MultiMatch(Box<full_text::MultiMatchQuery>),
    // Common(Box<full_text::CommonQuery>),
    // QueryString(Box<full_text::QueryStringQuery>),
    // SimpleQueryString(Box<full_text::SimpleQueryStringQuery>),

    // // Term level queries
    #[serde(rename="term")]
    Term(Box<term::TermQuery>),
    #[serde(rename="terms")]
    Terms(Box<term::TermsQuery>),
    #[serde(rename="range")]
    Range(Box<term::RangeQuery>),
    // Exists(Box<term::ExistsQuery>),
    // // Not implementing the Missing query, as it's deprecated, use `must_not` and `Exists`
    // // instead
    // Prefix(Box<term::PrefixQuery>),
    // Wildcard(Box<term::WildcardQuery>),
    // Regexp(Box<term::RegexpQuery>),
    // Fuzzy(Box<term::FuzzyQuery>),
    // Type(Box<term::TypeQuery>),
    // Ids(Box<term::IdsQuery>),

    // // Compound queries
    // ConstantScore(Box<compound::ConstantScoreQuery>),
    Bool(Box<compound::BoolQuery>),
    // DisMax(Box<compound::DisMaxQuery>),
    // FunctionScore(Box<compound::FunctionScoreQuery>),
    // Boosting(Box<compound::BoostingQuery>),
    // Indices(Box<compound::IndicesQuery>),
    // // Not implementing the And query, as it's deprecated, use `bool` instead.
    // // Not implementing the Not query, as it's deprecated
    // // Not implementing the Or query, as it's deprecated, use `bool` instead.
    // // Not implementing the Filtered query, as it's deprecated.
    // // Not implementing the Limit query, as it's deprecated.

    // // Joining queries
    // Nested(Box<joining::NestedQuery>),
    // HasChild(Box<joining::HasChildQuery>),
    // HasParent(Box<joining::HasParentQuery>),

    // // Geo queries
    // GeoShape(Box<geo::GeoShapeQuery>),
    // GeoBoundingBox(Box<geo::GeoBoundingBoxQuery>),
    // GeoDistance(Box<geo::GeoDistanceQuery>),
    // // TODO: implement me - pending changes to range query
    // //GeoDistanceRange(Box<geo::GeoDistanceRangeQuery>)
    // GeoPolygon(Box<geo::GeoPolygonQuery>),
    // GeohashCell(Box<geo::GeohashCellQuery>),

    // // Specialized queries
    // MoreLikeThis(Box<specialized::MoreLikeThisQuery>),
    // // TODO: template queries
    // // TODO: Search by script

    // // Span queries
    // // TODO: SpanTerm(Box<term::TermQuery>),
    // // TODO: Span multi term query
    // // TODO: Span first query
    // // TODO: Span near query
    // // TODO: Span or query
    // // TODO: Span not query
    // // TODO: Span containing query
    // // TODO: Span within query
}

impl Default for Query {
    fn default() -> Query {
        // TODO - put this back
        //Query::MatchAll(Default::default())
        Query::build_terms("test_idx").build()
    }
}

/// Convert a Query to Json
impl ToJson for Query {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        match self {
            &Query::MatchAll(ref q) => {
                d.insert("match_all".to_owned(), q.to_json());
            },
            // &Query::Match(ref q) => {
            //     d.insert("match".to_owned(), q.to_json());
            // },
            // &Query::MultiMatch(ref q) => {
            //     d.insert("multi_match".to_owned(), q.to_json());
            // },
            // &Query::Common(ref q) => {
            //     d.insert("common".to_owned(), q.to_json());
            // },
            // &Query::QueryString(ref q) => {
            //     d.insert("query_string".to_owned(), q.to_json());
            // },
            // &Query::SimpleQueryString(ref q) => {
            //     d.insert("simple_query_string".to_owned(), q.to_json());
            // },
            &Query::Term(ref q) => {
                d.insert("term".to_owned(), q.to_json());
            },
            &Query::Terms(ref q) => {
                d.insert("terms".to_owned(), q.to_json());
            },
            &Query::Range(ref q) => {
                d.insert("range".to_owned(), q.to_json());
            },
            // &Query::Exists(ref q) => {
            //     d.insert("exists".to_owned(), q.to_json());
            // },
            // &Query::Prefix(ref q) => {
            //     d.insert("prefix".to_owned(), q.to_json());
            // },
            // &Query::Wildcard(ref q) => {
            //     d.insert("wildcard".to_owned(), q.to_json());
            // },
            // &Query::Regexp(ref q) => {
            //     d.insert("regexp".to_owned(), q.to_json());
            // },
            // &Query::Fuzzy(ref q) => {
            //     d.insert("fuzzy".to_owned(), q.to_json());
            // },
            // &Query::Type(ref q) => {
            //     d.insert("type".to_owned(), q.to_json());
            // },
            // &Query::Ids(ref q) => {
            //     d.insert("ids".to_owned(), q.to_json());
            // },
            // &Query::ConstantScore(ref q) => {
            //     d.insert("constant_score".to_owned(), q.to_json());
            // },
            &Query::Bool(ref q) => {
                d.insert("bool".to_owned(), q.to_json());
            },
            // &Query::DisMax(ref q) => {
            //     d.insert("dis_max".to_owned(), q.to_json());
            // },
            // &Query::FunctionScore(ref q) => {
            //     d.insert("function_score".to_owned(), q.to_json());
            // },
            // &Query::Boosting(ref q) => {
            //     d.insert("boosting".to_owned(), q.to_json());
            // },
            // &Query::Indices(ref q) => {
            //     d.insert("indices".to_owned(), q.to_json());
            // },
            // &Query::Nested(ref q) => {
            //     d.insert("nested".to_owned(), q.to_json());
            // },
            // &Query::HasChild(ref q) => {
            //     d.insert("has_child".to_owned(), q.to_json());
            // },
            // &Query::HasParent(ref q) => {
            //     d.insert("has_parent".to_owned(), q.to_json());
            // },
            // &Query::GeoShape(ref q) => {
            //     d.insert("geo_shape".to_owned(), q.to_json());
            // },
            // &Query::GeoBoundingBox(ref q) => {
            //     d.insert("geo_bounding_box".to_owned(), q.to_json());
            // },
            // &Query::GeoDistance(ref q) => {
            //     d.insert("geo_distance".to_owned(), q.to_json());
            // },
            // &Query::GeoPolygon(ref q) => {
            //     d.insert("geo_polygon".to_owned(), q.to_json());
            // },
            // &Query::GeohashCell(ref q) => {
            //     d.insert("geohash_cell".to_owned(), q.to_json());
            // },
            // &Query::MoreLikeThis(ref q) => {
            //     d.insert("more_like_this".to_owned(), q.to_json());
            // },
        }
        Json::Object(d)
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

impl ToJson for MatchAllQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        optional_add!(self, d, boost);
        Json::Object(d)
    }
}

#[cfg(test)]
mod tests {
    // TODO: replace with Serde
    extern crate rustc_serialize;

    extern crate serde_json;

    use rustc_serialize::json::ToJson;

    use super::{Flags, Query};
    use super::full_text::SimpleQueryStringFlags;
    use super::functions::Function;
    use super::term::TermsQueryLookup;

    #[test]
    fn test_simple_query_string_flags() {
        let opts = vec![SimpleQueryStringFlags::And, SimpleQueryStringFlags::Not];
        let flags:Flags<SimpleQueryStringFlags> = opts.into();
        let json = flags.to_json();
        assert_eq!("AND|NOT", json.as_string().unwrap());
    }

    #[test]
    fn test_terms_query() {
        let terms_query = Query::build_terms("field_name")
            .with_values(vec!["a", "b", "c"])
            .build();
        assert_eq!("{\"terms\":{\"field_name\":[\"a\",\"b\",\"c\"]}}",
                   serde_json::to_string(&terms_query).unwrap());
        assert_eq!("{\"terms\":{\"field_name\":[\"a\",\"b\",\"c\"]}}",
                   terms_query.to_json().to_string());

        let terms_query_2 = Query::build_terms("field_name")
            .with_values(["a", "b", "c"].as_ref())
            .build();
        assert_eq!("{\"terms\":{\"field_name\":[\"a\",\"b\",\"c\"]}}",
                   serde_json::to_string(&terms_query_2).unwrap());
        assert_eq!("{\"terms\":{\"field_name\":[\"a\",\"b\",\"c\"]}}",
                   terms_query_2.to_json().to_string());

        let terms_query_3 = Query::build_terms("field_name")
            .with_values(TermsQueryLookup::new(123, "blah.de.blah").with_index("other"))
            .build();
        assert_eq!("{\"terms\":{\"field_name\":{\"id\":123,\"index\":\"other\",\"path\":\"blah.de.blah\"}}}",
                   serde_json::to_string(&terms_query_3).unwrap());
        assert_eq!("{\"terms\":{\"field_name\":{\"id\":123,\"index\":\"other\",\"path\":\"blah.de.blah\"}}}",
                   terms_query_3.to_json().to_string());
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
