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
//! Previously auto-generated, changes in 2.0 of ElasticSearch have reduced the
//! duplication so this file is in progress of being converted into hand-edited
//! code.

use std::collections::BTreeMap;

use rustc_serialize::json::{Json, ToJson};

use units::{DistanceType,
            DistanceUnit,
            Duration,
            GeoBox,
            JsonPotential,
            JsonVal,
            Location,
            OneOrMany};
use util::StrJoin;

#[macro_use]
mod common;

pub mod compound;
pub mod full_text;
pub mod functions;
pub mod geo;
pub mod joining;
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

    // TODO: below this line, not yet converted
//    FuzzyLikeThis(FuzzyLikeThisQuery),
//    FuzzyLikeThisField(FuzzyLikeThisFieldQuery),

//    Indices(IndicesQuery),
//    MoreLikeThis(MoreLikeThisQuery),

//    SpanFirst(SpanFirstQuery),
//    SpanMulti(SpanMultiQuery),
//    SpanNear(SpanNearQuery),
//    SpanNot(SpanNotQuery),
//    SpanOr(SpanOrQuery<'a>),
//    SpanTerm(SpanTermQuery)
}

impl Default for Query {
    fn default() -> Query {
        Query::MatchAll(Default::default())
    }
}

/// Convert a Query to Json
impl ToJson for Query {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::<String, Json>::new();
        match self {
            &Query::MatchAll(ref q) => {
                d.insert("match_all".to_owned(), q.to_json());
            },
            &Query::Match(ref q) => {
                d.insert("match".to_owned(), q.to_json());
            },
            &Query::MultiMatch(ref q) => {
                d.insert("multi_match".to_owned(), q.to_json());
            },
            &Query::Common(ref q) => {
                d.insert("common".to_owned(), q.to_json());
            },
            &Query::QueryString(ref q) => {
                d.insert("query_string".to_owned(), q.to_json());
            },
            &Query::SimpleQueryString(ref q) => {
                d.insert("simple_query_string".to_owned(), q.to_json());
            },
            &Query::Term(ref q) => {
                d.insert("term".to_owned(), q.to_json());
            },
            &Query::Terms(ref q) => {
                d.insert("terms".to_owned(), q.to_json());
            },
            &Query::Range(ref q) => {
                d.insert("range".to_owned(), q.to_json());
            },
            &Query::Exists(ref q) => {
                d.insert("exists".to_owned(), q.to_json());
            },
            &Query::Prefix(ref q) => {
                d.insert("prefix".to_owned(), q.to_json());
            },
            &Query::Wildcard(ref q) => {
                d.insert("wildcard".to_owned(), q.to_json());
            },
            &Query::Regexp(ref q) => {
                d.insert("regexp".to_owned(), q.to_json());
            },
            &Query::Fuzzy(ref q) => {
                d.insert("fuzzy".to_owned(), q.to_json());
            },
            &Query::Type(ref q) => {
                d.insert("type".to_owned(), q.to_json());
            },
            &Query::Ids(ref q) => {
                d.insert("ids".to_owned(), q.to_json());
            },
            &Query::ConstantScore(ref q) => {
                d.insert("constant_score".to_owned(), q.to_json());
            },
            &Query::Bool(ref q) => {
                d.insert("bool".to_owned(), q.to_json());
            },
            &Query::DisMax(ref q) => {
                d.insert("dis_max".to_owned(), q.to_json());
            },
            &Query::FunctionScore(ref q) => {
                d.insert("function_score".to_owned(), q.to_json());
            },
            &Query::Boosting(ref q) => {
                d.insert("boosting".to_owned(), q.to_json());
            },
            &Query::Indices(ref q) => {
                d.insert("indices".to_owned(), q.to_json());
            },
            &Query::Nested(ref q) => {
                d.insert("nested".to_owned(), q.to_json());
            },
            &Query::HasChild(ref q) => {
                d.insert("has_child".to_owned(), q.to_json());
            },
            &Query::HasParent(ref q) => {
                d.insert("has_parent".to_owned(), q.to_json());
            },
            &Query::GeoShape(ref q) => {
                d.insert("geo_shape".to_owned(), q.to_json());
            },
            &Query::GeoBoundingBox(ref q) => {
                d.insert("geo_bounding_box".to_owned(), q.to_json());
            },
            &Query::GeoDistance(ref q) => {
                d.insert("geo_distance".to_owned(), q.to_json());
            }
        }
        Json::Object(d)
    }
}

// Specific query types go here

/// Match all query

#[derive(Debug, Default)]
pub struct MatchAllQuery {
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
        let mut inner = BTreeMap::new();
        optional_add!(self, inner, boost);
        d.insert("match_all".to_owned(), Json::Object(inner));
        Json::Object(d)
    }
}

// Old queries - TODO: move or delete these

impl Query {
                  pub fn build_fuzzy_like_this<A: Into<String>>(

                         like_text: A
                     ) -> FuzzyLikeThisQuery {

                         FuzzyLikeThisQuery {

                                 fields:
                                                     None
                                                 ,

                                 like_text:
                                                     like_text.into()
                                                 ,

                                 ignore_tf:
                                                     None
                                                 ,

                                 max_query_terms:
                                                     None
                                                 ,

                                 fuzziness:
                                                     None
                                                 ,

                                 prefix_length:
                                                     None
                                                 ,

                                 boost:
                                                     None
                                                 ,

                                 analyzer:
                                                     None


                          }

                  }

