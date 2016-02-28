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

use std::collections::{BTreeMap, HashMap};

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

// Helper macros

/// This package is full of builder interfaces, with much repeated code for adding
/// optional fields.  This macro removes much of the repetition.
macro_rules! add_option {
    ($n:ident, $e:ident, $t:ty) => (
        pub fn $n<T: Into<$t>>(mut self, val: T) -> Self {
            self.$e = Some(val.into());
            self
        }
    )
}

/// Build the `build` function for each builder struct
macro_rules! build {
    ($t:ident) => (
        pub fn build(self) -> Query {
            Query::$t(Box::new(self))
        }
    )
}

// Miscellaneous types required by queries go here

// Enums

/// MatchType - the type of Match query
#[derive(Debug)]
pub enum MatchType {
    Boolean,
    Phrase,
    PhrasePrefix
}

impl ToJson for MatchType {
    fn to_json(&self) -> Json {
        match self {
            &MatchType::Boolean => "boolean",
            &MatchType::Phrase => "phrase",
            &MatchType::PhrasePrefix => "phrase_prefix"
        }.to_json()
    }
}

/// MatchQueryType - the type of the multi Match Query
#[derive(Debug)]
pub enum MatchQueryType {
    BestFields,
    MostFields,
    CrossFields,
    Phrase,
    PhrasePrefix,
}

impl ToJson for MatchQueryType {
    fn to_json(&self) -> Json {
        match self {
            &MatchQueryType::BestFields => "best_fields",
            &MatchQueryType::MostFields => "most_fields",
            &MatchQueryType::CrossFields => "cross_fields",
            &MatchQueryType::Phrase => "phrase",
            &MatchQueryType::PhrasePrefix => "phrase_prefix",
        }.to_json()
    }
}

/// Minimum should match - used in numerous queries

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

/// Zero Terms Query

#[derive(Debug)]
pub enum ZeroTermsQuery {
    None,
    All
}

impl ToJson for ZeroTermsQuery {
    fn to_json(&self) -> Json {
        match self {
            &ZeroTermsQuery::None => "none",
            &ZeroTermsQuery::All => "all"
        }.to_json()
    }
}

// Functions

/// Function
#[derive(Debug)]
pub struct Function {
    // TODO : implement specific fields
    //     filter: Option<Filter>,
    //     function: Func,
    weight: Option<f64>
}

impl ToJson for Function {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        // TODO - implement body
//         optional_add!(self, d, filter);
//         optional_add!(self, d, weight);
//         d.insert(self.function.name(), self.function.to_json());
         Json::Object(d)
    }
}

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

/// Query represents all available queries
///
/// Each value is boxed as Queries can be recursive, they also vary
/// significantly in size

// TODO: Filters and Queries are merged, ensure all filters are included in this enum
#[derive(Debug)]
pub enum Query {
    MatchAll(Box<MatchAllQuery>),

    // Full-text queries
    Match(Box<MatchQuery>),
    MultiMatch(Box<MultiMatchQuery>),
    Common(Box<CommonQuery>),
    QueryString(Box<QueryStringQuery>),
    SimpleQueryString(Box<SimpleQueryStringQuery>),

    // Term level queries
    Term(Box<TermQuery>),
    Terms(Box<TermsQuery>),
    Range(Box<RangeQuery>),
    Exists(Box<ExistsQuery>),

    // TODO: below this line, not yet converted
//    Bool(BoolQuery),
//    Boosting(BoostingQuery),
//    ConstantScore(ConstantScoreQuery),
//    DisMax(DisMaxQuery),
//    FuzzyLikeThis(FuzzyLikeThisQuery),
//    FuzzyLikeThisField(FuzzyLikeThisFieldQuery),
//    FunctionScore(FunctionScoreQuery),
//    Fuzzy(FuzzyQuery),
//    GeoShape(GeoShapeQuery),
//    HasChild(HasChildQuery),
//    HasParent(HasParentQuery),
//    Ids(IdsQuery),
//    Indices(IndicesQuery),
//    MoreLikeThis(MoreLikeThisQuery),
//    Nested(NestedQuery),
//    Prefix(PrefixQuery),

//    Regexp(RegexpQuery),
//    SpanFirst(SpanFirstQuery),
//    SpanMulti(SpanMultiQuery),
//    SpanNear(SpanNearQuery),
//    SpanNot(SpanNotQuery),
//    SpanOr(SpanOrQuery<'a>),
//    SpanTerm(SpanTermQuery),

//    Wildcard(WildcardQuery)
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
            }
            &Query::Exists(ref q) => {
                d.insert("exists".to_owned(), q.to_json());
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

impl<'a> MatchAllQuery {
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

/// Match query

#[derive(Debug, Default)]
pub struct MatchQuery {
    field: String,
    query: JsonVal,
    match_type: Option<MatchType>,
    cutoff_frequency: Option<f64>,
    lenient: Option<bool>,
    analyzer: Option<String>,
    boost: Option<f64>,
    operator: Option<String>,
    minimum_should_match: Option<MinimumShouldMatch>,
    fuzziness: Option<Fuzziness>,
    prefix_length: Option<u64>,
    max_expansions: Option<u64>,
    rewrite: Option<String>,
    zero_terms_query: Option<ZeroTermsQuery>,
    slop: Option<i64>
}

impl Query {
    pub fn build_match<A: Into<String>, B: Into<JsonVal>>(field: A, query: B) -> MatchQuery {
        MatchQuery {
            field: field.into(),
            query: query.into(),
            ..Default::default()
        }
    }
}

impl MatchQuery {
    add_option!(with_type, match_type, MatchType);
    add_option!(with_cutoff_frequency, cutoff_frequency, f64);
    add_option!(with_lenient, lenient, bool);
    add_option!(with_analyzer, analyzer, String);
    add_option!(with_boost, boost, f64);
    add_option!(with_operator, operator, String);
    add_option!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_option!(with_fuzziness, fuzziness, Fuzziness);
    add_option!(with_prefix_length, prefix_length, u64);
    add_option!(with_max_expansions, max_expansions, u64);
    add_option!(with_rewrite, rewrite, String);
    add_option!(with_zero_terms_query, zero_terms_query, ZeroTermsQuery);
    add_option!(with_slop, slop, i64);

    build!(Match);
}

impl ToJson for MatchQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert("query".to_owned(), self.query.to_json());
        optional_add!(self, inner, match_type);
        optional_add!(self, inner, cutoff_frequency);
        optional_add!(self, inner, lenient);
        optional_add!(self, inner, analyzer);
        optional_add!(self, inner, boost);
        optional_add!(self, inner, operator);
        optional_add!(self, inner, minimum_should_match);
        optional_add!(self, inner, fuzziness);
        optional_add!(self, inner, prefix_length);
        optional_add!(self, inner, max_expansions);
        optional_add!(self, inner, rewrite);
        optional_add!(self, inner, zero_terms_query);
        optional_add!(self, inner, slop);
        d.insert(self.field.clone(), Json::Object(inner));
        Json::Object(d)
    }
}

/// Multi Match Query
#[derive(Debug, Default)]
pub struct MultiMatchQuery {
    fields: Vec<String>,
    query: JsonVal,
    match_type: Option<MatchQueryType>,
    tie_breaker: Option<f64>,
    analyzer: Option<String>,
    boost: Option<f64>,
    operator: Option<String>,
    minimum_should_match: Option<MinimumShouldMatch>,
    fuzziness: Option<Fuzziness>,
    prefix_length: Option<u64>,
    max_expansions: Option<u64>,
    rewrite: Option<String>,
    zero_terms_query: Option<ZeroTermsQuery>,
    cutoff_frequency: Option<f64>,
    slop: Option<i64>
}

impl Query {
    pub fn build_multi_match<A, B>(fields: A, query: B) -> MultiMatchQuery
        where A: Into<Vec<String>>,
              B: Into<JsonVal> {
        MultiMatchQuery {
            fields: fields.into(),
            query: query.into(),
            ..Default::default()
        }
    }
}

impl MultiMatchQuery {
    add_option!(with_type, match_type, MatchQueryType);
    add_option!(with_tie_breaker, tie_breaker, f64);
    add_option!(with_analyzer, analyzer, String);
    add_option!(with_boost, boost, f64);
    add_option!(with_operator, operator, String);
    add_option!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_option!(with_fuzziness, fuzziness, Fuzziness);
    add_option!(with_prefix_length, prefix_length, u64);
    add_option!(with_max_expansions, max_expansions, u64);
    add_option!(with_rewrite, rewrite, String);
    add_option!(with_zero_terms_query, zero_terms_query, ZeroTermsQuery);
    add_option!(with_cutoff_frequency, cutoff_frequency, f64);
    add_option!(with_slop, slop, i64);

