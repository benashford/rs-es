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

use std::collections::BTreeMap;

use rustc_serialize::json::{Json, ToJson};

use super::{MinimumShouldMatch, Query};

/// More like this query
#[derive(Debug, Default)]
pub struct MoreLikeThisQuery {
    fields: Option<Vec<String>>,
    like_text: Option<String>,
    ids: Option<Vec<String>>,
    docs: Option<Vec<Doc>>,
    max_query_terms: Option<u64>,
    min_term_freq: Option<u64>,
    min_doc_freq: Option<u64>,
    max_doc_freq: Option<u64>,
    min_word_length: Option<u64>,
    max_word_length: Option<u64>,
    stop_words: Option<Vec<String>>,
    analyzer: Option<String>,
    minimum_should_match: Option<MinimumShouldMatch>,
    boost_terms: Option<f64>,
    include: Option<bool>,
    boost: Option<f64>
}

impl Query {
    pub fn build_more_like_this() -> MoreLikeThisQuery {
        Default::default()
    }
}

impl MoreLikeThisQuery {
    add_option!(with_fields, fields, Vec<String>);
    add_option!(with_like_text, like_text, String);
    add_option!(with_ids, ids, Vec<String>);
    add_option!(with_docs, docs, Vec<Doc>);
    add_option!(with_max_query_terms, max_query_terms, u64);
    add_option!(with_min_term_freq, min_term_freq, u64);
    add_option!(with_min_doc_freq, min_doc_freq, u64);
    add_option!(with_max_doc_freq, max_doc_freq, u64);
    add_option!(with_min_word_length, min_word_length, u64);
    add_option!(with_max_word_length, max_word_length, u64);
    add_option!(with_stop_words, stop_words, Vec<String>);
    add_option!(with_analyzer, analyzer, String);
    add_option!(with_minimum_should_match, minimum_should_match, MinimumShouldMatch);
    add_option!(with_boost_terms, boost_terms, f64);
    add_option!(with_include, include, bool);
    add_option!(with_boost, boost, f64);

    build!(MoreLikeThis);
}

impl ToJson for MoreLikeThisQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        optional_add!(self, d, fields);
        optional_add!(self, d, like_text);
        optional_add!(self, d, ids);
        optional_add!(self, d, docs);
        optional_add!(self, d, max_query_terms);
        optional_add!(self, d, min_term_freq);
        optional_add!(self, d, min_doc_freq);
        optional_add!(self, d, max_doc_freq);
        optional_add!(self, d, min_word_length);
        optional_add!(self, d, max_word_length);
        optional_add!(self, d, stop_words);
        optional_add!(self, d, analyzer);
        optional_add!(self, d, minimum_should_match);
        optional_add!(self, d, boost_terms);
        optional_add!(self, d, include);
        optional_add!(self, d, boost);
        Json::Object(d)
    }
}

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