                  pub fn build_fuzzy_like_this_field<A: Into<String>,B: Into<String>>(

                         field: A,

                         like_text: B
                     ) -> FuzzyLikeThisFieldQuery {

                         FuzzyLikeThisFieldQuery {

                                 field:
                                                     field.into()
                                                 ,

                                 like_text:
                                                     like_text.into()
                                                 ,

                                 ignore_tf:
                                                     None
                                                 ,

                                 max_query_terms:
                                                     None
                                                 ,

                                 fuzziness:
                                                     None
                                                 ,

                                 prefix_length:
                                                     None
                                                 ,

                                 boost:
                                                     None
                                                 ,

                                 analyzer:
                                                     None


                          }

                  }

                  pub fn build_more_like_this(
                     ) -> MoreLikeThisQuery {

                         MoreLikeThisQuery {

                                 fields:
                                                     None
                                                 ,

                                 like_text:
                                                     None
                                                 ,

                                 ids:
                                                     None
                                                 ,

                                 docs:
                                                     None
                                                 ,

                                 max_query_terms:
                                                     None
                                                 ,

                                 min_term_freq:
                                                     None
                                                 ,

                                 min_doc_freq:
                                                     None
                                                 ,

                                 max_doc_freq:
                                                     None
                                                 ,

                                 min_word_length:
                                                     None
                                                 ,

                                 max_word_length:
                                                     None
                                                 ,

                                 stop_words:
                                                     None
                                                 ,

                                 analyzer:
                                                     None
                                                 ,

                                 minimum_should_match:
                                                     None
                                                 ,

                                 boost_terms:
                                                     None
                                                 ,

                                 include:
                                                     None
                                                 ,

                                 boost:
                                                     None


                          }

                  }

                  // pub fn build_span_first<A: Into<Box<Query>>,B: Into<i64>>(

                  //        span_match: A,

                  //        end: B
                  //    ) -> SpanFirstQuery {

                  //        SpanFirstQuery {

                  //                span_match:
                  //                                    span_match.into()
                  //                                ,

                  //                end:
                  //                                    end.into()


                  //         }

                  // }

                  // pub fn build_span_multi<A: Into<Box<Query>>>(

                  //        span_match: A
                  //    ) -> SpanMultiQuery {

                  //        SpanMultiQuery {

                  //                span_match:
                  //                                    span_match.into()


                  //         }

                  // }

                  // pub fn build_span_near<A: Into<Vec<Query>>,B: Into<i64>>(

                  //        clauses: A,

                  //        slop: B
                  //    ) -> SpanNearQuery {

                  //        SpanNearQuery {

                  //                clauses:
                  //                                    clauses.into()
                  //                                ,

                  //                slop:
                  //                                    slop.into()
                  //                                ,

                  //                in_order:
                  //                                    None
                  //                                ,

                  //                collect_payloads:
                  //                                    None


                  //         }

                  // }

                  // pub fn build_span_not<A: Into<Box<Query>>,B: Into<Box<Query>>>(

                  //        include: A,

                  //        exclude: B
                  //    ) -> SpanNotQuery {

                  //        SpanNotQuery {

                  //                include:
                  //                                    include.into()
                  //                                ,

                  //                exclude:
                  //                                    exclude.into()
                  //                                ,

                  //                pre:
                  //                                    None
                  //                                ,

                  //                post:
                  //                                    None
                  //                                ,

                  //                dist:
                  //                                    None


                  //         }

                  // }

                  // pub fn build_span_or<A: Into<Vec<Query>>>(

                  //        clauses: A
                  //    ) -> SpanOrQuery {

                  //        SpanOrQuery {

                  //                clauses:
                  //                                    clauses.into()


                  //         }

                  // }

                  pub fn build_span_term<A: Into<String>,B: Into<JsonVal>>(

                         field: A,

                         value: B
                     ) -> SpanTermQuery {

                         SpanTermQuery {

                                 field:
                                                     field.into()
                                                 ,

                                 value:
                                                     value.into()
                                                 ,

                                 boost:
                                                     None


                          }

                  }

          }


// Match queries








// Option structs for Query(ies)







        //   #[derive(Debug)]
        //   pub struct BoolQuery {

        //           must:
        //                                  Option<Vec<Query>>
        //                               ,

        //           must_not:
        //                                  Option<Vec<Query>>
        //                               ,

        //           should:
        //                                  Option<Vec<Query>>
        //                               ,

        //           minimum_should_match:
        //                                  Option<MinimumShouldMatch>
        //                               ,

        //           boost:
        //                                  Option<f64>


        //   }

        //   impl BoolQuery {

        //           pub fn with_must<T: Into<Vec<Query>>>(mut self, value: T) -> Self {
        //               self.must = Some(value.into());
        //               self
        //           }

        //           pub fn with_must_not<T: Into<Vec<Query>>>(mut self, value: T) -> Self {
        //               self.must_not = Some(value.into());
        //               self
        //           }

        //           pub fn with_should<T: Into<Vec<Query>>>(mut self, value: T) -> Self {
        //               self.should = Some(value.into());
        //               self
        //           }

        //           pub fn with_minimum_should_match<T: Into<MinimumShouldMatch>>(mut self, value: T) -> Self {
        //               self.minimum_should_match = Some(value.into());
        //               self
        //           }

        //           pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
        //               self.boost = Some(value.into());
        //               self
        //           }


        //       #[allow(dead_code, unused_variables)]
        //       fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //               // optional_add!(self, m, self.must, "must");

        //               // optional_add!(self, m, self.must_not, "must_not");

        //               // optional_add!(self, m, self.should, "should");

        //               // optional_add!(self, m, self.minimum_should_match, "minimum_should_match");

        //               // optional_add!(self, m, self.boost, "boost");

        //       }

        //       #[allow(dead_code, unused_variables)]
        //       fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //       }

        //       pub fn build(self) -> Query {
        //           Query::Bool(self)
        //       }
        //   }

        // impl ToJson for BoolQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


