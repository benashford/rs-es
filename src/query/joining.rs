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

use ::json::ShouldSkip;
use ::serde_json::Value;

use super::{ScoreMode, Query};

/// Nested query
#[derive(Debug, Default, Serialize)]
pub struct NestedQuery {
    path: String,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
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
    add_field!(with_score_mode, score_mode, ScoreMode);

    build!(Nested);
}

/// Has Child query
#[derive(Debug, Default, Serialize)]
pub struct HasChildQuery {
    #[serde(rename="type")]
    doc_type: String,
    query: Query,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    score_mode: Option<ScoreMode>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    min_children: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    max_children: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    inner_hits: Option<Value>
}

/// Has Parent query
#[derive(Debug, Default, Serialize)]
pub struct HasParentQuery {
    parent_type: String,
    query: Query,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
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
    add_field!(with_score_mode, score_mode, ScoreMode);
    add_field!(with_min_children, min_children, u64);
    add_field!(with_max_children, max_children, u64);
    add_field!(with_inner_hits, inner_hits, Value);

    build!(HasChild);
}

impl HasParentQuery {
    add_field!(with_score_mode, score_mode, ScoreMode);

    build!(HasParent);
}
