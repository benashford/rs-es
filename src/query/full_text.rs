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
use ::operations::search::highlight::Highlight;

/// MatchType - the type of Match query
#[derive(Debug, Clone)]
pub enum MatchType {
    Boolean,
    Phrase,
    PhrasePrefix
}

impl Serialize for MatchType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
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
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {

        match self {
            &ZeroTermsQuery::None => "none",
            &ZeroTermsQuery::All => "all"
        }.serialize(serializer)
    }
}

/// MatchQueryType - the type of the multi Match Query
#[derive(Debug, Clone)]
pub enum MatchQueryType {
    BestFields,
    MostFields,
    CrossFields,
    Phrase,
    PhrasePrefix,
}

impl Serialize for MatchQueryType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        use self::MatchQueryType::*;
        match self {
            &BestFields => "best_fields",
            &MostFields => "most_fields",
            &CrossFields => "cross_fields",
            &Phrase => "phrase",
            &PhrasePrefix => "phrase_prefix"
        }.serialize(serializer)
    }
}

/// Match query

#[derive(Debug, Serialize)]
pub struct MatchQuery(FieldBasedQuery<MatchQueryInner, NoOuter>);

#[derive(Debug, Default, Serialize)]
pub struct MatchQueryInner {
    query: JsonVal,
    #[serde(skip_serializing_if="ShouldSkip::should_skip", rename="type")]
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
    slop: Option<i64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    highlight: Option<Highlight>
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
    add_inner_field!(with_type, match_type, MatchType);
    add_inner_field!(with_cutoff_frequency, cutoff_frequency, f64);
    add_inner_field!(with_lenient, lenient, bool);
    add_inner_field!(with_analyzer, analyzer, String);
    add_inner_field!(with_boost, boost, f64);
    add_inner_field!(with_operator, operator, String);
    add_inner_field!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_inner_field!(with_fuzziness, fuzziness, Fuzziness);
    add_inner_field!(with_prefix_length, prefix_length, u64);
    add_inner_field!(with_max_expansions, max_expansions, u64);
    add_inner_field!(with_rewrite, rewrite, String);
    add_inner_field!(with_zero_terms_query, zero_terms_query, ZeroTermsQuery);
    add_inner_field!(with_slop, slop, i64);
    add_inner_field!(with_highlight, highlight, Highlight);

    build!(Match);
}

/// Multi Match Query
#[derive(Debug, Default, Serialize)]
pub struct MultiMatchQuery {
    fields: Vec<String>,
    query: JsonVal,
    #[serde(skip_serializing_if="ShouldSkip::should_skip", rename="type")]
    match_type: Option<MatchQueryType>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    tie_breaker: Option<f64>,
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
    cutoff_frequency: Option<f64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    slop: Option<i64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    highlight: Option<Highlight>
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
    add_field!(with_type, match_type, MatchQueryType);
    add_field!(with_tie_breaker, tie_breaker, f64);
    add_field!(with_analyzer, analyzer, String);
    add_field!(with_boost, boost, f64);
    add_field!(with_operator, operator, String);
    add_field!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_field!(with_fuzziness, fuzziness, Fuzziness);
    add_field!(with_prefix_length, prefix_length, u64);
    add_field!(with_max_expansions, max_expansions, u64);
    add_field!(with_rewrite, rewrite, String);
    add_field!(with_zero_terms_query, zero_terms_query, ZeroTermsQuery);
    add_field!(with_cutoff_frequency, cutoff_frequency, f64);
    add_field!(with_slop, slop, i64);
    add_field!(with_highlight, highlight, Highlight);

    build!(MultiMatch);
}

/// Common terms query
#[derive(Debug, Serialize)]
pub struct CommonQuery(FieldBasedQuery<CommonQueryInner, NoOuter>);