#[derive(Debug)]
pub enum Strategy {
    LeapFrogQueryFirst,
    LeapFrogFilterFirst,
    QueryFirst,
    RandomAccess(i64),
    RandomAccessAlways
}

from!(i64, Strategy, RandomAccess);

impl ToJson for Strategy {
    fn to_json(&self) -> Json {
        match self {
            &Strategy::LeapFrogQueryFirst  => "leap_frog_query_first".to_json(),
            &Strategy::LeapFrogFilterFirst => "leap_frog_filter_first".to_json(),
            &Strategy::QueryFirst          => "query_first".to_json(),
            &Strategy::RandomAccess(amt)   => format!("random_access_{}", amt).to_json(),
            &Strategy::RandomAccessAlways  => "random_access_always".to_json()
        }
    }
}

          #[derive(Debug)]
          pub struct FuzzyLikeThisQuery {

                  fields:
                                         Option<Vec<String>>
                                      ,

                  like_text:
                                         String
                                      ,

                  ignore_tf:
                                         Option<bool>
                                      ,

                  max_query_terms:
                                         Option<u64>
                                      ,

                  fuzziness:
                                         Option<Fuzziness>
                                      ,

                  prefix_length:
                                         Option<u64>
                                      ,

                  boost:
                                         Option<f64>
                                      ,

                  analyzer:
                                         Option<String>


          }

          // impl FuzzyLikeThisQuery {

          //         pub fn with_fields<T: Into<Vec<String>>>(mut self, value: T) -> Self {
          //             self.fields = Some(value.into());
          //             self
          //         }

          //         pub fn with_ignore_tf<T: Into<bool>>(mut self, value: T) -> Self {
          //             self.ignore_tf = Some(value.into());
          //             self
          //         }

          //         pub fn with_max_query_terms<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.max_query_terms = Some(value.into());
          //             self
          //         }

          //         pub fn with_fuzziness<T: Into<Fuzziness>>(mut self, value: T) -> Self {
          //             self.fuzziness = Some(value.into());
          //             self
          //         }

          //         pub fn with_prefix_length<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.prefix_length = Some(value.into());
          //             self
          //         }

          //         pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost = Some(value.into());
          //             self
          //         }

          //         pub fn with_analyzer<T: Into<String>>(mut self, value: T) -> Self {
          //             self.analyzer = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.fields, "fields");

          //             // optional_add!(self, m, self.ignore_tf, "ignore_tf");

          //             // optional_add!(self, m, self.max_query_terms, "max_query_terms");

          //             // optional_add!(self, m, self.fuzziness, "fuzziness");

          //             // optional_add!(self, m, self.prefix_length, "prefix_length");

          //             // optional_add!(self, m, self.boost, "boost");

          //             // optional_add!(self, m, self.analyzer, "analyzer");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::FuzzyLikeThis(self)
          //     }
          // }

        // impl ToJson for FuzzyLikeThisQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("like_text".to_owned(),
        //                    self.like_text.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          #[derive(Debug)]
          pub struct FuzzyLikeThisFieldQuery {

                  field:
                                         String
                                      ,

                  like_text:
                                         String
                                      ,

                  ignore_tf:
                                         Option<bool>
                                      ,

                  max_query_terms:
                                         Option<u64>
                                      ,

                  fuzziness:
                                         Option<Fuzziness>
                                      ,

                  prefix_length:
                                         Option<u64>
                                      ,

                  boost:
                                         Option<f64>
                                      ,

                  analyzer:
                                         Option<String>


          }

          // impl FuzzyLikeThisFieldQuery {

          //         pub fn with_ignore_tf<T: Into<bool>>(mut self, value: T) -> Self {
          //             self.ignore_tf = Some(value.into());
          //             self
          //         }

          //         pub fn with_max_query_terms<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.max_query_terms = Some(value.into());
          //             self
          //         }

          //         pub fn with_fuzziness<T: Into<Fuzziness>>(mut self, value: T) -> Self {
          //             self.fuzziness = Some(value.into());
          //             self
          //         }

          //         pub fn with_prefix_length<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.prefix_length = Some(value.into());
          //             self
          //         }

          //         pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost = Some(value.into());
          //             self
          //         }

          //         pub fn with_analyzer<T: Into<String>>(mut self, value: T) -> Self {
          //             self.analyzer = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.ignore_tf, "ignore_tf");

          //             // optional_add!(self, m, self.max_query_terms, "max_query_terms");

          //             // optional_add!(self, m, self.fuzziness, "fuzziness");

          //             // optional_add!(self, m, self.prefix_length, "prefix_length");

          //             // optional_add!(self, m, self.boost, "boost");

          //             // optional_add!(self, m, self.analyzer, "analyzer");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::FuzzyLikeThisField(self)
          //     }
          // }

        // impl ToJson for FuzzyLikeThisFieldQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();
        //         let mut inner = BTreeMap::new();


        //           inner.insert("like_text".to_owned(),
        //                        self.like_text.to_json());

        //         self.add_optionals(&mut inner);
        //         d.insert(self.field.clone(), Json::Object(inner));
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


// A document can be provided as an example
#[derive(Debug)]
pub struct Doc {
    index:    String,
    doc_type: String,
    doc:      Option<Json>,
    id:       Option<String>
}

impl Doc {
    pub fn from_doc<A, B>(index: A, doc_type: B, doc: Json) -> Doc
        where A: Into<String>, B: Into<String>
    {
        Doc {
            index:    index.into(),
            doc_type: doc_type.into(),
            doc:      Some(doc),
            id:       None
        }
    }

    pub fn id<A, B, C>(index: A, doc_type: B, id: C) -> Doc
        where A: Into<String>, B: Into<String>, C: Into<String>
    {
        Doc {
            index:    index.into(),
            doc_type: doc_type.into(),
            doc:      None,
            id:       Some(id.into())
        }
    }
}

