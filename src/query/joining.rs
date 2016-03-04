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