    build!(MultiMatch);
}

impl ToJson for MultiMatchQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("fields".to_owned(), self.fields.to_json());
        d.insert("query".to_owned(), self.query.to_json());
        optional_add!(self, d, match_type);
        optional_add!(self, d, tie_breaker);
        optional_add!(self, d, analyzer);
        optional_add!(self, d, boost);
        optional_add!(self, d, operator);
        optional_add!(self, d, minimum_should_match);
        optional_add!(self, d, fuzziness);
        optional_add!(self, d, prefix_length);
        optional_add!(self, d, max_expansions);
        optional_add!(self, d, rewrite);
        optional_add!(self, d, zero_terms_query);
        optional_add!(self, d, cutoff_frequency);
        optional_add!(self, d, slop);
        Json::Object(d)
    }
}

/// Common terms query
#[derive(Debug, Default)]
pub struct CommonQuery {
    query: JsonVal,
    cutoff_frequency: Option<f64>,
    low_freq_operator: Option<String>,
    high_freq_operator: Option<String>,
    minimum_should_match: Option<MinimumShouldMatch>,
    boost: Option<f64>,
    analyzer: Option<String>,
    disable_coord: Option<bool>
}

impl Query {
    pub fn build_common<A>(query: A) -> CommonQuery
        where A: Into<JsonVal> {
        CommonQuery {
            query: query.into(),
            ..Default::default()
        }
    }
}

impl CommonQuery {
    add_option!(with_cutoff_frequency, cutoff_frequency, f64);
    add_option!(with_low_freq_operator, low_freq_operator, String);
    add_option!(with_high_freq_operator, high_freq_operator, String);
    add_option!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_option!(with_boost, boost, f64);
    add_option!(with_analyzer, analyzer, String);
    add_option!(with_disable_coord, disable_coord, bool);

    build!(Common);
}

impl ToJson for CommonQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert("query".to_owned(), self.query.to_json());
        optional_add!(self, inner, cutoff_frequency);
        optional_add!(self, inner, low_freq_operator);
        optional_add!(self, inner, high_freq_operator);
        optional_add!(self, inner, minimum_should_match);
        optional_add!(self, inner, boost);
        optional_add!(self, inner, analyzer);
        optional_add!(self, inner, disable_coord);
        d.insert("body".to_owned(), Json::Object(inner));
        Json::Object(d)
    }
}

/// Query string query
#[derive(Debug, Default)]
pub struct QueryStringQuery {
    query: String,
    default_field: Option<String>,
    fields: Option<Vec<String>>,
    default_operator: Option<String>,
    analyzer: Option<String>,
    allow_leading_wildcard: Option<bool>,
    lowercase_expanded_terms: Option<bool>,
    enable_position_increments: Option<bool>,
    fuzzy_max_expansions: Option<u64>,
    fuzziness: Option<Fuzziness>,
    fuzzy_prefix_length: Option<u64>,
    phrase_slop: Option<i64>,
    boost: Option<f64>,
    analyze_wildcard: Option<bool>,
    auto_generate_phrase_queries: Option<bool>,
    max_determined_states: Option<u64>,
    minimum_should_match: Option<MinimumShouldMatch>,
    lenient: Option<bool>,
    locale: Option<String>,
    time_zone: Option<String>,
    use_dis_max: Option<bool>
}

impl Query {
    pub fn build_query_string<A: Into<String>>(query: A) -> QueryStringQuery {
        QueryStringQuery {
            query: query.into(),
            ..Default::default()
        }
    }
}

impl QueryStringQuery {
    add_option!(with_default_field, default_field, String);
    add_option!(with_fields, fields, Vec<String>);
    add_option!(with_default_operator, default_operator, String);
    add_option!(with_analyzer, analyzer, String);
    add_option!(with_allow_leading_wildcard, allow_leading_wildcard, bool);
    add_option!(with_lowercase_expanded_terms, lowercase_expanded_terms, bool);
    add_option!(with_enable_position_increments, enable_position_increments, bool);
    add_option!(with_fuzzy_max_expansions, fuzzy_max_expansions, u64);
    add_option!(with_fuzziness, fuzziness, Fuzziness);
    add_option!(with_fuzzy_prefix_length, fuzzy_prefix_length, u64);
    add_option!(with_phrase_slop, phrase_slop, i64);
    add_option!(with_boost, boost, f64);
    add_option!(with_analyze_wildcard, analyze_wildcard, bool);
    add_option!(with_auto_generate_phrase_queries, auto_generate_phrase_queries, bool);
    add_option!(with_max_determined_states, max_determined_states, u64);
    add_option!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_option!(with_lenient, lenient, bool);
    add_option!(with_locale, locale, String);
    add_option!(with_time_zone, time_zone, String);
    add_option!(with_use_dis_max, use_dis_max, bool);

    build!(QueryString);
}

impl ToJson for QueryStringQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("query".to_owned(), self.query.to_json());
        optional_add!(self, d, default_field);
        optional_add!(self, d, fields);
        optional_add!(self, d, default_operator);
        optional_add!(self, d, analyzer);
        optional_add!(self, d, allow_leading_wildcard);
        optional_add!(self, d, lowercase_expanded_terms);
        optional_add!(self, d, enable_position_increments);
        optional_add!(self, d, fuzzy_max_expansions);
        optional_add!(self, d, fuzziness);
        optional_add!(self, d, fuzzy_prefix_length);
        optional_add!(self, d, phrase_slop);
        optional_add!(self, d, boost);
        optional_add!(self, d, analyze_wildcard);
        optional_add!(self, d, auto_generate_phrase_queries);
        optional_add!(self, d, max_determined_states);
        optional_add!(self, d, minimum_should_match);
        optional_add!(self, d, lenient);
        optional_add!(self, d, locale);
        optional_add!(self, d, time_zone);
        optional_add!(self, d, use_dis_max);
        Json::Object(d)
    }
}

/// Flags for the SimpleQueryString query
#[derive(Debug)]
pub enum SimpleQueryStringFlags {
    All,
    None,
    And,
    Or,
    Not,
    Prefix,
    Phrase,
    Precedence,
    Escape,
    Whitespace,
    Fuzzy,
    Near,
    Slop
}

impl AsRef<str> for SimpleQueryStringFlags {
    fn as_ref(&self) -> &str {
        match self {
            &SimpleQueryStringFlags::All => "ALL",
            &SimpleQueryStringFlags::None => "NONE",
            &SimpleQueryStringFlags::And => "AND",
            &SimpleQueryStringFlags::Or => "OR",
            &SimpleQueryStringFlags::Not => "NOT",
            &SimpleQueryStringFlags::Prefix => "PREFIX",
            &SimpleQueryStringFlags::Phrase => "PHRASE",
            &SimpleQueryStringFlags::Precedence => "PRECEDENCE",
            &SimpleQueryStringFlags::Escape => "ESCAPE",
            &SimpleQueryStringFlags::Whitespace => "WHITESPACE",
            &SimpleQueryStringFlags::Fuzzy => "FUZZY",
            &SimpleQueryStringFlags::Near => "NEAR",
            &SimpleQueryStringFlags::Slop => "SLOP"
        }
    }
}

/// SimpleQueryString query
#[derive(Debug, Default)]
pub struct SimpleQueryStringQuery {
    query: String,
    fields: Option<Vec<String>>,
    default_operator: Option<String>,
    analyzer: Option<String>,
    flags: Option<Flags<SimpleQueryStringFlags>>,
    lowercase_expanded_terms: Option<bool>,
    analyze_wildcard: Option<bool>,
    locale: Option<String>,
    lenient: Option<bool>,
    minimum_should_match: Option<MinimumShouldMatch>
}

impl Query {
    pub fn build_simple_query_string<A: Into<String>>(query: A) -> SimpleQueryStringQuery {
        SimpleQueryStringQuery {
            query: query.into(),
            ..Default::default()
        }
    }
}

impl SimpleQueryStringQuery {
    add_option!(with_fields, fields, Vec<String>);
    add_option!(with_default_operator, default_operator, String);
    add_option!(with_analyzer, analyzer, String);
    add_option!(with_flags, flags, Flags<SimpleQueryStringFlags>);
    add_option!(with_lowercase_expanded_terms, lowercase_expanded_terms, bool);
    add_option!(with_analyze_wildcard, analyze_wildcard, bool);
    add_option!(with_locale, locale, String);
    add_option!(with_lenient, lenient, bool);
    add_option!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);

    build!(SimpleQueryString);
}