impl ToJson for Doc {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("_index".to_owned(), self.index.to_json());
        d.insert("_type".to_owned(), self.doc_type.to_json());

        // optional_add!(self, d, self.doc, "doc");
        // optional_add!(self, d, self.id, "_id");

        Json::Object(d)
    }
}

          #[derive(Debug)]
          pub struct MoreLikeThisQuery {

                  fields:
                                         Option<Vec<String>>
                                      ,

                  like_text:
                                         Option<String>
                                      ,

                  ids:
                                         Option<Vec<String>>
                                      ,

                  docs:
                                         Option<Vec<Doc>>
                                      ,

                  max_query_terms:
                                         Option<u64>
                                      ,

                  min_term_freq:
                                         Option<u64>
                                      ,

                  min_doc_freq:
                                         Option<u64>
                                      ,

                  max_doc_freq:
                                         Option<u64>
                                      ,

                  min_word_length:
                                         Option<u64>
                                      ,

                  max_word_length:
                                         Option<u64>
                                      ,

                  stop_words:
                                         Option<Vec<String>>
                                      ,

                  analyzer:
                                         Option<String>
                                      ,

                  minimum_should_match:
                                         Option<MinimumShouldMatch>
                                      ,

                  boost_terms:
                                         Option<f64>
                                      ,

                  include:
                                         Option<bool>
                                      ,

                  boost:
                                         Option<f64>


          }

          // impl MoreLikeThisQuery {

          //         pub fn with_fields<T: Into<Vec<String>>>(mut self, value: T) -> Self {
          //             self.fields = Some(value.into());
          //             self
          //         }

          //         pub fn with_like_text<T: Into<String>>(mut self, value: T) -> Self {
          //             self.like_text = Some(value.into());
          //             self
          //         }

          //         pub fn with_ids<T: Into<Vec<String>>>(mut self, value: T) -> Self {
          //             self.ids = Some(value.into());
          //             self
          //         }

          //         pub fn with_docs<T: Into<Vec<Doc>>>(mut self, value: T) -> Self {
          //             self.docs = Some(value.into());
          //             self
          //         }

          //         pub fn with_max_query_terms<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.max_query_terms = Some(value.into());
          //             self
          //         }

          //         pub fn with_min_term_freq<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.min_term_freq = Some(value.into());
          //             self
          //         }

          //         pub fn with_min_doc_freq<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.min_doc_freq = Some(value.into());
          //             self
          //         }

          //         pub fn with_max_doc_freq<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.max_doc_freq = Some(value.into());
          //             self
          //         }

          //         pub fn with_min_word_length<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.min_word_length = Some(value.into());
          //             self
          //         }

          //         pub fn with_max_word_length<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.max_word_length = Some(value.into());
          //             self
          //         }

          //         pub fn with_stop_words<T: Into<Vec<String>>>(mut self, value: T) -> Self {
          //             self.stop_words = Some(value.into());
          //             self
          //         }

          //         pub fn with_analyzer<T: Into<String>>(mut self, value: T) -> Self {
          //             self.analyzer = Some(value.into());
          //             self
          //         }

          //         pub fn with_minimum_should_match<T: Into<MinimumShouldMatch>>(mut self, value: T) -> Self {
          //             self.minimum_should_match = Some(value.into());
          //             self
          //         }

          //         pub fn with_boost_terms<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost_terms = Some(value.into());
          //             self
          //         }

          //         pub fn with_include<T: Into<bool>>(mut self, value: T) -> Self {
          //             self.include = Some(value.into());
          //             self
          //         }

          //         pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.fields, "fields");

          //             // optional_add!(self, m, self.like_text, "like_text");

          //             // optional_add!(self, m, self.ids, "ids");

          //             // optional_add!(self, m, self.docs, "docs");

          //             // optional_add!(self, m, self.max_query_terms, "max_query_terms");

          //             // optional_add!(self, m, self.min_term_freq, "min_term_freq");

          //             // optional_add!(self, m, self.min_doc_freq, "min_doc_freq");

          //             // optional_add!(self, m, self.max_doc_freq, "max_doc_freq");

          //             // optional_add!(self, m, self.min_word_length, "min_word_length");

          //             // optional_add!(self, m, self.max_word_length, "max_word_length");

          //             // optional_add!(self, m, self.stop_words, "stop_words");

          //             // optional_add!(self, m, self.analyzer, "analyzer");

          //             // optional_add!(self, m, self.minimum_should_match, "minimum_should_match");

          //             // optional_add!(self, m, self.boost_terms, "boost_terms");

          //             // optional_add!(self, m, self.include, "include");

          //             // optional_add!(self, m, self.boost, "boost");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::MoreLikeThis(self)
          //     }
          // }

        // impl ToJson for MoreLikeThisQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }

        //   #[derive(Debug)]
        //   pub struct SpanFirstQuery {

        //           span_match:
        //                                  Box<Query>
        //                               ,

        //           end:
        //                                  i64


        //   }

        //   impl SpanFirstQuery {


        //       #[allow(dead_code, unused_variables)]
        //       fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //       }

        //       #[allow(dead_code, unused_variables)]
        //       fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //       }

        //       pub fn build(self) -> Query {
        //           Query::SpanFirst(self)
        //       }
        //   }

        // impl ToJson for SpanFirstQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("match".to_owned(),
        //                    self.span_match.to_json());

        //           d.insert("end".to_owned(),
        //                    self.end.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


        //   #[derive(Debug)]
        //   pub struct SpanMultiQuery {

        //           span_match:
        //                                  Box<Query>


        //   }

        //   impl SpanMultiQuery {


        //       #[allow(dead_code, unused_variables)]
        //       fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //       }

        //       #[allow(dead_code, unused_variables)]
        //       fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //       }

        //       pub fn build(self) -> Query {
        //           Query::SpanMulti(self)
        //       }
        //   }

        // impl ToJson for SpanMultiQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("match".to_owned(),
        //                    self.span_match.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          // #[derive(Debug)]
          // pub struct SpanNearQuery {

          //         clauses:
          //                                Vec<Query>
          //                             ,

          //         slop:
          //                                i64
          //                             ,

          //         in_order:
          //                                Option<bool>
          //                             ,

          //         collect_payloads:
          //                                Option<bool>


          // }

          // impl SpanNearQuery {

          //         pub fn with_in_order<T: Into<bool>>(mut self, value: T) -> Self {
          //             self.in_order = Some(value.into());
          //             self
          //         }

          //         pub fn with_collect_payloads<T: Into<bool>>(mut self, value: T) -> Self {
          //             self.collect_payloads = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.in_order, "in_order");

          //             // optional_add!(self, m, self.collect_payloads, "collect_payloads");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::SpanNear(self)
          //     }
          // }

        // impl ToJson for SpanNearQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("clauses".to_owned(),
        //                    self.clauses.to_json());

        //           d.insert("slop".to_owned(),
        //                    self.slop.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          // #[derive(Debug)]
          // pub struct SpanNotQuery {

          //         include:
          //                                Box<Query>
          //                             ,

          //         exclude:               Box<Query>
          //                             ,

          //         pre:
          //                                Option<i64>
          //                             ,

          //         post:
          //                                Option<i64>
          //                             ,

          //         dist:
          //                                Option<i64>


          // }

          // impl SpanNotQuery {

          //         pub fn with_pre<T: Into<i64>>(mut self, value: T) -> Self {
          //             self.pre = Some(value.into());
          //             self
          //         }

          //         pub fn with_post<T: Into<i64>>(mut self, value: T) -> Self {
          //             self.post = Some(value.into());
          //             self
          //         }

          //         pub fn with_dist<T: Into<i64>>(mut self, value: T) -> Self {
          //             self.dist = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.pre, "pre");

          //             // optional_add!(self, m, self.post, "post");

          //             // optional_add!(self, m, self.dist, "dist");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::SpanNot(self)
          //     }
          // }

        // impl ToJson for SpanNotQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("include".to_owned(),
        //                    self.include.to_json());

        //           d.insert("exclude".to_owned(),
        //                    self.exclude.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          #[derive(Debug)]
          pub struct SpanOrQuery {

                  clauses:
                                         Vec<Query>


          }

          // impl SpanOrQuery {


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::SpanOr(self)
          //     }
          // }

        // impl<'a> ToJson for SpanOrQuery<'a> {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("clauses".to_owned(),
        //                    self.clauses.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          #[derive(Debug)]
          pub struct SpanTermQuery {

                  field:
                                         String
                                      ,

                  value:
                                         JsonVal
                                      ,

                  boost:
                                         Option<f64>


          }

          // impl SpanTermQuery {

          //         pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //         //optional_add!(self, m, self.boost, "boost");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::SpanTerm(self)
          //     }
          // }

        // impl ToJson for SpanTermQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();
        //         let mut inner = BTreeMap::new();


        //           inner.insert("value".to_owned(),
        //                        self.value.to_json());

        //         self.add_optionals(&mut inner);
        //         d.insert(self.field.clone(), Json::Object(inner));
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }







