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

//! Implementations of full-text ES queries

use ::units::JsonVal;

use serde::{Serialize, Serializer};

use ::json::{NoOuter, ShouldSkip};

use super::{Flags, Fuzziness, MinimumShouldMatch, Query};
use super::common::FieldBasedQuery;

/// MatchType - the type of Match query
#[derive(Debug)]
pub enum MatchType {
    Boolean,
    Phrase,
    PhrasePrefix
}

impl Serialize for MatchType {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        use self::MatchType::*;
        match self {
            &Boolean => "boolean",
            &Phrase => "phrase",
            &PhrasePrefix => "phrase_prefix"
        }.serialize(serializer)
    }
}

/// Zero Terms Query

#[derive(Debug)]
pub enum ZeroTermsQuery {
    None,
    All
}

impl Serialize for ZeroTermsQuery {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        match self {
            &ZeroTermsQuery::None => "none",
            &ZeroTermsQuery::All => "all"
        }.serialize(serializer)
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

// TODO - deprecated
// impl ToJson for MatchQueryType {
//     fn to_json(&self) -> Json {
//         match self {
//             &MatchQueryType::BestFields => "best_fields",
//             &MatchQueryType::MostFields => "most_fields",
//             &MatchQueryType::CrossFields => "cross_fields",
//             &MatchQueryType::Phrase => "phrase",
//             &MatchQueryType::PhrasePrefix => "phrase_prefix",
//         }.to_json()
//     }
// }

/// Match query

#[derive(Debug, Serialize)]
pub struct MatchQuery(FieldBasedQuery<MatchQueryInner, NoOuter>);

#[derive(Debug, Default, Serialize)]
pub struct MatchQueryInner {
    query: JsonVal,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    match_type: Option<MatchType>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    cutoff_frequency: Option<f64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    lenient: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    analyzer: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    boost: Option<f64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    operator: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    minimum_should_match: Option<MinimumShouldMatch>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    fuzziness: Option<Fuzziness>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    prefix_length: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    max_expansions: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    rewrite: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    zero_terms_query: Option<ZeroTermsQuery>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    slop: Option<i64>
}

impl Query {
    pub fn build_match<A, B>(field: A, query: B) -> MatchQuery
        where A: Into<String>,
              B: Into<JsonVal> {
        MatchQuery(FieldBasedQuery::new(field.into(),
                                        MatchQueryInner {
                                            query: query.into(),
                                            ..Default::default()
                                        },
                                        NoOuter))
    }
}

impl MatchQuery {
    add_inner_option!(with_type, match_type, MatchType);
    add_inner_option!(with_cutoff_frequency, cutoff_frequency, f64);
    add_inner_option!(with_lenient, lenient, bool);
    add_inner_option!(with_analyzer, analyzer, String);
    add_inner_option!(with_boost, boost, f64);
    add_inner_option!(with_operator, operator, String);
    add_inner_option!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_inner_option!(with_fuzziness, fuzziness, Fuzziness);
    add_inner_option!(with_prefix_length, prefix_length, u64);
    add_inner_option!(with_max_expansions, max_expansions, u64);
    add_inner_option!(with_rewrite, rewrite, String);
    add_inner_option!(with_zero_terms_query, zero_terms_query, ZeroTermsQuery);
    add_inner_option!(with_slop, slop, i64);

    build!(Match);
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

    //build!(MultiMatch);
}

// TODO - deprecated
// impl ToJson for MultiMatchQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("fields".to_owned(), self.fields.to_json());
//         d.insert("query".to_owned(), self.query.to_json());
//         optional_add!(self, d, match_type);
//         optional_add!(self, d, tie_breaker);
//         optional_add!(self, d, analyzer);
//         optional_add!(self, d, boost);
//         optional_add!(self, d, operator);
//         optional_add!(self, d, minimum_should_match);
//         optional_add!(self, d, fuzziness);
//         optional_add!(self, d, prefix_length);
//         optional_add!(self, d, max_expansions);
//         optional_add!(self, d, rewrite);
//         optional_add!(self, d, zero_terms_query);
//         optional_add!(self, d, cutoff_frequency);
//         optional_add!(self, d, slop);
//         Json::Object(d)
//     }
// }

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

    //build!(Common);
}

// TODO - deprecated
// impl ToJson for CommonQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         let mut inner = BTreeMap::new();
//         inner.insert("query".to_owned(), self.query.to_json());
//         optional_add!(self, inner, cutoff_frequency);
//         optional_add!(self, inner, low_freq_operator);
//         optional_add!(self, inner, high_freq_operator);
//         optional_add!(self, inner, minimum_should_match);
//         optional_add!(self, inner, boost);
//         optional_add!(self, inner, analyzer);
//         optional_add!(self, inner, disable_coord);
//         d.insert("body".to_owned(), Json::Object(inner));
//         Json::Object(d)
//     }
// }

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

    //build!(QueryString);
}

// TODO - deprecated
// impl ToJson for QueryStringQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("query".to_owned(), self.query.to_json());
//         optional_add!(self, d, default_field);
//         optional_add!(self, d, fields);
//         optional_add!(self, d, default_operator);
//         optional_add!(self, d, analyzer);
//         optional_add!(self, d, allow_leading_wildcard);
//         optional_add!(self, d, lowercase_expanded_terms);
//         optional_add!(self, d, enable_position_increments);
//         optional_add!(self, d, fuzzy_max_expansions);
//         optional_add!(self, d, fuzziness);
//         optional_add!(self, d, fuzzy_prefix_length);
//         optional_add!(self, d, phrase_slop);
//         optional_add!(self, d, boost);
//         optional_add!(self, d, analyze_wildcard);
//         optional_add!(self, d, auto_generate_phrase_queries);
//         optional_add!(self, d, max_determined_states);
//         optional_add!(self, d, minimum_should_match);
//         optional_add!(self, d, lenient);
//         optional_add!(self, d, locale);
//         optional_add!(self, d, time_zone);
//         optional_add!(self, d, use_dis_max);
//         Json::Object(d)
//     }
// }

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

    //build!(SimpleQueryString);
}

// TODO - deprecated
// impl ToJson for SimpleQueryStringQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("query".to_owned(), self.query.to_json());
//         optional_add!(self, d, fields);
//         optional_add!(self, d, analyzer);
//         optional_add!(self, d, flags);
//         optional_add!(self, d, lowercase_expanded_terms);
//         optional_add!(self, d, analyze_wildcard);
//         optional_add!(self, d, locale);
//         optional_add!(self, d, lenient);
//         optional_add!(self, d, minimum_should_match);
//         Json::Object(d)
//     }
// }