impl ToJson for SimpleQueryStringQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("query".to_owned(), self.query.to_json());
        optional_add!(self, d, fields);
        optional_add!(self, d, analyzer);
        optional_add!(self, d, flags);
        optional_add!(self, d, lowercase_expanded_terms);
        optional_add!(self, d, analyze_wildcard);
        optional_add!(self, d, locale);
        optional_add!(self, d, lenient);
        optional_add!(self, d, minimum_should_match);
        Json::Object(d)
    }
}

/// Term query
#[derive(Debug, Default)]
pub struct TermQuery {
    field: String,
    value: JsonVal,
    boost: Option<f64>
}

impl Query {
    pub fn build_term<A, B>(field: A, value: B) -> TermQuery
        where A: Into<String>,
              B: Into<JsonVal> {
        TermQuery {
            field: field.into(),
            value: value.into(),
            ..Default::default()
        }
    }
}

impl TermQuery {
    add_option!(with_boost, boost, f64);

    build!(Term);
}

impl ToJson for TermQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();

        inner.insert("value".to_owned(), self.value.to_json());
        optional_add!(self, inner, boost);

        d.insert(self.field.clone(), Json::Object(inner));
        Json::Object(d)
    }
}

// Terms query
/// Terms Query Lookup
#[derive(Debug, Default)]
pub struct TermsQueryLookup {
    index: Option<String>,
    doc_type: Option<String>,
    id: JsonVal,
    path: String,
    routing: Option<String>
}

impl<'a> TermsQueryLookup {
    pub fn new<A, B>(id: A, path: B) -> TermsQueryLookup
        where A: Into<JsonVal>,
              B: Into<String> {

        TermsQueryLookup {
            id: id.into(),
            path: path.into(),
            ..Default::default()
        }
    }

    add_option!(with_index, index, String);
    add_option!(with_type, doc_type, String);
    add_option!(with_routing, routing, String);
}

impl ToJson for TermsQueryLookup {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("id".to_owned(), self.id.to_json());
        d.insert("path".to_owned(), self.path.to_json());
        optional_add!(self, d, index);
        optional_add!(self, d, doc_type, "type");
        optional_add!(self, d, routing);
        Json::Object(d)
    }
}

/// TermsQueryIn
#[derive(Debug)]
pub enum TermsQueryIn {
    /// A `Vec` of values
    Values(Vec<JsonVal>),

    /// An indirect reference to another document
    Lookup(TermsQueryLookup)
}

impl Default for TermsQueryIn {
    fn default() -> Self {
        TermsQueryIn::Values(Default::default())
    }
}

impl ToJson for TermsQueryIn {
    fn to_json(&self) -> Json {
        match self {
            &TermsQueryIn::Values(ref v) => v.to_json(),
            &TermsQueryIn::Lookup(ref l) => l.to_json()
        }
    }
}

impl From<TermsQueryLookup> for TermsQueryIn {
    fn from(from: TermsQueryLookup) -> TermsQueryIn {
        TermsQueryIn::Lookup(from)
    }
}

impl From<Vec<JsonVal>> for TermsQueryIn {
    fn from(from: Vec<JsonVal>) -> TermsQueryIn {
        TermsQueryIn::Values(from)
    }
}

impl<'a, A> From<&'a [A]> for TermsQueryIn
    where A: JsonPotential {

    fn from(from: &'a [A]) -> TermsQueryIn {
        TermsQueryIn::Values(from.iter().map(|f| f.to_json_val()).collect())
    }
}

impl<A> From<Vec<A>> for TermsQueryIn
    where A: JsonPotential {

    fn from(from: Vec<A>) -> TermsQueryIn {
        (&from[..]).into()
    }
}

/// Terms Query
#[derive(Debug, Default)]
pub struct TermsQuery {
    field: String,
    values: TermsQueryIn
}

impl Query {
    pub fn build_terms<A>(field: A) -> TermsQuery
        where A: Into<String> {

        TermsQuery {
            field: field.into(),
            ..Default::default()
        }
    }
}

impl TermsQuery {
    pub fn with_values<T>(mut self, values: T) -> Self
        where T: Into<TermsQueryIn> {

        self.values = values.into();
        self
    }

    build!(Terms);
}

impl ToJson for TermsQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert(self.field.clone(), self.values.to_json());
        Json::Object(d)
    }
}

/// Range query
#[derive(Debug, Default)]
pub struct RangeQuery {
    field: String,
    gte: Option<JsonVal>,
    gt: Option<JsonVal>,
    lte: Option<JsonVal>,
    lt: Option<JsonVal>,
    boost: Option<f64>,
    time_zone: Option<String>,
    format: Option<String>
}

impl Query {
    pub fn build_range<A>(field: A) -> RangeQuery
        where A: Into<String> {
        RangeQuery {
            field: field.into(),
            ..Default::default()
        }
    }
}

impl RangeQuery {
    add_option!(with_gte, gte, JsonVal);
    add_option!(with_gt, gt, JsonVal);
    add_option!(with_lte, lte, JsonVal);
    add_option!(with_lt, lt, JsonVal);
    add_option!(with_boost, boost, f64);
    add_option!(with_time_zone, time_zone, String);
    add_option!(with_format, format, String);

    build!(Range);
}

impl ToJson for RangeQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();

        optional_add!(self, inner, gte);
        optional_add!(self, inner, gt);
        optional_add!(self, inner, lte);
        optional_add!(self, inner, lt);
        optional_add!(self, inner, boost);
        optional_add!(self, inner, time_zone);
        optional_add!(self, inner, format);

        d.insert(self.field.clone(), Json::Object(inner));
        Json::Object(d)
    }
}


/// Exists query
#[derive(Debug)]
pub struct ExistsQuery {
    field: String
}

impl Query {
    pub fn build_exists<A>(field: A) -> ExistsQuery
        where A: Into<String> {

        ExistsQuery {
            field: field.into()
        }
    }
}

impl ExistsQuery {
    build!(Exists);
}

impl ToJson for ExistsQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("field".to_owned(), self.field.to_json());
        Json::Object(d)
    }
}

// Old queries - TODO: move or delete these

impl Query {
                  // pub fn build_boosting(
                  //    ) -> BoostingQuery {

                  //        BoostingQuery {

                  //                positive:
                  //                                    None
                  //                                ,

                  //                negative:
                  //                                    None
                  //                                ,

                  //                negative_boost:
                  //                                    None


                  //         }

                  // }

                  // pub fn build_constant_score(
                  //    ) -> ConstantScoreQuery {

                  //        ConstantScoreQuery {

                  //                query:
                  //                                    None
                  //                                ,

                  //                boost:
                  //                                    None


                  //         }

                  // }

                  // pub fn build_dis_max<A: Into<Vec<Query>>>(

                  //        queries: A
                  //    ) -> DisMaxQuery {

                  //        DisMaxQuery {

                  //                tie_breaker:
                  //                                    None
                  //                                ,

                  //                boost:
                  //                                    None
                  //                                ,

                  //                queries:
                  //                                    queries.into()


                  //         }

                  // }

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

                  // pub fn build_function_score<A: Into<Vec<Function>>>(

                  //        functions: A
                  //    ) -> FunctionScoreQuery {

                  //        FunctionScoreQuery {

                  //                query:
                  //                                    None
                  //                                ,

                  //                boost:
                  //                                    None
                  //                                ,

                  //                functions:
                  //                                    functions.into()
                  //                                ,

                  //                max_boost:
                  //                                    None
                  //                                ,

                  //                score_mode:
                  //                                    None
                  //                                ,

                  //                boost_mode:
                  //                                    None
                  //                                ,

                  //                min_score:
                  //                                    None


                  //         }

                  // }

                  pub fn build_fuzzy<A: Into<String>,B: Into<String>>(

                         field: A,

                         value: B
                     ) -> FuzzyQuery {

                         FuzzyQuery {

                                 field:
                                                     field.into()
                                                 ,

                                 value:
                                                     value.into()
                                                 ,

                                 boost:
                                                     None
                                                 ,

                                 fuzziness:
                                                     None
                                                 ,

                                 prefix_length:
                                                     None
                                                 ,

                                 max_expansions:
                                                     None


                          }

                  }

                  pub fn build_geo_shape<A: Into<String>>(

                         field: A
                     ) -> GeoShapeQuery {

                         GeoShapeQuery {

                                 field:
                                                     field.into()
                                                 ,

                                 shape:
                                                     None
                                                 ,

                                 indexed_shape:
                                                     None


                          }

                  }