#[derive(Debug, Default, Serialize)]
pub struct CommonQueryInner {
    query: JsonVal,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    cutoff_frequency: Option<f64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    low_freq_operator: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    high_freq_operator: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    minimum_should_match: Option<MinimumShouldMatch>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    boost: Option<f64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    analyzer: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    disable_coord: Option<bool>
}

impl Query {
    pub fn build_common<A>(query: A) -> CommonQuery
        where A: Into<JsonVal> {

        CommonQuery(FieldBasedQuery::new("body".to_owned(),
                                         CommonQueryInner {
                                             query: query.into(),
                                             ..Default::default()
                                         },
                                         NoOuter))
    }
}

impl CommonQuery {
    add_inner_field!(with_cutoff_frequency, cutoff_frequency, f64);
    add_inner_field!(with_low_freq_operator, low_freq_operator, String);
    add_inner_field!(with_high_freq_operator, high_freq_operator, String);
    add_inner_field!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_inner_field!(with_boost, boost, f64);
    add_inner_field!(with_analyzer, analyzer, String);
    add_inner_field!(with_disable_coord, disable_coord, bool);

    build!(Common);
}

/// Query string query
#[derive(Debug, Default, Serialize)]
pub struct QueryStringQuery {
    query: String,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    default_field: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    fields: Option<Vec<String>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    default_operator: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    analyzer: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    allow_leading_wildcard: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    lowercase_expanded_terms: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    enable_position_increments: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    fuzzy_max_expansions: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    fuzziness: Option<Fuzziness>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    fuzzy_prefix_length: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    phrase_slop: Option<i64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    boost: Option<f64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    analyze_wildcard: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    auto_generate_phrase_queries: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    max_determined_states: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    minimum_should_match: Option<MinimumShouldMatch>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    lenient: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    locale: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    time_zone: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
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
    add_field!(with_default_field, default_field, String);
    add_field!(with_fields, fields, Vec<String>);
    add_field!(with_default_operator, default_operator, String);
    add_field!(with_analyzer, analyzer, String);
    add_field!(with_allow_leading_wildcard, allow_leading_wildcard, bool);
    add_field!(with_lowercase_expanded_terms, lowercase_expanded_terms, bool);
    add_field!(with_enable_position_increments, enable_position_increments, bool);
    add_field!(with_fuzzy_max_expansions, fuzzy_max_expansions, u64);
    add_field!(with_fuzziness, fuzziness, Fuzziness);
    add_field!(with_fuzzy_prefix_length, fuzzy_prefix_length, u64);
    add_field!(with_phrase_slop, phrase_slop, i64);
    add_field!(with_boost, boost, f64);
    add_field!(with_analyze_wildcard, analyze_wildcard, bool);
    add_field!(with_auto_generate_phrase_queries, auto_generate_phrase_queries, bool);
    add_field!(with_max_determined_states, max_determined_states, u64);
    add_field!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_field!(with_lenient, lenient, bool);
    add_field!(with_locale, locale, String);
    add_field!(with_time_zone, time_zone, String);
    add_field!(with_use_dis_max, use_dis_max, bool);

    build!(QueryString);
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
#[derive(Debug, Default, Serialize)]
pub struct SimpleQueryStringQuery {
    query: String,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    fields: Option<Vec<String>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    default_operator: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    analyzer: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    flags: Option<Flags<SimpleQueryStringFlags>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    lowercase_expanded_terms: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    analyze_wildcard: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    locale: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    lenient: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
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
    add_field!(with_fields, fields, Vec<String>);
    add_field!(with_default_operator, default_operator, String);
    add_field!(with_analyzer, analyzer, String);
    add_field!(with_flags, flags, Flags<SimpleQueryStringFlags>);
    add_field!(with_lowercase_expanded_terms, lowercase_expanded_terms, bool);
    add_field!(with_analyze_wildcard, analyze_wildcard, bool);
    add_field!(with_locale, locale, String);
    add_field!(with_lenient, lenient, bool);
    add_field!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);

    build!(SimpleQueryString);
}