// Filters
// TODO - determine which of these are required
//           #[derive(Debug)]
//           pub struct AndFilter {

//                   filters:
//                                          Option<Vec<Filter>>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl AndFilter {

//                   pub fn with_filters<T: Into<Vec<Filter>>>(mut self, value: T) -> Self {
//                       self.filters = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.filters, "filters");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::And(self)
//               }
//           }

//         impl ToJson for AndFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct BoolFilter {

//                   must:
//                                          Option<Vec<Filter>>
//                                       ,

//                   must_not:
//                                          Option<Vec<Filter>>
//                                       ,

//                   should:
//                                          Option<Vec<Filter>>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl BoolFilter {

//                   pub fn with_must<T: Into<Vec<Filter>>>(mut self, value: T) -> Self {
//                       self.must = Some(value.into());
//                       self
//                   }

//                   pub fn with_must_not<T: Into<Vec<Filter>>>(mut self, value: T) -> Self {
//                       self.must_not = Some(value.into());
//                       self
//                   }

//                   pub fn with_should<T: Into<Vec<Filter>>>(mut self, value: T) -> Self {
//                       self.should = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.must, "must");

//                       optional_add!(self, m, self.must_not, "must_not");

//                       optional_add!(self, m, self.should, "should");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Bool(self)
//               }
//           }

//         impl ToJson for BoolFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct ExistsFilter {

//                   field:
//                                          String
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl ExistsFilter {

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Exists(self)
//               }
//           }

//         impl ToJson for ExistsFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                   d.insert("field".to_owned(),
//                            self.field.to_json());

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }




//           impl GeoDistanceFilter {

//                   pub fn with_distance_type<T: Into<DistanceType>>(mut self, value: T) -> Self {
//                       self.distance_type = Some(value.into());
//                       self
//                   }

//                   pub fn with_optimize_bbox<T: Into<OptimizeBbox>>(mut self, value: T) -> Self {
//                       self.optimize_bbox = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.distance_type, "distance_type");

//                       optional_add!(self, m, self.optimize_bbox, "optimize_bbox");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::GeoDistance(self)
//               }
//           }





// impl ToJson for GeoDistanceFilter {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert(self.field.clone(), self.location.to_json());
//         d.insert("distance".to_owned(), self.distance.to_json());
//         self.add_optionals(&mut d);
//         self.add_core_optionals(&mut d);
//         Json::Object(d)
//     }
// }