                  // pub fn build_has_child<A: Into<String>,B: Into<Box<Query>>>(

                  //        doc_type: A,

                  //        query: B
                  //    ) -> HasChildQuery {

                  //        HasChildQuery {

                  //                doc_type:
                  //                                    doc_type.into()
                  //                                ,

                  //                query:
                  //                                    query.into()
                  //                                ,

                  //                score_mode:
                  //                                    None
                  //                                ,

                  //                min_children:
                  //                                    None
                  //                                ,

                  //                max_children:
                  //                                    None


                  //         }

                  // }

                  // pub fn build_has_parent<A: Into<String>,B: Into<Box<Query>>>(

                  //        parent_type: A,

                  //        query: B
                  //    ) -> HasParentQuery {

                  //        HasParentQuery {

                  //                parent_type:
                  //                                    parent_type.into()
                  //                                ,

                  //                query:
                  //                                    query.into()
                  //                                ,

                  //                score_mode:
                  //                                    None


                  //         }

                  // }

                  pub fn build_ids<A: Into<Vec<String>>>(

                         values: A
                     ) -> IdsQuery {

                         IdsQuery {

                                 doc_type:
                                                     None
                                                 ,

                                 values:
                                                     values.into()


                          }

                  }

                  // pub fn build_indices<A: Into<Box<Query>>>(

                  //        query: A
                  //    ) -> IndicesQuery {

                  //        IndicesQuery {

                  //                index:
                  //                                    None
                  //                                ,

                  //                indices:
                  //                                    None
                  //                                ,

                  //                query:
                  //                                    query.into()
                  //                                ,

                  //                no_match_query:
                  //                                    None


                  //         }

                  // }

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

                  // pub fn build_nested<A: Into<String>,B: Into<Box<Query>>>(

                  //        path: A,

                  //        query: B
                  //    ) -> NestedQuery {

                  //        NestedQuery {

                  //                path:
                  //                                    path.into()
                  //                                ,

                  //                score_mode:
                  //                                    None
                  //                                ,

                  //                query:
                  //                                    query.into()


                  //         }

                  // }

                  pub fn build_prefix<A: Into<String>,B: Into<String>>(

                         field: A,

                         value: B
                     ) -> PrefixQuery {

                         PrefixQuery {

                                 field:
                                                     field.into()
                                                 ,

                                 value:
                                                     value.into()
                                                 ,

                                 boost:
                                                     None
                                                 ,

                                 rewrite:
                                                     None


                          }

                  }

                  // pub fn build_regexp<A: Into<String>,B: Into<String>>(

                  //        field: A,

                  //        value: B
                  //    ) -> RegexpQuery {

                  //        RegexpQuery {

                  //                field:
                  //                                    field.into()
                  //                                ,

                  //                value:
                  //                                    value.into()
                  //                                ,

                  //                boost:
                  //                                    None
                  //                                ,

                  //                flags:
                  //                                    None
                  //                                ,

                  //                max_determined_states:
                  //                                    None


                  //         }

                  // }

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

                  pub fn build_wildcard<A: Into<String>,B: Into<String>>(

                         field: A,

                         value: B
                     ) -> WildcardQuery {

                         WildcardQuery {

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


          // #[derive(Debug)]
          // pub struct BoostingQuery {

          //         positive:
          //                                Option<Box<Query>>
          //                             ,

          //         negative:
          //                                Option<Box<Query>>
          //                             ,

          //         negative_boost:
          //                                Option<f64>


          // }

          // impl BoostingQuery {

          //         pub fn with_positive<T: Into<Box<Query>>>(mut self, value: T) -> Self {
          //             self.positive = Some(value.into());
          //             self
          //         }

          //         pub fn with_negative<T: Into<Box<Query>>>(mut self, value: T) -> Self {
          //             self.negative = Some(value.into());
          //             self
          //         }

          //         pub fn with_negative_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.negative_boost = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.positive, "positive");

          //             // optional_add!(self, m, self.negative, "negative");

          //             // optional_add!(self, m, self.negative_boost, "negative_boost");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::Boosting(self)
          //     }
          // }

        // impl ToJson for BoostingQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }



          // #[derive(Debug)]
          // pub struct ConstantScoreQuery {

          //         query:
          //                                Option<Box<Query>>
          //                             ,

          //         boost:
          //                                Option<f64>


          // }

          // impl ConstantScoreQuery {

          //         pub fn with_query<T: Into<Box<Query>>>(mut self, value: T) -> Self {
          //             self.query = Some(value.into());
          //             self
          //         }

          //         pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {
          //             // optional_add!(self, m, self.query, "query");

          //             // optional_add!(self, m, self.boost, "boost");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::ConstantScore(self)
          //     }
          // }

        // impl ToJson for ConstantScoreQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          // #[derive(Debug)]
          // pub struct DisMaxQuery {

          //         tie_breaker:
          //                                Option<f64>
          //                             ,

          //         boost:
          //                                Option<f64>
          //                             ,

          //         queries:
          //                                Vec<Query>


          // }

          // impl DisMaxQuery {

          //         pub fn with_tie_breaker<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.tie_breaker = Some(value.into());
          //             self
          //         }

          //         pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.tie_breaker, "tie_breaker");

          //             // optional_add!(self, m, self.boost, "boost");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::DisMax(self)
          //     }
          // }

        // impl ToJson for DisMaxQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("queries".to_owned(),
        //                    self.queries.to_json());

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


          // #[derive(Debug)]
          // pub struct FunctionScoreQuery {

          //         query:
          //                                Option<Box<Query>>
          //                             ,

          //         boost:
          //                                Option<f64>
          //                             ,

          //         functions:
          //                                Vec<Function>
          //                             ,

          //         max_boost:
          //                                Option<f64>
          //                             ,

          //         score_mode:
          //                                Option<ScoreMode>
          //                             ,

          //         boost_mode:
          //                                Option<BoostMode>
          //                             ,

          //         min_score:
          //                                Option<f64>


          // }

          // impl FunctionScoreQuery {

          //         pub fn with_query<T: Into<Box<Query>>>(mut self, value: T) -> Self {
          //             self.query = Some(value.into());
          //             self
          //         }

          //         pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost = Some(value.into());
          //             self
          //         }

          //         pub fn with_max_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.max_boost = Some(value.into());
          //             self
          //         }

          //         pub fn with_score_mode<T: Into<ScoreMode>>(mut self, value: T) -> Self {
          //             self.score_mode = Some(value.into());
          //             self
          //         }

          //         pub fn with_boost_mode<T: Into<BoostMode>>(mut self, value: T) -> Self {
          //             self.boost_mode = Some(value.into());
          //             self
          //         }

          //         pub fn with_min_score<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.min_score = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.query, "query");

          //             // optional_add!(self, m, self.boost, "boost");

          //             // optional_add!(self, m, self.max_boost, "max_boost");

          //             // optional_add!(self, m, self.score_mode, "score_mode");

          //             // optional_add!(self, m, self.boost_mode, "boost_mode");

          //             // optional_add!(self, m, self.min_score, "min_score");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::FunctionScore(self)
          //     }
          // }


        #[derive(Debug)]
        pub enum ScoreMode {

                Multiply
                ,

                Sum
                ,

                Avg
                ,

                First
                ,

                Max
                ,

                Min


        }

        impl ToJson for ScoreMode {
            fn to_json(&self) -> Json {
                match self {

                        &ScoreMode::Multiply
                        => "multiply".to_json()
                        ,

                        &ScoreMode::Sum
                        => "sum".to_json()
                        ,

                        &ScoreMode::Avg
                        => "avg".to_json()
                        ,

                        &ScoreMode::First
                        => "first".to_json()
                        ,

                        &ScoreMode::Max
                        => "max".to_json()
                        ,

                        &ScoreMode::Min
                        => "min".to_json()


                }
            }
        }

        #[derive(Debug)]
        pub enum BoostMode {

                Multiply
                ,

                Replace
                ,

                Sum
                ,

                Avg
                ,

                Max
                ,

                Min


        }

        impl ToJson for BoostMode {
            fn to_json(&self) -> Json {
                match self {

                        &BoostMode::Multiply
                        => "multiply".to_json()
                        ,

                        &BoostMode::Replace
                        => "replace".to_json()
                        ,

                        &BoostMode::Sum
                        => "sum".to_json()
                        ,

                        &BoostMode::Avg
                        => "avg".to_json()
                        ,

                        &BoostMode::Max
                        => "max".to_json()
                        ,

                        &BoostMode::Min
                        => "min".to_json()


                }
            }
        }


        // impl ToJson for FunctionScoreQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("functions".to_owned(),
        //                    self.functions.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          #[derive(Debug)]
          pub struct FuzzyQuery {

                  field:
                                         String
                                      ,

                  value:
                                         String
                                      ,

                  boost:
                                         Option<f64>
                                      ,

                  fuzziness:
                                         Option<Fuzziness>
                                      ,

                  prefix_length:
                                         Option<u64>
                                      ,

                  max_expansions:
                                         Option<u64>


          }

          // impl FuzzyQuery {

          //         pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost = Some(value.into());
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

          //         pub fn with_max_expansions<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.max_expansions = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.boost, "boost");

          //             // optional_add!(self, m, self.fuzziness, "fuzziness");

          //             // optional_add!(self, m, self.prefix_length, "prefix_length");

          //             // optional_add!(self, m, self.max_expansions, "max_expansions");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::Fuzzy(self)
          //     }
          // }

        // impl ToJson for FuzzyQuery {
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


// Required for GeoShape

#[derive(Debug)]
pub struct Shape {
    shape_type: String,
    coordinates: Vec<(f64, f64)>
}

impl Shape {
    pub fn new<A: Into<String>>(shape_type: A, coordinates: Vec<(f64, f64)>) -> Shape {
        Shape {
            shape_type:  shape_type.into(),
            coordinates: coordinates
        }
    }
}

impl ToJson for Shape {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();

        inner.insert("type".to_owned(), self.shape_type.to_json());

        let coordinates:Vec<Vec<f64>> = self.coordinates
            .iter()
            .map (|&(a, b)| vec![a, b])
            .collect();
        inner.insert("coordinates".to_owned(), coordinates.to_json());

        d.insert("shape".to_owned(), Json::Object(inner));
        Json::Object(d)
    }
}

#[derive(Debug)]
pub struct IndexedShape {
    id:       String,
    doc_type: String,
    index:    String,
    path:     String
}

impl IndexedShape {
    pub fn new<A, B, C, D>(id: A, doc_type: B, index: C, path: D) -> IndexedShape
        where A: Into<String>,
              B: Into<String>,
              C: Into<String>,
              D: Into<String>
    {
        IndexedShape {
            id: id.into(),
            doc_type: doc_type.into(),
            index: index.into(),
            path: path.into()
        }
    }
}

impl ToJson for IndexedShape {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();
        inner.insert("id".to_owned(), self.id.to_json());
        inner.insert("type".to_owned(), self.doc_type.to_json());
        inner.insert("index".to_owned(), self.index.to_json());
        inner.insert("path".to_owned(), self.path.to_json());
        d.insert("indexed_shape".to_owned(), Json::Object(inner));
        Json::Object(d)
    }
}

