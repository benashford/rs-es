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

//! Specific Term level queries

use rustc_serialize::json::{Json, ToJson};

use serde::{Serialize, Serializer};

use ::json::{NoOuter, ShouldSkip};
use ::units::{JsonPotential, JsonVal, OneOrMany};

use super::{Flags, Fuzziness, Query};
use super::common::FieldBasedQuery;

/// Values of the rewrite option used by multi-term queries
#[derive(Debug)]
pub enum Rewrite {
    ConstantScoreAuto,
    ScoringBoolean,
    ConstantScoreBoolean,
    ConstantScoreFilter,
    TopTerms(i64),
    TopTermsBoost(i64),
    TopTermsBlendedFreqs(i64),
}

impl ToJson for Rewrite {
    fn to_json(&self) -> Json {
        match self {
            &Rewrite::ConstantScoreAuto => "constant_score_auto".to_json(),
            &Rewrite::ScoringBoolean => "scoring_boolean".to_json(),
            &Rewrite::ConstantScoreBoolean => "constant_score_boolean".to_json(),
            &Rewrite::ConstantScoreFilter => "constant_score_filter".to_json(),
            &Rewrite::TopTerms(n) => format!("top_terms_{}", n).to_json(),
            &Rewrite::TopTermsBoost(n) => format!("top_terms_boost_{}", n).to_json(),
            &Rewrite::TopTermsBlendedFreqs(n) => format!("top_terms_blended_freqs_{}", n).to_json()
        }
    }
}

/// Term query
#[derive(Debug, Default, Serialize)]
pub struct TermQueryInner {
    value: JsonVal,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    boost: Option<f64>
}

