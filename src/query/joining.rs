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

//! Joining queries

use std::collections::BTreeMap;

use rustc_serialize::json::{Json, ToJson};

use super::{ScoreMode, Query};

/// Nested query
#[derive(Debug, Default)]
pub struct NestedQuery {
    path: String,
    score_mode: Option<ScoreMode>,
    query: Query
}

impl Query {
    pub fn build_nested<A, B>(path: A, query: B) -> NestedQuery
        where A: Into<String>,
              B: Into<Query> {
        NestedQuery {
            path: path.into(),
            query: query.into(),
            ..Default::default()
        }
    }
}

impl NestedQuery {
    add_option!(with_score_mode, score_mode, ScoreMode);

    build!(Nested);
}

impl ToJson for NestedQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("path".to_owned(), self.path.to_json());
        d.insert("query".to_owned(), self.query.to_json());
        optional_add!(self, d, score_mode);
        Json::Object(d)
    }
}

/// Has Child query
#[derive(Debug, Default)]
pub struct HasChildQuery {
    doc_type: String,
    query: Query,
    score_mode: Option<ScoreMode>,
    min_children: Option<u64>,
    max_children: Option<u64>
}

/// Has Parent query
#[derive(Debug, Default)]
pub struct HasParentQuery {
    parent_type: String,
    query: Query,
    score_mode: Option<ScoreMode>
}

impl Query {
    pub fn build_has_child<A, B>(doc_type: A, query: B) -> HasChildQuery
        where A: Into<String>,
              B: Into<Query> {
        HasChildQuery {
            doc_type: doc_type.into(),
            query: query.into(),
            ..Default::default()
        }
    }

    pub fn build_has_parent<A, B>(parent_type: A, query: B) -> HasParentQuery
        where A: Into<String>,
              B: Into<Query> {
        HasParentQuery {
            parent_type: parent_type.into(),
            query: query.into(),
            ..Default::default()
        }
    }
}

impl HasChildQuery {
    add_option!(with_score_mode, score_mode, ScoreMode);
    add_option!(with_min_children, min_children, u64);
    add_option!(with_max_children, max_children, u64);

    build!(HasChild);
}

impl HasParentQuery {
    add_option!(with_score_mode, score_mode, ScoreMode);

    build!(HasParent);
}

impl ToJson for HasChildQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("type".to_owned(), self.doc_type.to_json());
        d.insert("query".to_owned(), self.query.to_json());
        optional_add!(self, d, score_mode);
        optional_add!(self, d, min_children);
        optional_add!(self, d, max_children);
        Json::Object(d)
    }
}

impl ToJson for HasParentQuery {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("parent_type".to_owned(), self.parent_type.to_json());
        d.insert("query".to_owned(), self.query.to_json());
        optional_add!(self, d, score_mode);
        Json::Object(d)
    }
}