          #[derive(Debug)]
          pub struct GeoShapeQuery {

                  field:
                                         String
                                      ,

                  shape:
                                         Option<Shape>
                                      ,

                  indexed_shape:
                                         Option<IndexedShape>


          }

          // impl GeoShapeQuery {

          //         pub fn with_shape<T: Into<Shape>>(mut self, value: T) -> Self {
          //             self.shape = Some(value.into());
          //             self
          //         }

          //         pub fn with_indexed_shape<T: Into<IndexedShape>>(mut self, value: T) -> Self {
          //             self.indexed_shape = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.shape, "shape");

          //             // optional_add!(self, m, self.indexed_shape, "indexed_shape");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::GeoShape(self)
          //     }
          // }

        // impl ToJson for GeoShapeQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();
        //         let mut inner = BTreeMap::new();


        //         self.add_optionals(&mut inner);
        //         d.insert(self.field.clone(), Json::Object(inner));
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          // #[derive(Debug)]
          // pub struct HasChildQuery {

          //         doc_type:
          //                                String
          //                             ,

          //         query:
          //                                Box<Query>
          //                             ,

          //         score_mode:
          //                                Option<ScoreMode>
          //                             ,

          //         min_children:
          //                                Option<u64>
          //                             ,

          //         max_children:
          //                                Option<u64>


          // }

          // impl HasChildQuery {

          //         pub fn with_score_mode<T: Into<ScoreMode>>(mut self, value: T) -> Self {
          //             self.score_mode = Some(value.into());
          //             self
          //         }

          //         pub fn with_min_children<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.min_children = Some(value.into());
          //             self
          //         }

          //         pub fn with_max_children<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.max_children = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.score_mode, "score_mode");

          //             // optional_add!(self, m, self.min_children, "min_children");

          //             // optional_add!(self, m, self.max_children, "max_children");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::HasChild(self)
          //     }
          // }

        // impl ToJson for HasChildQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("type".to_owned(),
        //                    self.doc_type.to_json());

        //           d.insert("query".to_owned(),
        //                    self.query.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          // #[derive(Debug)]
          // pub struct HasParentQuery {

          //         parent_type:
          //                                String
          //                             ,

          //         query:
          //                                Box<Query>
          //                             ,

          //         score_mode:
          //                                Option<ScoreMode>


          // }

          // impl HasParentQuery {

          //         pub fn with_score_mode<T: Into<ScoreMode>>(mut self, value: T) -> Self {
          //             self.score_mode = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //         //optional_add!(self, m, self.score_mode, "score_mode");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::HasParent(self)
          //     }
          // }

        // impl ToJson for HasParentQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("parent_type".to_owned(),
        //                    self.parent_type.to_json());

        //           d.insert("query".to_owned(),
        //                    self.query.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          #[derive(Debug)]
          pub struct IdsQuery {

                  doc_type:
                                         Option<OneOrMany<String>>
                                      ,

                  values:
                                         Vec<String>


          }

//           impl IdsQuery {

//                   pub fn with_type<T: Into<OneOrMany<String>>>(mut self, value: T) -> Self {
//                       self.doc_type = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

// //                      optional_add!(self, m, self.doc_type, "type");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               pub fn build(self) -> Query {
//                   Query::Ids(self)
//               }
//           }

        // impl ToJson for IdsQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("values".to_owned(),
        //                    self.values.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          // #[derive(Debug)]
          // pub struct IndicesQuery {

          //         index:
          //                                Option<String>
          //                             ,

          //         indices:
          //                                Option<Vec<String>>
          //                             ,

          //         query:
          //                                Box<Query>
          //                             ,

          //         no_match_query:
          //                                Option<Box<Query>>


          // }

          // impl IndicesQuery {

          //         pub fn with_index<T: Into<String>>(mut self, value: T) -> Self {
          //             self.index = Some(value.into());
          //             self
          //         }

          //         pub fn with_indices<T: Into<Vec<String>>>(mut self, value: T) -> Self {
          //             self.indices = Some(value.into());
          //             self
          //         }

          //         pub fn with_no_match_query<T: Into<Box<Query>>>(mut self, value: T) -> Self {
          //             self.no_match_query = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.index, "index");

          //             // optional_add!(self, m, self.indices, "indices");

          //             // optional_add!(self, m, self.no_match_query, "no_match_query");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::Indices(self)
          //     }
          // }

        // impl ToJson for IndicesQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("query".to_owned(),
        //                    self.query.to_json());

        //         self.add_optionals(&mut d);
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


          // #[derive(Debug)]
          // pub struct NestedQuery {

          //         path:
          //                                String
          //                             ,

          //         score_mode:
          //                                Option<ScoreMode>
          //                             ,

          //         query:
          //                                Box<Query>


          // }

          // impl NestedQuery {

          //         pub fn with_score_mode<T: Into<ScoreMode>>(mut self, value: T) -> Self {
          //             self.score_mode = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //         //optional_add!(self, m, self.score_mode, "score_mode");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::Nested(self)
          //     }
          // }

        // impl ToJson for NestedQuery {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("path".to_owned(),
        //                    self.path.to_json());

        //           d.insert("query".to_owned(),
        //                    self.query.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          #[derive(Debug)]
          pub struct PrefixQuery {

                  field:
                                         String
                                      ,

                  value:
                                         String
                                      ,

                  boost:
                                         Option<f64>
                                      ,

                  rewrite:
                                         Option<Rewrite>


          }

          // impl PrefixQuery {

          //         pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost = Some(value.into());
          //             self
          //         }

          //         pub fn with_rewrite<T: Into<Rewrite>>(mut self, value: T) -> Self {
          //             self.rewrite = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.boost, "boost");

