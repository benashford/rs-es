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

//! Specialised queries

use serde_json::Value;

use ::json::ShouldSkip;

use super::{MinimumShouldMatch, Query};

/// More like this query
#[derive(Debug, Default, Serialize)]
pub struct MoreLikeThisQuery {
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    fields: Option<Vec<String>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    like_text: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    ids: Option<Vec<String>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    docs: Option<Vec<Doc>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    max_query_terms: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    min_term_freq: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    min_doc_freq: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    max_doc_freq: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    min_word_length: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    max_word_length: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    stop_words: Option<Vec<String>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    analyzer: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    minimum_should_match: Option<MinimumShouldMatch>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    boost_terms: Option<f64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    include: Option<bool>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    boost: Option<f64>
}

impl Query {
    pub fn build_more_like_this() -> MoreLikeThisQuery {
        Default::default()
    }
}

impl MoreLikeThisQuery {
    add_field!(with_fields, fields, Vec<String>);
    add_field!(with_like_text, like_text, String);
    add_field!(with_ids, ids, Vec<String>);
    add_field!(with_docs, docs, Vec<Doc>);
    add_field!(with_max_query_terms, max_query_terms, u64);
    add_field!(with_min_term_freq, min_term_freq, u64);
    add_field!(with_min_doc_freq, min_doc_freq, u64);
    add_field!(with_max_doc_freq, max_doc_freq, u64);
    add_field!(with_min_word_length, min_word_length, u64);
    add_field!(with_max_word_length, max_word_length, u64);
    add_field!(with_stop_words, stop_words, Vec<String>);
    add_field!(with_analyzer, analyzer, String);
    add_field!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_field!(with_boost_terms, boost_terms, f64);
    add_field!(with_include, include, bool);
    add_field!(with_boost, boost, f64);

    build!(MoreLikeThis);
}

// A document can be provided as an example
#[derive(Debug, Serialize)]
pub struct Doc {
    #[serde(rename="_index")]
    index:    String,
    #[serde(rename="_type")]
    doc_type: String,
    // TODO - consider generifying this option
    #[serde(skip_serializing_if="ShouldSkip::should_skip", rename="doc")]
    doc:      Option<Value>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip", rename="_id")]
    id:       Option<String>
}

impl Doc {
    pub fn from_doc<A, B>(index: A, doc_type: B, doc: Value) -> Doc
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