impl TermQueryInner {
    fn new(value: JsonVal) -> Self {
        TermQueryInner {
            value: value,
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TermQuery(FieldBasedQuery<TermQueryInner, NoOuter>);

impl Query {
    pub fn build_term<A, B>(field: A, value: B) -> TermQuery
        where A: Into<String>,
              B: Into<JsonVal> {
        TermQuery(FieldBasedQuery::new(field.into(), TermQueryInner::new(value.into()), NoOuter))
    }
}

impl TermQuery {
    add_inner_option!(with_boost, boost, f64);

    build!(Term);
}

// Terms query
/// Terms Query Lookup
#[derive(Debug, Default, Serialize)]
pub struct TermsQueryLookup {
    id: JsonVal,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    index: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    doc_type: Option<String>,
    path: String,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
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

// TODO - deprecated
// impl ToJson for TermsQueryLookup {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("id".to_owned(), self.id.to_json());
//         d.insert("path".to_owned(), self.path.to_json());
//         optional_add!(self, d, index);
//         optional_add!(self, d, doc_type, "type");
//         optional_add!(self, d, routing);
//         Json::Object(d)
//     }
// }

/// TermsQueryIn
#[derive(Debug)]
pub enum TermsQueryIn {
    /// A `Vec` of values
    Values(Vec<JsonVal>),

    /// An indirect reference to another document
    Lookup(TermsQueryLookup)
}

// TODO - if this looks useful it can be extracted into a macro
impl Serialize for TermsQueryIn {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        match self {
            &TermsQueryIn::Values(ref q) => q.serialize(serializer),
            &TermsQueryIn::Lookup(ref q) => q.serialize(serializer)
        }
    }
}

impl Default for TermsQueryIn {
    fn default() -> Self {
        TermsQueryIn::Values(Default::default())
    }
}

// TODO - deprecated
// impl ToJson for TermsQueryIn {
//     fn to_json(&self) -> Json {
//         match self {
//             &TermsQueryIn::Values(ref v) => v.to_json(),
//             &TermsQueryIn::Lookup(ref l) => l.to_json()
//         }
//     }
// }

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
#[derive(Debug, Serialize)]
pub struct TermsQuery(FieldBasedQuery<TermsQueryIn, NoOuter>);

impl Query {
    pub fn build_terms<A>(field: A) -> TermsQuery
        where A: Into<String> {

        TermsQuery(FieldBasedQuery::new(field.into(), Default::default(), NoOuter))
    }
}

impl TermsQuery {
    pub fn with_values<T>(mut self, values: T) -> Self
        where T: Into<TermsQueryIn> {

        self.0.inner = values.into();
        self
    }

    build!(Terms);
}

/// Range query
/// TODO: Check all possible combinations: gt, gte, lte, lt, from, to, include_upper, include_lower
/// and share with other range queries
#[derive(Debug, Default, Serialize)]
pub struct RangeQueryInner {
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    gte: Option<JsonVal>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    gt: Option<JsonVal>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    lte: Option<JsonVal>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    lt: Option<JsonVal>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    boost: Option<f64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    time_zone: Option<String>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    format: Option<String>
}

#[derive(Debug, Serialize)]
pub struct RangeQuery(FieldBasedQuery<RangeQueryInner, NoOuter>);

impl Query {
    pub fn build_range<A>(field: A) -> RangeQuery
        where A: Into<String> {

        RangeQuery(FieldBasedQuery::new(field.into(), Default::default(), NoOuter))
    }
}

impl RangeQuery {
    add_inner_option!(with_gte, gte, JsonVal);
    add_inner_option!(with_gt, gt, JsonVal);
    add_inner_option!(with_lte, lte, JsonVal);
    add_inner_option!(with_lt, lt, JsonVal);
    add_inner_option!(with_boost, boost, f64);
    add_inner_option!(with_time_zone, time_zone, String);
    add_inner_option!(with_format, format, String);

    build!(Range);
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
    //build!(Exists);
}

// impl ToJson for ExistsQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("field".to_owned(), self.field.to_json());
//         Json::Object(d)
//     }
// }

/// Prefix query
#[derive(Debug, Default)]
pub struct PrefixQuery {
    field: String,
    value: String,
    boost: Option<f64>,
    rewrite: Option<Rewrite>
}

impl Query {
    pub fn build_prefix<A, B>(field: A, value: B) -> PrefixQuery
        where A: Into<String>,
              B: Into<String> {
        PrefixQuery {
            field: field.into(),
            value: value.into(),
            ..Default::default()
        }
    }
}

impl PrefixQuery {
    add_option!(with_boost, boost, f64);
    add_option!(with_rewrite, rewrite, Rewrite);

    //build!(Prefix);
}

// impl ToJson for PrefixQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         let mut inner = BTreeMap::new();
//         inner.insert("value".to_owned(), self.value.to_json());
//         optional_add!(self, inner, boost);
//         optional_add!(self, inner, rewrite);
//         d.insert(self.field.clone(), Json::Object(inner));
//         Json::Object(d)
//     }
// }

/// Wildcard query
#[derive(Debug, Default)]
pub struct WildcardQuery {
    field: String,
    value: String,
    boost: Option<f64>,
    rewrite: Option<Rewrite>
}

impl Query {
    pub fn build_wildcard<A, B>(field: A, value: B) -> WildcardQuery
        where A: Into<String>,
              B: Into<String> {
        WildcardQuery {
            field: field.into(),
            value: value.into(),
            ..Default::default()
        }
    }
}

impl WildcardQuery {
    add_option!(with_boost, boost, f64);
    add_option!(with_rewrite, rewrite, Rewrite);

    //build!(Wildcard);
}

// impl ToJson for WildcardQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         let mut inner = BTreeMap::new();
//         inner.insert("value".to_owned(), self.value.to_json());
//         optional_add!(self, inner, boost);
//         optional_add!(self, inner, rewrite);
//         d.insert(self.field.clone(), Json::Object(inner));
//         Json::Object(d)
//     }
// }

// Regexp query
/// Flags for the Regexp query
#[derive(Debug)]
pub enum RegexpQueryFlags {
    All,
    Anystring,
    Complement,
    Empty,
    Intersection,
    Interval,
    None
}

impl AsRef<str> for RegexpQueryFlags {
    fn as_ref(&self) -> &str {
        match self {
            &RegexpQueryFlags::All => "ALL",
            &RegexpQueryFlags::Anystring => "ANYSTRING",
            &RegexpQueryFlags::Complement => "COMPLEMENT",
            &RegexpQueryFlags::Empty => "EMPTY",
            &RegexpQueryFlags::Intersection => "INTERSECTION",
            &RegexpQueryFlags::Interval => "INTERVAL",
            &RegexpQueryFlags::None => "NONE"
        }
    }
}

/// Regexp query
#[derive(Debug, Default)]
pub struct RegexpQuery {
    field: String,
    value: String,
    boost: Option<f64>,
    flags: Option<Flags<RegexpQueryFlags>>,
    max_determined_states: Option<u64>
}

impl Query {
    pub fn build_query<A, B>(field: A, value: B) -> RegexpQuery
        where A: Into<String>,
              B: Into<String> {
        RegexpQuery {
            field: field.into(),
            value: value.into(),
            ..Default::default()
        }
    }
}

impl RegexpQuery {
    add_option!(with_boost, boost, f64);
    add_option!(with_flags, flags, Flags<RegexpQueryFlags>);
    add_option!(with_max_determined_states, max_determined_states, u64);

    //build!(Regexp);
}

// impl ToJson for RegexpQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         let mut inner = BTreeMap::new();

//         inner.insert("value".to_owned(), self.value.to_json());
//         optional_add!(self, inner, boost);
//         optional_add!(self, inner, flags);
//         optional_add!(self, inner, max_determined_states);

//         d.insert(self.field.clone(), Json::Object(inner));
//         Json::Object(d)
//     }
// }

/// Fuzzy query
#[derive(Debug, Default)]
pub struct FuzzyQuery {
    field: String,
    value: String,
    boost: Option<f64>,
    fuzziness: Option<Fuzziness>,
    prefix_length: Option<u64>,
    max_expansions: Option<u64>
}

impl Query {
    pub fn build_fuzzy<A, B>(field: A, value: B) -> FuzzyQuery
        where A: Into<String>,
              B: Into<String> {
        FuzzyQuery {
            field: field.into(),
            value: value.into(),
            ..Default::default()
        }
    }
}

impl FuzzyQuery {
    add_option!(with_boost, boost, f64);
    add_option!(with_fuzziness, fuzziness, Fuzziness);
    add_option!(with_prefix_length, prefix_length, u64);
    add_option!(with_max_expansions, max_expansions, u64);

    //build!(Fuzzy);
}

// impl ToJson for FuzzyQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         let mut inner = BTreeMap::new();
//         inner.insert("value".to_owned(), self.value.to_json());
//         optional_add!(self, inner, boost);
//         optional_add!(self, inner, fuzziness);
//         optional_add!(self, inner, prefix_length);
//         optional_add!(self, inner, max_expansions);
//         d.insert(self.field.clone(), Json::Object(inner));
//         Json::Object(d)
//     }
// }

/// Type query
#[derive(Debug)]
pub struct TypeQuery {
    value: String
}

impl Query {
    pub fn build_type<A>(value: A) -> TypeQuery
        where A: Into<String> {

        TypeQuery {
            value: value.into()
        }
    }
}

impl TypeQuery {
    //build!(Type);
}

// impl ToJson for TypeQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("value".to_owned(), self.value.to_json());
//         Json::Object(d)
//     }
// }

/// Ids query
#[derive(Debug, Default)]
pub struct IdsQuery {
    doc_type: Option<OneOrMany<String>>,
    values: Vec<JsonVal>
}

impl Query {
    pub fn build_ids<A>(values: A) -> IdsQuery
        where A: Into<Vec<JsonVal>> {

        IdsQuery {
            values: values.into(),
            ..Default::default()
        }
    }
}

impl IdsQuery {
    add_option!(with_type, doc_type, OneOrMany<String>);

    //build!(Ids);
}

// impl ToJson for IdsQuery {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("values".to_owned(), self.values.to_json());
//         optional_add!(self, d, doc_type, "type");
//         Json::Object(d)
//     }
// }