          //             // optional_add!(self, m, self.rewrite, "rewrite");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::Prefix(self)
          //     }
          // }


#[derive(Debug)]
pub enum Rewrite {
    ConstantScoreAuto,
    ScoringBoolean,
    ConstantScoreBoolean,
    ConstantScoreFilter,
    TopTerms(i64),
    TopTermsBoost(i64)
}

impl ToJson for Rewrite {
    fn to_json(&self) -> Json {
        match self {
            &Rewrite::ConstantScoreAuto    => "constant_score_auto".to_json(),
            &Rewrite::ScoringBoolean       => "scoring_boolean".to_json(),
            &Rewrite::ConstantScoreBoolean => "constant_score_boolean".to_json(),
            &Rewrite::ConstantScoreFilter  => "constant_score_filter".to_json(),
            &Rewrite::TopTerms(n)          => format!("top_terms_{}", n).to_json(),
            &Rewrite::TopTermsBoost(n)     => format!("top_terms_boost_{}", n).to_json()
        }
    }
}

        // impl ToJson for PrefixQuery {
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



          // #[derive(Debug)]
          // pub struct RegexpQuery {

          //         field:
          //                                String
          //                             ,

          //         value:
          //                                String
          //                             ,

          //         boost:
          //                                Option<f64>
          //                             ,

          //         flags:
          //                                Option<Flags>
          //                             ,

          //         max_determined_states:
          //                                Option<u64>


          // }

          // impl RegexpQuery {

          //         pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.boost = Some(value.into());
          //             self
          //         }

          //         pub fn with_flags<T: Into<Flags>>(mut self, value: T) -> Self {
          //             self.flags = Some(value.into());
          //             self
          //         }

          //         pub fn with_max_determined_states<T: Into<u64>>(mut self, value: T) -> Self {
          //             self.max_determined_states = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             // optional_add!(self, m, self.boost, "boost");

          //             // optional_add!(self, m, self.flags, "flags");

          //             // optional_add!(self, m, self.max_determined_states, "max_determined_states");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Query {
          //         Query::Regexp(self)
          //     }
          // }


// TODO: this is a set of flags to one specific operation, and should probably
// be renamed because of that.
#[derive(Debug)]
pub enum Flag {
    All,
    AnyString,
    Complement,
    Intersection,
    Interval,
    None
}

impl ToString for Flag {
    fn to_string(&self) -> String {
        match self {
            &Flag::All => "ALL",
            &Flag::AnyString => "ANYSTRING",
            &Flag::Complement => "COMPLEMENT",
            &Flag::Intersection => "INTERSECTION",
            &Flag::Interval => "INTERVAL",
            &Flag::None => "NONE"
        }.to_owned()
    }
}

// TODO: remove due to be being superseded
// #[derive(Debug)]
// pub struct Flags {
//     flags: Vec<Flag>
// }

// impl Flags {
//     pub fn new() -> Flags {
//         Flags {
//             flags: vec![]
//         }
//     }

//     pub fn add_flag(mut self, flag: Flag) -> Self {
//         self.flags.push(flag);
//         self
//     }
// }

// impl ToJson for Flags {
//     fn to_json(&self) -> Json {
//         Json::String(self.flags.iter().map(|f| f.to_string()).join("|"))
//     }
// }

        // impl ToJson for RegexpQuery {
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





          #[derive(Debug)]
          pub struct WildcardQuery {

                  field:
                                         String
                                      ,

                  value:
                                         String
                                      ,

                  boost:
                                         Option<f64>


          }

        //   impl WildcardQuery {

        //           pub fn with_boost<T: Into<f64>>(mut self, value: T) -> Self {
        //               self.boost = Some(value.into());
        //               self
        //           }


        //       #[allow(dead_code, unused_variables)]
        //       fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //           //optional_add!(self, m, self.boost, "boost");

        //       }

        //       #[allow(dead_code, unused_variables)]
        //       fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //       }

        //       pub fn build(self) -> Query {
        //           Query::Wildcard(self)
        //       }
        //   }

        // impl ToJson for WildcardQuery {
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


//           #[derive(Debug)]
//           pub struct GeoBoundingBoxFilter {

//                   field:
//                                          String
//                                       ,

//                   geo_box:
//                                          GeoBox
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

//           impl GeoBoundingBoxFilter {

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
//                   Filter::GeoBoundingBox(self)
//               }
//           }


// impl ToJson for GeoBoundingBoxFilter {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert(self.field.clone(), self.geo_box.to_json());
//         self.add_optionals(&mut d);
//         self.add_core_optionals(&mut d);
//         Json::Object(d)
//     }
// }

//           #[derive(Debug)]
//           pub struct GeoDistanceFilter {

//                   field:
//                                          String
//                                       ,

//                   location:
//                                          Location
//                                       ,

//                   distance:
//                                          Distance
//                                       ,

//                   distance_type:
//                                          Option<DistanceType>
//                                       ,

//                   optimize_bbox:
//                                          Option<OptimizeBbox>
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


// #[derive(Debug)]
// pub struct Distance {
//     amt: f64,
//     unit: DistanceUnit
// }

// impl Distance {
//     pub fn new(amt: f64, unit: DistanceUnit) -> Distance {
//         Distance {
//             amt:  amt,
//             unit: unit
//         }
//     }
// }

// impl ToJson for Distance {
//     fn to_json(&self) -> Json {
//         Json::String(format!("{}{}", self.amt, self.unit.to_string()))
//     }
// }

//         #[derive(Debug)]
//         pub enum OptimizeBbox {

//                 Memory
//                 ,

//                 Indexed
//                 ,

//                 None


//         }

//         impl ToJson for OptimizeBbox {
//             fn to_json(&self) -> Json {
//                 match self {

//                         &OptimizeBbox::Memory
//                         => "memory".to_json()
//                         ,

//                         &OptimizeBbox::Indexed
//                         => "indexed".to_json()
//                         ,

//                         &OptimizeBbox::None
//                         => "none".to_json()


//                 }
//             }
//         }


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


// #[derive(Debug)]
// pub enum NoMatchFilter {
//     None,
//     All,
//     Filter(Box<Filter>)
// }

// from_exp!(Filter, NoMatchFilter, from, NoMatchFilter::Filter(Box::new(from)));

// impl ToJson for NoMatchFilter {
//     fn to_json(&self) -> Json {
//         match self {
//             &NoMatchFilter::None               => "none".to_json(),
//             &NoMatchFilter::All                => "all".to_json(),
//             &NoMatchFilter::Filter(ref filter) => filter.to_json()
//         }
//     }
// }

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
//           pub struct MissingFilter {

//                   field:
//                                          String
//                                       ,

//                   existence:
//                                          Option<bool>
//                                       ,

//                   null_value:
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

//           impl MissingFilter {

//                   pub fn with_existence<T: Into<bool>>(mut self, value: T) -> Self {
//                       self.existence = Some(value.into());
//                       self
//                   }

//                   pub fn with_null_value<T: Into<bool>>(mut self, value: T) -> Self {
//                       self.null_value = Some(value.into());
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

//                       optional_add!(self, m, self.existence, "existence");

//                       optional_add!(self, m, self.null_value, "null_value");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self._cache, "_cache");

//                       optional_add!(self, m, self._cache_key, "_cache_key");

//                       optional_add!(self, m, self._name, "_name");

//               }

//               pub fn build(self) -> Filter {
//                   Filter::Missing(self)
//               }
//           }

//         impl ToJson for MissingFilter {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();

//                   d.insert("field".to_owned(),
//                            self.field.to_json());

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


// impl ToJson for TermsFilter {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert(self.field.clone(), self.values.to_json());
//         self.add_optionals(&mut d);
//         self.add_core_optionals(&mut d);
//         Json::Object(d)
//     }
// }

// TODO: determine whether this is used or not
        //   #[derive(Debug)]
        //   pub struct TypeFilter {

        //           value:
        //                                  String
        //                               ,

        //           _cache:
        //                                  Option<bool>
        //                               ,

        //           _cache_key:
        //                                  Option<String>
        //                               ,

        //           _name:
        //                                  Option<String>


        //   }

        //   impl TypeFilter {

        //           pub fn with_cache<T: Into<bool>>(mut self, value: T) -> Self {
        //               self._cache = Some(value.into());
        //               self
        //           }

        //           pub fn with_cache_key<T: Into<String>>(mut self, value: T) -> Self {
        //               self._cache_key = Some(value.into());
        //               self
        //           }

        //           pub fn with_name<T: Into<String>>(mut self, value: T) -> Self {
        //               self._name = Some(value.into());
        //               self
        //           }