//           #[derive(Debug)]
//           pub struct GeoPolygonFilter {

//                   field:
//                                          String
//                                       ,

//                   points:
//                                          Vec<Location>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl GeoPolygonFilter {

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::GeoPolygon(self)
//               }
//           }

//         impl ToJson for GeoPolygonFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();
//                 let mut inner = BTreeMap::new();


//                   inner.insert("points".to_owned(),
//                                self.points.to_json());

//                 self.add_optionals(&mut inner);
//                 d.insert(self.field.clone(), Json::Object(inner));
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct GeoShapeFilter {

//                   field:
//                                          String
//                                       ,

//                   shape:
//                                          Option<Shape>
//                                       ,

//                   indexed_shape:
//                                          Option<IndexedShape>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl GeoShapeFilter {

//                   pub fn with_shape<T: Into<Shape>>(mut self, value: T) -> Self {
//                       self.shape = Some(value.into());
//                       self
//                   }

//                   pub fn with_indexed_shape<T: Into<IndexedShape>>(mut self, value: T) -> Self {
//                       self.indexed_shape = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.shape, "shape");

//                       optional_add!(self, m, self.indexed_shape, "indexed_shape");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::GeoShape(self)
//               }
//           }

//         impl ToJson for GeoShapeFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();
//                 let mut inner = BTreeMap::new();


//                 self.add_optionals(&mut inner);
//                 d.insert(self.field.clone(), Json::Object(inner));
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct GeohashCellFilter {

//                   field:
//                                          String
//                                       ,

//                   location:
//                                          Location
//                                       ,

//                   precision:
//                                          Option<Precision>
//                                       ,

//                   neighbors:
//                                          Option<bool>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl GeohashCellFilter {

//                   pub fn with_precision<T: Into<Precision>>(mut self, value: T) -> Self {
//                       self.precision = Some(value.into());
//                       self
//                   }

//                   pub fn with_neighbors<T: Into<bool>>(mut self, value: T) -> Self {
//                       self.neighbors = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.precision, "precision");

//                       optional_add!(self, m, self.neighbors, "neighbors");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::GeohashCell(self)
//               }
//           }


// #[derive(Debug)]
// pub enum Precision {
//     Geohash(u64),
//     Distance(Distance)
// }

// from!(u64, Precision, Geohash);
// from!(Distance, Precision, Distance);

// impl ToJson for Precision {
//     fn to_json(&self) -> Json {
//         match self {
//             &Precision::Geohash(geohash_precision) => Json::U64(geohash_precision),
//             &Precision::Distance(ref distance)     => distance.to_json()
//         }
//     }
// }

// impl ToJson for GeohashCellFilter {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert(self.field.clone(), self.location.to_json());
//         self.add_optionals(&mut d);
//         self.add_core_optionals(&mut d);
//         Json::Object(d)
//     }
// }

//           #[derive(Debug)]
//           pub struct HasChildFilter {

//                   doc_type:
//                                          String
//                                       ,

//                   query:
//                                          Option<Box<Query>>
//                                       ,

//                   filter:
//                                          Option<Box<Filter>>
//                                       ,

//                   min_children:
//                                          Option<u64>
//                                       ,

//                   max_children:
//                                          Option<u64>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl HasChildFilter {

//                   pub fn with_query<T: Into<Box<Query>>>(mut self, value: T) -> Self {
//                       self.query = Some(value.into());
//                       self
//                   }

//                   pub fn with_filter<T: Into<Box<Filter>>>(mut self, value: T) -> Self {
//                       self.filter = Some(value.into());
//                       self
//                   }

//                   pub fn with_min_children<T: Into<u64>>(mut self, value: T) -> Self {
//                       self.min_children = Some(value.into());
//                       self
//                   }

//                   pub fn with_max_children<T: Into<u64>>(mut self, value: T) -> Self {
//                       self.max_children = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.query, "query");

//                       optional_add!(self, m, self.filter, "filter");

//                       optional_add!(self, m, self.min_children, "min_children");

//                       optional_add!(self, m, self.max_children, "max_children");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::HasChild(self)
//               }
//           }

//         impl ToJson for HasChildFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                   d.insert("type".to_owned(),
//                            self.doc_type.to_json());

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct HasParentFilter {

//                   parent_type:
//                                          String
//                                       ,

//                   query:
//                                          Option<Box<Query>>
//                                       ,

//                   filter:
//                                          Option<Box<Filter>>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl HasParentFilter {

//                   pub fn with_query<T: Into<Box<Query>>>(mut self, value: T) -> Self {
//                       self.query = Some(value.into());
//                       self
//                   }

//                   pub fn with_filter<T: Into<Box<Filter>>>(mut self, value: T) -> Self {
//                       self.filter = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.query, "query");

//                       optional_add!(self, m, self.filter, "filter");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::HasParent(self)
//               }
//           }

//         impl ToJson for HasParentFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                   d.insert("parent_type".to_owned(),
//                            self.parent_type.to_json());

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct IdsFilter {

//                   doc_type:
//                                          Option<OneOrMany<String>>
//                                       ,

//                   values:
//                                          Vec<String>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl IdsFilter {

//                   pub fn with_type<T: Into<OneOrMany<String>>>(mut self, value: T) -> Self {
//                       self.doc_type = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.doc_type, "type");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Ids(self)
//               }
//           }

//         impl ToJson for IdsFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                   d.insert("values".to_owned(),
//                            self.values.to_json());

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct IndicesFilter {

//                   index:
//                                          Option<String>
//                                       ,

//                   indices:
//                                          Option<Vec<String>>
//                                       ,

//                   filter:
//                                          Option<Box<Filter>>
//                                       ,

//                   no_match_filter:
//                                          Option<NoMatchFilter>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl IndicesFilter {

//                   pub fn with_index<T: Into<String>>(mut self, value: T) -> Self {
//                       self.index = Some(value.into());
//                       self
//                   }

//                   pub fn with_indices<T: Into<Vec<String>>>(mut self, value: T) -> Self {
//                       self.indices = Some(value.into());
//                       self
//                   }

//                   pub fn with_filter<T: Into<Box<Filter>>>(mut self, value: T) -> Self {
//                       self.filter = Some(value.into());
//                       self
//                   }

//                   pub fn with_no_match_filter<T: Into<NoMatchFilter>>(mut self, value: T) -> Self {
//                       self.no_match_filter = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.index, "index");

//                       optional_add!(self, m, self.indices, "indices");

//                       optional_add!(self, m, self.filter, "filter");

//                       optional_add!(self, m, self.no_match_filter, "no_match_filter");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Indices(self)
//               }
//           }


//         impl ToJson for IndicesFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct MatchAllFilter {

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl MatchAllFilter {

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::MatchAll(self)
//               }
//           }

//         impl ToJson for MatchAllFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct NestedFilter {

//                   path:
//                                          String
//                                       ,

//                   filter:
//                                          Box<Filter>
//                                       ,

//                   score_mode:
//                                          Option<ScoreMode>
//                                       ,

//                   join:
//                                          Option<bool>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl NestedFilter {

//                   pub fn with_score_mode<T: Into<ScoreMode>>(mut self, value: T) -> Self {
//                       self.score_mode = Some(value.into());
//                       self
//                   }

//                   pub fn with_join<T: Into<bool>>(mut self, value: T) -> Self {
//                       self.join = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.score_mode, "score_mode");

//                       optional_add!(self, m, self.join, "join");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Nested(self)
//               }
//           }

//         impl ToJson for NestedFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                   d.insert("path".to_owned(),
//                            self.path.to_json());

//                   d.insert("filter".to_owned(),
//                            self.filter.to_json());

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct NotFilter {

//                   filter:
//                                          Box<Filter>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl NotFilter {

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Not(self)
//               }
//           }

//         impl ToJson for NotFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                   d.insert("filter".to_owned(),
//                            self.filter.to_json());

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct OrFilter {

//                   filters:
//                                          Vec<Filter>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl OrFilter {

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Or(self)
//               }
//           }

//         impl ToJson for OrFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                   d.insert("filters".to_owned(),
//                            self.filters.to_json());

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct PrefixFilter {

//                   field:
//                                          String
//                                       ,

//                   value:
//                                          String
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl PrefixFilter {

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Prefix(self)
//               }
//           }


// impl ToJson for PrefixFilter {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert(self.field.clone(), self.value.to_json());
//         self.add_optionals(&mut d);
//         self.add_core_optionals(&mut d);
//         Json::Object(d)
//     }
// }

//           #[derive(Debug)]
//           pub struct QueryFilter {

//                   query:
//                                          Box<Query>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl QueryFilter {

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Query(self)
//               }
//           }

//         impl ToJson for QueryFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                   d.insert("query".to_owned(),
//                            self.query.to_json());

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct RangeFilter {

//                   field:
//                                          String
//                                       ,

//                   gte:
//                                          Option<JsonVal>
//                                       ,

//                   gt:
//                                          Option<JsonVal>
//                                       ,

//                   lte:
//                                          Option<JsonVal>
//                                       ,

//                   lt:
//                                          Option<JsonVal>
//                                       ,

//                   boost:
//                                          Option<f64>
//                                       ,

//                   time_zone:
//                                          Option<String>
//                                       ,

//                   format:
//                                          Option<String>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl RangeFilter {

//                   pub fn with_gte<T: Into<JsonVal>>(mut self, value: T) -> Self {
//                       self.gte = Some(value.into());
//                       self
//                   }

//                   pub fn with_gt<T: Into<JsonVal>>(mut self, value: T) -> Self {
//                       self.gt = Some(value.into());
//                       self
//                   }

//                   pub fn with_lte<T: Into<JsonVal>>(mut self, value: T) -> Self {
//                       self.lte = Some(value.into());
//                       self
//                   }

//                   pub fn with_lt<T: Into<JsonVal>>(mut self, value: T) -> Self {
//                       self.lt = Some(value.into());
//                       self
//                   }

//                   pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
//                       self.boost = Some(value.into());
//                       self
//                   }

//                   pub fn with_time_zone<T: Into<String>>(mut self, value: T) -> Self {
//                       self.time_zone = Some(value.into());
//                       self
//                   }

//                   pub fn with_format<T: Into<String>>(mut self, value: T) -> Self {
//                       self.format = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.gte, "gte");

//                       optional_add!(self, m, self.gt, "gt");

//                       optional_add!(self, m, self.lte, "lte");

//                       optional_add!(self, m, self.lt, "lt");

//                       optional_add!(self, m, self.boost, "boost");

//                       optional_add!(self, m, self.time_zone, "time_zone");

//                       optional_add!(self, m, self.format, "format");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Range(self)
//               }
//           }

//         impl ToJson for RangeFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();
//                 let mut inner = BTreeMap::new();


//                 self.add_optionals(&mut inner);
//                 d.insert(self.field.clone(), Json::Object(inner));
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct RegexpFilter {

//                   field:
//                                          String
//                                       ,

//                   value:
//                                          String
//                                       ,

//                   boost:
//                                          Option<f64>
//                                       ,

//                   flags:
//                                          Option<Flags>
//                                       ,

//                   max_determined_states:
//                                          Option<u64>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl RegexpFilter {

//                   pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
//                       self.boost = Some(value.into());
//                       self
//                   }

//                   pub fn with_flags<T: Into<Flags>>(mut self, value: T) -> Self {
//                       self.flags = Some(value.into());
//                       self
//                   }

//                   pub fn with_max_determined_states<T: Into<u64>>(mut self, value: T) -> Self {
//                       self.max_determined_states = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.boost, "boost");

//                       optional_add!(self, m, self.flags, "flags");

//                       optional_add!(self, m, self.max_determined_states, "max_determined_states");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Regexp(self)
//               }
//           }

//         impl ToJson for RegexpFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();
//                 let mut inner = BTreeMap::new();


//                   inner.insert("value".to_owned(),
//                                self.value.to_json());

//                 self.add_optionals(&mut inner);
//                 d.insert(self.field.clone(), Json::Object(inner));
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct ScriptFilter {

//                   script:
//                                          String
//                                       ,

//                   params:
//                                          Option<BTreeMap<String, JsonVal>>
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl ScriptFilter {

//                   pub fn with_params<T: Into<BTreeMap<String, JsonVal>>>(mut self, value: T) -> Self {
//                       self.params = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.params, "params");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Script(self)
//               }
//           }

//         impl ToJson for ScriptFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                   d.insert("script".to_owned(),
//                            self.script.to_json());

//                 self.add_optionals(&mut d);
//                 self.add_core_optionals(&mut d);
//                 Json::Object(d)
//             }
//         }


//           #[derive(Debug)]
//           pub struct TermFilter {

//                   field:
//                                          String
//                                       ,

//                   value:
//                                          JsonVal
//                                       ,

//                   _cache:
//                                          Option<bool>
//                                       ,

//                   _cache_key:
//                                          Option<String>
//                                       ,

//                   _name:
//                                          Option<String>


//           }

//           impl TermFilter {

//                   pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
//                       self._cache = Some(value.into());
//                       self
//                   }

//                   pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
//                       self._cache_key = Some(value.into());
//                       self
//                   }

//                   pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
//                       self._name = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Term(self)
//               }
//           }


// impl ToJson for TermFilter {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert(self.field.clone(), self.value.to_json());
//         self.add_optionals(&mut d);
//         self.add_core_optionals(&mut d);
//         Json::Object(d)
//     }
// }

// TODO: determine if required or not
          // #[derive(Debug)]
          // pub struct TermsFilter {

          //         field:
          //                                String
          //                             ,

          //         values:
          //                                Vec<JsonVal>
          //                             ,

          //         execution:
          //                                Option<Execution>
          //                             ,

          //         _cache:
          //                                Option<bool>
          //                             ,

          //         _cache_key:
          //                                Option<String>
          //                             ,

          //         _name:
          //                                Option<String>


          // }

          // impl TermsFilter {

          //         pub fn with_execution<T: Into<Execution>>(mut self, value: T) -> Self {
          //             self.execution = Some(value.into());
          //             self
          //         }

          //         pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
          //             self._cache = Some(value.into());
          //             self
          //         }

          //         pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
          //             self._cache_key = Some(value.into());
          //             self
          //         }

          //         pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
          //             self._name = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             optional_add!(self, m, self.execution, "execution");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             optional_add!(self, m, self._cache, "_cache");

          //             optional_add!(self, m, self._cache_key, "_cache_key");

          //             optional_add!(self, m, self._name, "_name");

          //     }

          //     pub fn build(self) -> Filter {
          //         Filter::Terms(self)
          //     }
          // }


        #[derive(Debug)]
        pub enum Execution {

                Plain
                ,

                Fielddata
                ,

                Bool
                ,

                BoolNocache
                ,

                And
                ,

                AndNocache
                ,

                Or
                ,

                OrNocache


        }

        impl ToJson for Execution {
            fn to_json(&self) -> Json {
                match self {

                        &Execution::Plain
                        => "plain".to_json()
                        ,

                        &Execution::Fielddata
                        => "fielddata".to_json()
                        ,

                        &Execution::Bool
                        => "bool".to_json()
                        ,

                        &Execution::BoolNocache
                        => "bool_nocache".to_json()
                        ,

                        &Execution::And
                        => "and".to_json()
                        ,

                        &Execution::AndNocache
                        => "and_nocache".to_json()
                        ,

                        &Execution::Or
                        => "or".to_json()
                        ,

                        &Execution::OrNocache
                        => "or_nocache".to_json()


                }
            }
        }

#[cfg(test)]
mod tests {
    extern crate rustc_serialize;

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
                   terms_query.to_json().to_string());

        let terms_query_2 = Query::build_terms("field_name")
            .with_values(["a", "b", "c"].as_ref())
            .build();
        assert_eq!("{\"terms\":{\"field_name\":[\"a\",\"b\",\"c\"]}}",
                   terms_query_2.to_json().to_string());

        let terms_query_3 = Query::build_terms("field_name")
            .with_values(TermsQueryLookup::new(123, "blah.de.blah").with_index("other"))
            .build();
        assert_eq!("{\"terms\":{\"field_name\":{\"id\":123,\"index\":\"other\",\"path\":\"blah.de.blah\"}}}",
                   terms_query_3.to_json().to_string());
    }

    #[test]
    fn test_function_score_query() {
        let function_score_query = Query::build_function_score()
            .with_function(Function::build_script_score("this_is_a_script")
                           .with_lang("made_up")
                           .add_param("A", 12)
                           .build())
            .build();
        assert_eq!("{\"function_score\":{\"functions\":[{\"script_score\":{\"inline\":\"this_is_a_script\",\"lang\":\"made_up\",\"params\":{\"A\":12}}}]}}",
                   function_score_query.to_json().to_string());
    }
}