        //       #[allow(dead_code, unused_variables)]
        //       fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //       }

        //       #[allow(dead_code, unused_variables)]
        //       fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //               optional_add!(self, m, self._cache, "_cache");

        //               optional_add!(self, m, self._cache_key, "_cache_key");

        //               optional_add!(self, m, self._name, "_name");

        //       }

        //       pub fn build(self) -> Filter {
        //           Filter::Type(self)
        //       }
        //   }

        // impl ToJson for TypeFilter {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("value".to_owned(),
        //                    self.value.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


// Functions for use in `FunctionScoreQuery`

        // #[derive(Debug)]
        // pub enum Func {

        //         ScriptScore(ScriptScoreFunc)
        //         ,

        //         RandomScore(RandomScoreFunc)
        //         ,

        //         FieldValueFactor(FieldValueFactorFunc)
        //         ,

        //         Linear(LinearFunc)
        //         ,

        //         Exp(ExpFunc)
        //         ,

        //         Gauss(GaussFunc)


        // }

        // impl Func {

        //         pub fn build_script_score<A: Into<String>>(

        //               script: A

        //         ) -> ScriptScoreFunc {
        //            ScriptScoreFunc {

        //                   script:
        //                                      script.into()
        //                                   ,

        //                   lang:
        //                                      None
        //                                   ,

        //                   params:
        //                                      None


        //            }
        //         }

        //         pub fn build_random_score<A: Into<u64>>(

        //               seed: A

        //         ) -> RandomScoreFunc {
        //            RandomScoreFunc {

        //                   seed:
        //                                      seed.into()


        //            }
        //         }

        //         pub fn build_field_value_factor<A: Into<String>>(

        //               field: A

        //         ) -> FieldValueFactorFunc {
        //            FieldValueFactorFunc {

        //                   field:
        //                                      field.into()
        //                                   ,

        //                   factor:
        //                                      None
        //                                   ,

        //                   modifier:
        //                                      None


        //            }
        //         }

        //         pub fn build_linear<A: Into<String>,B: Into<Origin>>(

        //               field: A,

        //               origin: B

        //         ) -> LinearFunc {
        //            LinearFunc {

        //                   field:
        //                                      field.into()
        //                                   ,

        //                   origin:
        //                                      origin.into()
        //                                   ,

        //                   scale:
        //                                      None
        //                                   ,

        //                   offset:
        //                                      None
        //                                   ,

        //                   decay:
        //                                      None
        //                                   ,

        //                   multi_value_mode:
        //                                      None


        //            }
        //         }

        //         pub fn build_exp<A: Into<String>,B: Into<Origin>>(

        //               field: A,

        //               origin: B

        //         ) -> ExpFunc {
        //            ExpFunc {

        //                   field:
        //                                      field.into()
        //                                   ,

        //                   origin:
        //                                      origin.into()
        //                                   ,

        //                   scale:
        //                                      None
        //                                   ,

        //                   offset:
        //                                      None
        //                                   ,

        //                   decay:
        //                                      None
        //                                   ,

        //                   multi_value_mode:
        //                                      None


        //            }
        //         }

        //         pub fn build_gauss<A: Into<String>,B: Into<Origin>>(

        //               field: A,

        //               origin: B

        //         ) -> GaussFunc {
        //            GaussFunc {

        //                   field:
        //                                      field.into()
        //                                   ,

        //                   origin:
        //                                      origin.into()
        //                                   ,

        //                   scale:
        //                                      None
        //                                   ,

        //                   offset:
        //                                      None
        //                                   ,

        //                   decay:
        //                                      None
        //                                   ,

        //                   multi_value_mode:
        //                                      None


        //            }
        //         }


        //     fn name(&self) -> String {
        //         match self {

        //                 &Func::ScriptScore(_) => "script_score"
        //                 ,

        //                 &Func::RandomScore(_) => "random_score"
        //                 ,

        //                 &Func::FieldValueFactor(_) => "field_value_factor"
        //                 ,

        //                 &Func::Linear(_) => "linear"
        //                 ,

        //                 &Func::Exp(_) => "exp"
        //                 ,

        //                 &Func::Gauss(_) => "gauss"


        //         }.to_owned()
        //     }
        // }

        // impl ToJson for Func {
        //     fn to_json(&self) -> Json {
        //         match self {

        //                 &Func::ScriptScore(ref inner)
        //                 => inner.to_json()
        //                 ,

        //                 &Func::RandomScore(ref inner)
        //                 => inner.to_json()
        //                 ,

        //                 &Func::FieldValueFactor(ref inner)
        //                 => inner.to_json()
        //                 ,

        //                 &Func::Linear(ref inner)
        //                 => inner.to_json()
        //                 ,

        //                 &Func::Exp(ref inner)
        //                 => inner.to_json()
        //                 ,

        //                 &Func::Gauss(ref inner)
        //                 => inner.to_json()


        //         }
        //     }
        // }


        //   #[derive(Debug)]
        //   pub struct ScriptScoreFunc {

        //           script:
        //                                  String
        //                               ,

        //           lang:
        //                                  Option<String>
        //                               ,

        //           params:
        //                                  Option<BTreeMap<String, JsonVal>>


        //   }

        //   impl ScriptScoreFunc {

        //           pub fn with_lang<T: Into<String>>(mut self, value: T) -> Self {
        //               self.lang = Some(value.into());
        //               self
        //           }

        //           pub fn with_params<T: Into<BTreeMap<String, JsonVal>>>(mut self, value: T) -> Self {
        //               self.params = Some(value.into());
        //               self
        //           }


        //       #[allow(dead_code, unused_variables)]
        //       fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //               optional_add!(self, m, self.lang, "lang");

        //               optional_add!(self, m, self.params, "params");

        //       }

        //       #[allow(dead_code, unused_variables)]
        //       fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //       }

        //       pub fn build(self) -> Func {
        //           Func::ScriptScore(self)
        //       }
        //   }

        // impl ToJson for ScriptScoreFunc {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("script".to_owned(),
        //                    self.script.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


        //   #[derive(Debug)]
        //   pub struct RandomScoreFunc {

        //           seed:
        //                                  u64


        //   }

        //   impl RandomScoreFunc {


        //       #[allow(dead_code, unused_variables)]
        //       fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //       }

        //       #[allow(dead_code, unused_variables)]
        //       fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

        //       }

        //       pub fn build(self) -> Func {
        //           Func::RandomScore(self)
        //       }
        //   }

        // impl ToJson for RandomScoreFunc {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("seed".to_owned(),
        //                    self.seed.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          // #[derive(Debug)]
          // pub struct FieldValueFactorFunc {

          //         field:
          //                                String
          //                             ,

          //         factor:
          //                                Option<f64>
          //                             ,

          //         modifier:
          //                                Option<Modifier>


          // }

          // impl FieldValueFactorFunc {

          //         pub fn with_factor<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.factor = Some(value.into());
          //             self
          //         }

          //         pub fn with_modifier<T: Into<Modifier>>(mut self, value: T) -> Self {
          //             self.modifier = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             optional_add!(self, m, self.factor, "factor");

          //             optional_add!(self, m, self.modifier, "modifier");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Func {
          //         Func::FieldValueFactor(self)
          //     }
          // }


        #[derive(Debug)]
        pub enum Modifier {

                None
                ,

                Log
                ,

                Log1p
                ,

                Log2p
                ,

                Ln
                ,

                Ln1p
                ,

                Ln2p
                ,

                Square
                ,

                Sqrt
                ,

                Reciprocal


        }

        impl ToJson for Modifier {
            fn to_json(&self) -> Json {
                match self {

                        &Modifier::None
                        => "none".to_json()
                        ,

                        &Modifier::Log
                        => "log".to_json()
                        ,

                        &Modifier::Log1p
                        => "log1p".to_json()
                        ,

                        &Modifier::Log2p
                        => "log2p".to_json()
                        ,

                        &Modifier::Ln
                        => "ln".to_json()
                        ,

                        &Modifier::Ln1p
                        => "ln1p".to_json()
                        ,

                        &Modifier::Ln2p
                        => "ln2p".to_json()
                        ,

                        &Modifier::Square
                        => "square".to_json()
                        ,

                        &Modifier::Sqrt
                        => "sqrt".to_json()
                        ,

                        &Modifier::Reciprocal
                        => "reciprocal".to_json()


                }
            }
        }


        // impl ToJson for FieldValueFactorFunc {
        //     fn to_json(&self) -> Json {
        //         let mut d = BTreeMap::new();

        //           d.insert("field".to_owned(),
        //                    self.field.to_json());

        //         self.add_optionals(&mut d);
        //         self.add_core_optionals(&mut d);
        //         Json::Object(d)
        //     }
        // }


          // #[derive(Debug)]
          // pub struct LinearFunc {

          //         field:
          //                                String
          //                             ,

          //         origin:
          //                                Origin
          //                             ,

          //         scale:
          //                                Option<Scale>
          //                             ,

          //         offset:
          //                                Option<Scale>
          //                             ,

          //         decay:
          //                                Option<f64>
          //                             ,

          //         multi_value_mode:
          //                                Option<MultiValueMode>


          // }

          // impl LinearFunc {

          //         pub fn with_scale<T: Into<Scale>>(mut self, value: T) -> Self {
          //             self.scale = Some(value.into());
          //             self
          //         }

          //         pub fn with_offset<T: Into<Scale>>(mut self, value: T) -> Self {
          //             self.offset = Some(value.into());
          //             self
          //         }

          //         pub fn with_decay<T: Into<f64>>(mut self, value: T) -> Self {
          //             self.decay = Some(value.into());
          //             self
          //         }

          //         pub fn with_multi_value_mode<T: Into<MultiValueMode>>(mut self, value: T) -> Self {
          //             self.multi_value_mode = Some(value.into());
          //             self
          //         }


          //     #[allow(dead_code, unused_variables)]
          //     fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //             optional_add!(self, m, self.scale, "scale");

          //             optional_add!(self, m, self.offset, "offset");

          //             optional_add!(self, m, self.decay, "decay");

          //             optional_add!(self, m, self.multi_value_mode, "multi_value_mode");

          //     }

          //     #[allow(dead_code, unused_variables)]
          //     fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

          //     }

          //     pub fn build(self) -> Func {
          //         Func::Linear(self)
          //     }
          // }


// #[derive(Debug)]
// enum Scale {
//     I64(i64),
//     U64(u64),
//     F64(f64),
//     Distance(Distance),
//     Duration(Duration)
// }

// from!(i64, Scale, I64);
// from!(u64, Scale, U64);
// from!(f64, Scale, F64);
// from!(Distance, Scale, Distance);
// from!(Duration, Scale, Duration);

// impl ToJson for Scale {
//     fn to_json(&self) -> Json {
//         match self {
//             &Scale::I64(s)          => Json::I64(s),
//             &Scale::U64(s)          => Json::U64(s),
//             &Scale::F64(s)          => Json::F64(s),
//             &Scale::Distance(ref s) => s.to_json(),
//             &Scale::Duration(ref s) => s.to_json()
//         }
//     }
// }

#[derive(Debug)]
enum Origin {
    I64(i64),
    U64(u64),
    F64(f64),
    Location(Location),
    Date(String)
}

from!(i64, Origin, I64);
from!(u64, Origin, U64);
from!(f64, Origin, F64);
from!(Location, Origin, Location);
from!(String, Origin, Date);

impl ToJson for Origin {
    fn to_json(&self) -> Json {
        match self {
            &Origin::I64(orig)          => Json::I64(orig),
            &Origin::U64(orig)          => Json::U64(orig),
            &Origin::F64(orig)          => Json::F64(orig),
            &Origin::Location(ref orig) => orig.to_json(),
            &Origin::Date(ref orig)     => Json::String(orig.clone())
        }
    }
}

// TODO: determine if still required or not
// macro_rules! decay_func_json_impl {
//     ($df:ident) => {
//         impl ToJson for $df {
//             fn to_json(&self) -> Json {
//                 let mut d = BTreeMap::new();
//                 let mut inner = BTreeMap::new();
//                 inner.insert("origin".to_owned(), self.origin.to_json());
//                 optional_add!(self, inner, self.scale, "scale");
//                 optional_add!(self, inner, self.decay, "decay");
//                 optional_add!(self, inner, self.offset, "offset");
//                 d.insert(self.field.clone(), Json::Object(inner));
//                 optional_add!(self, d, self.multi_value_mode, "multi_value_mode");
//                 Json::Object(d)
//             }
//         }
//     }
// }

        #[derive(Debug)]
        pub enum MultiValueMode {

                Min
                ,

                Max
                ,

                Avg
                ,

                Sum


        }

        impl ToJson for MultiValueMode {
            fn to_json(&self) -> Json {
                match self {

                        &MultiValueMode::Min
                        => "min".to_json()
                        ,

                        &MultiValueMode::Max
                        => "max".to_json()
                        ,

                        &MultiValueMode::Avg
                        => "avg".to_json()
                        ,

                        &MultiValueMode::Sum
                        => "sum".to_json()


                }
            }
        }


// decay_func_json_impl!(LinearFunc);

//           #[derive(Debug)]
//           pub struct ExpFunc {

//                   field:
//                                          String
//                                       ,

//                   origin:
//                                          Origin
//                                       ,

//                   scale:
//                                          Option<Scale>
//                                       ,

//                   offset:
//                                          Option<Scale>
//                                       ,

//                   decay:
//                                          Option<f64>
//                                       ,

//                   multi_value_mode:
//                                          Option<MultiValueMode>


//           }

//           impl ExpFunc {

//                   pub fn with_scale<T: Into<Scale>>(mut self, value: T) -> Self {
//                       self.scale = Some(value.into());
//                       self
//                   }

//                   pub fn with_offset<T: Into<Scale>>(mut self, value: T) -> Self {
//                       self.offset = Some(value.into());
//                       self
//                   }

//                   pub fn with_decay<T: Into<f64>>(mut self, value: T) -> Self {
//                       self.decay = Some(value.into());
//                       self
//                   }

//                   pub fn with_multi_value_mode<T: Into<MultiValueMode>>(mut self, value: T) -> Self {
//                       self.multi_value_mode = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.scale, "scale");

//                       optional_add!(self, m, self.offset, "offset");

//                       optional_add!(self, m, self.decay, "decay");

//                       optional_add!(self, m, self.multi_value_mode, "multi_value_mode");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               pub fn build(self) -> Func {
//                   Func::Exp(self)
//               }
//           }

// decay_func_json_impl!(ExpFunc);

//           #[derive(Debug)]
//           pub struct GaussFunc {

//                   field:
//                                          String
//                                       ,

//                   origin:
//                                          Origin
//                                       ,

//                   scale:
//                                          Option<Scale>
//                                       ,

//                   offset:
//                                          Option<Scale>
//                                       ,

//                   decay:
//                                          Option<f64>
//                                       ,

//                   multi_value_mode:
//                                          Option<MultiValueMode>


//           }

//           impl GaussFunc {

//                   pub fn with_scale<T: Into<Scale>>(mut self, value: T) -> Self {
//                       self.scale = Some(value.into());
//                       self
//                   }

//                   pub fn with_offset<T: Into<Scale>>(mut self, value: T) -> Self {
//                       self.offset = Some(value.into());
//                       self
//                   }

//                   pub fn with_decay<T: Into<f64>>(mut self, value: T) -> Self {
//                       self.decay = Some(value.into());
//                       self
//                   }

//                   pub fn with_multi_value_mode<T: Into<MultiValueMode>>(mut self, value: T) -> Self {
//                       self.multi_value_mode = Some(value.into());
//                       self
//                   }


//               #[allow(dead_code, unused_variables)]
//               fn add_optionals(&self, m: &mut BTreeMap<String, Json>) {

//                       optional_add!(self, m, self.scale, "scale");

//                       optional_add!(self, m, self.offset, "offset");

//                       optional_add!(self, m, self.decay, "decay");

//                       optional_add!(self, m, self.multi_value_mode, "multi_value_mode");

//               }

//               #[allow(dead_code, unused_variables)]
//               fn add_core_optionals(&self, m: &mut BTreeMap<String, Json>) {

//               }

//               pub fn build(self) -> Func {
//                   Func::Gauss(self)
//               }
//           }

// decay_func_json_impl!(GaussFunc);

// TODO - implementation (definition moved to top)
// impl Function {
//     pub fn new(function: Func) -> Function {
//         Function {
//             filter:   None,
//             function: function,
//             weight:   None
//         }
//     }

//     pub fn with_filter(mut self, filter: Filter) -> Function {
//         self.filter = Some(filter);
//         self
//     }

//     pub fn with_weight(mut self, weight: f64) -> Function {
//         self.weight = Some(weight);
//         self
//     }
// }

#[cfg(test)]
mod tests {
    extern crate rustc_serialize;

    use rustc_serialize::json::ToJson;

    use super::{Flags, SimpleQueryStringFlags, TermsQueryLookup, Query};

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
}
