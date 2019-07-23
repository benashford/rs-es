/*
 * Copyright 2015-2019 Ben Ashford
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

//! Implementations of both Search-by-URI and Search-by-Query operations

pub mod aggregations;
pub mod count;
pub mod highlight;

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;

use reqwest::StatusCode;

use serde::{de::DeserializeOwned, ser::Serializer, Deserialize, Serialize};
use serde_json::Value;

use super::{
    common::{OptionVal, Options},
    format_indexes_and_types, ShardCountResult,
};
use crate::{
    error::EsError,
    json::{FieldBased, NoOuter, ShouldSkip},
    query::Query,
    units::{DistanceType, DistanceUnit, Duration, JsonVal, Location, OneOrMany},
    util::StrJoin,
    Client, EsResponse,
};

use self::aggregations::AggregationsResult;
use self::highlight::HighlightResult;

/// Representing a search-by-uri option
#[derive(Debug)]
pub struct SearchURIOperation<'a, 'b> {
    client: &'a mut Client,
    indexes: &'b [&'b str],
    doc_types: &'b [&'b str],
    options: Options<'b>,
}

/// Options for the various search_type parameters
#[derive(Debug)]
pub enum SearchType {
    DFSQueryThenFetch,
    DFSQueryAndFetch,
    QueryThenFetch,
    QueryAndFetch,
}

impl ToString for SearchType {
    fn to_string(&self) -> String {
        match self {
            SearchType::DFSQueryThenFetch => "dfs_query_then_fetch",
            SearchType::DFSQueryAndFetch => "dfs_query_and_fetch",
            SearchType::QueryThenFetch => "query_then_fetch",
            SearchType::QueryAndFetch => "query_and_fetch",
        }
        .to_owned()
    }
}

/// Order of a sort
#[derive(Debug)]
pub enum Order {
    Asc,
    Desc,
}

impl ToString for Order {
    fn to_string(&self) -> String {
        match self {
            Order::Asc => "asc",
            Order::Desc => "desc",
        }
        .to_owned()
    }
}

impl Serialize for Order {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

/// The (Sort mode option)[https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-sort.html#_sort_mode_option].
#[derive(Debug)]
pub enum Mode {
    Min,
    Max,
    Sum,
    Avg,
}

impl Serialize for Mode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Mode::Min => "min",
            Mode::Max => "max",
            Mode::Sum => "sum",
            Mode::Avg => "avg",
        }
        .serialize(serializer)
    }
}

/// Options for handling (missing values)[https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-sort.html#_missing_values]
#[derive(Debug)]
pub enum Missing {
    First,
    Last,
    Custom(String),
}

impl Serialize for Missing {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Missing::First => "first".serialize(serializer),
            Missing::Last => "last".serialize(serializer),
            Missing::Custom(ref s) => s.serialize(serializer),
        }
    }
}

/// Convert anything that can be converted into a `String` into a
/// `Missing::Custom` value
impl<S: Into<String>> From<S> for Missing {
    fn from(from: S) -> Missing {
        Missing::Custom(from.into())
    }
}

/// Representing sort options for a specific field, can be combined with others
/// to produce the full sort clause
#[derive(Debug, Serialize)]
pub struct SortField(FieldBased<String, SortFieldInner, NoOuter>);

#[derive(Debug, Default, Serialize)]
pub struct SortFieldInner {
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    order: Option<Order>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    mode: Option<Mode>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    nested_path: Option<String>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    nested_filter: Option<Query>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    missing: Option<Missing>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    unmapped_type: Option<String>,
}

impl SortField {
    /// Create a `SortField` for a given `field` and `order`
    pub fn new<S: Into<String>>(field: S, order: Option<Order>) -> Self {
        SortField(FieldBased::new(
            field.into(),
            SortFieldInner {
                order,
                ..Default::default()
            },
            NoOuter,
        ))
    }

    add_inner_field!(with_mode, mode, Mode);
    add_inner_field!(with_nested_path, nested_path, String);
    add_inner_field!(with_nested_filter, nested_filter, Query);
    add_inner_field!(with_missing, missing, Missing);
    add_inner_field!(with_unmapped_type, unmapped_type, String);

    pub fn build(self) -> SortBy {
        SortBy::Field(self)
    }
}

impl ToString for SortField {
    fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&self.0.field);
        if let Some(ref order) = self.0.inner.order {
            s.push_str(":");
            // TODO - find less clumsy way of implementing the following
            // line
            s.push_str(&order.to_string());
        }
        s
    }
}

/// Representing sort options for sort by geodistance
// TODO - fix structure to represent reality
#[derive(Debug, Serialize)]
pub struct GeoDistance {
    field: String,
    location: OneOrMany<Location>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    order: Option<Order>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    unit: Option<DistanceUnit>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    mode: Option<Mode>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    distance_type: Option<DistanceType>,
}

impl GeoDistance {
    pub fn new<S>(field: S) -> GeoDistance
    where
        S: Into<String>,
    {
        GeoDistance {
            field: field.into(),
            location: OneOrMany::Many(vec![]),
            order: None,
            unit: None,
            mode: None,
            distance_type: None,
        }
    }

    pub fn with_location<L: Into<Location>>(mut self, location: L) -> Self {
        self.location = OneOrMany::One(location.into());
        self
    }

    pub fn with_locations<L: Into<Location>>(mut self, location: Vec<L>) -> Self {
        self.location = OneOrMany::Many(location.into_iter().map(Into::into).collect());
        self
    }

    add_field!(with_order, order, Order);
    add_field!(with_unit, unit, DistanceUnit);
    add_field!(with_mode, mode, Mode);
    add_field!(with_distance_type, distance_type, DistanceType);

    pub fn build(self) -> SortBy {
        SortBy::Distance(self)
    }
}

/// Representing options for sort by script
// TODO - fix structure
// TODO - there are other 'Script's defined elsewhere, perhaps de-duplicate them
// if it makes sense.
#[derive(Debug, Serialize)]
pub struct Script {
    script: String,
    #[serde(rename = "type")]
    script_type: String,
    params: HashMap<String, JsonVal>,
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    order: Option<Order>,
}

impl Script {
    pub fn new<S, ST>(script: S, script_type: ST) -> Script
    where
        S: Into<String>,
        ST: Into<String>,
    {
        Script {
            script: script.into(),
            script_type: script_type.into(),
            params: HashMap::new(),
            order: None,
        }
    }

    add_field!(with_order, order, Order);

    pub fn add_param<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<JsonVal>,
    {
        self.params.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> SortBy {
        SortBy::Script(self)
    }
}

#[derive(Debug)]
pub enum SortBy {
    Field(SortField),
    Distance(GeoDistance),
    Script(Script),
}

impl Serialize for SortBy {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            SortBy::Field(ref f) => f.serialize(serializer),
            SortBy::Distance(ref d) => d.serialize(serializer),
            SortBy::Script(ref s) => s.serialize(serializer),
        }
    }
}

impl ToString for SortBy {
    fn to_string(&self) -> String {
        match self {
            SortBy::Field(ref field) => field.to_string(),
            _ => panic!("Can only convert field sorting ToString"),
        }
    }
}

/// A full sort clause
#[derive(Debug)]
pub struct Sort {
    fields: Vec<SortBy>,
}

impl Serialize for Sort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.fields.serialize(serializer)
    }
}

impl Sort {
    pub fn new(fields: Vec<SortBy>) -> Self {
        Sort { fields }
    }

    /// Convenience function for a single field default
    pub fn field<S: Into<String>>(fieldname: S) -> Self {
        Sort {
            fields: vec![SortField::new(fieldname, None).build()],
        }
    }

    pub fn field_order<S: Into<String>>(fieldname: S, order: Order) -> Self {
        Sort {
            fields: vec![SortField::new(fieldname, Some(order)).build()],
        }
    }

    pub fn fields<S: Into<String>>(fieldnames: Vec<S>) -> Self {
        Sort {
            fields: fieldnames
                .into_iter()
                .map(|fieldname| SortField::new(fieldname, None).build())
                .collect(),
        }
    }

    pub fn field_orders<S: Into<String>>(fields: Vec<(S, Order)>) -> Self {
        Sort {
            fields: fields
                .into_iter()
                .map(|(fieldname, order)| SortField::new(fieldname, Some(order)).build())
                .collect(),
        }
    }
}

/// Conversion of a `Sort` into an `OptionVal` for use in search-by-URI queries
///
/// ```
/// use rs_es::operations::common::OptionVal;
/// use rs_es::operations::search::{Sort, SortField, Order};
/// let sort = Sort::new(vec![SortField::new("a", Some(Order::Asc)).build(),
///                                     SortField::new("b", None).build()]);
/// let op_val:OptionVal = (&sort).into();
/// assert_eq!("a:asc,b", op_val.0);
/// ```
impl<'a> From<&'a Sort> for OptionVal {
    fn from(from: &'a Sort) -> OptionVal {
        // TODO - stop requiring `to_string` if `AsRef<str>` could be implemented
        // instead
        OptionVal(from.fields.iter().map(ToString::to_string).join(","))
    }
}

impl<'a, 'b> SearchURIOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> SearchURIOperation<'a, 'b> {
        SearchURIOperation {
            client,
            indexes: &[],
            doc_types: &[],
            options: Options::default(),
        }
    }

    pub fn with_indexes(&'b mut self, indexes: &'b [&'b str]) -> &'b mut Self {
        self.indexes = indexes;
        self
    }

    pub fn with_types(&'b mut self, doc_types: &'b [&'b str]) -> &'b mut Self {
        self.doc_types = doc_types;
        self
    }

    pub fn with_query<S: Into<String>>(&'b mut self, qs: S) -> &'b mut Self {
        self.options.push("q", qs.into());
        self
    }

    add_option!(with_df, "df");
    add_option!(with_analyzer, "analyzer");
    add_option!(with_lowercase_expanded_terms, "lowercase_expanded_terms");
    add_option!(with_analyze_wildcard, "analyze_wildcard");
    add_option!(with_default_operator, "default_operator");
    add_option!(with_lenient, "lenient");
    add_option!(with_explain, "explain");
    add_option!(with_source, "_source");
    add_option!(with_highlight, "highlight");
    add_option!(with_version, "version");
    add_option!(with_sort, "sort");
    add_option!(with_routing, "routing");
    add_option!(with_track_scores, "track_scores");
    add_option!(with_timeout, "timeout");
    add_option!(with_terminate_after, "terminate_after");
    add_option!(with_from, "from");
    add_option!(with_size, "size");
    add_option!(with_search_type, "search_type");
    add_option!(with_ignore_unavailable, "ignore_unavailable");
    add_option!(with_allow_no_indices, "allow_no_indices");
    add_option!(with_expand_wildcards, "expand_wildcards");

    #[cfg(not(feature = "es5"))]
    pub fn with_fields(&'b mut self, fields: &[&str]) -> &'b mut Self {
        self.options.push("fields", fields.iter().join(","));
        self
    }

    pub fn send<T>(&'b mut self) -> Result<SearchResult<T>, EsError>
    where
        T: DeserializeOwned,
    {
        let url = format!(
            "/{}/_search{}",
            format_indexes_and_types(&self.indexes, &self.doc_types),
            self.options
        );
        log::info!("Searching with: {}", url);
        let response = self.client.get_op(&url)?;
        match response.status_code() {
            StatusCode::OK => {
                let interim: SearchResultInterim<T> = response.read_response()?;
                Ok(interim.finalize())
            }
            status_code => Err(EsError::EsError(format!(
                "Unexpected status: {}",
                status_code
            ))),
        }
    }
}

/// Options for source filtering
#[derive(Debug)]
pub enum Source<'a> {
    /// Disable source documents
    Off,

    /// Filtering
    Filter(Option<&'a [&'a str]>, Option<&'a [&'a str]>),
}

impl<'a> Serialize for Source<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Source::Off => false.serialize(serializer),
            Source::Filter(incl, excl) => {
                let mut d = BTreeMap::new();
                match incl {
                    Some(val) => {
                        d.insert("include".to_owned(), val);
                    }
                    None => (),
                }
                match excl {
                    Some(val) => {
                        d.insert("exclude".to_owned(), val);
                    }
                    None => (),
                }
                d.serialize(serializer)
            }
        }
    }
}

impl<'a> Source<'a> {
    /// An include-only source filter
    pub fn include(fields: &'a [&'a str]) -> Source<'a> {
        Source::Filter(Some(fields), None)
    }

    /// An exclude-only source filter
    pub fn exclude(fields: &'a [&'a str]) -> Source<'a> {
        Source::Filter(None, Some(fields))
    }

    /// An include and exclude source filter
    pub fn filter(incl: &'a [&'a str], excl: &'a [&'a str]) -> Source<'a> {
        Source::Filter(Some(incl), Some(excl))
    }
}

#[derive(Debug, Default, Serialize)]
struct SearchQueryOperationBody<'b> {
    /// The query
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    query: Option<&'b Query>,

    /// Timeout
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    timeout: Option<&'b str>,

    /// From
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    from: Option<u64>,

    /// Size
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    size: Option<u64>,

    /// Terminate early (marked as experimental in the ES docs)
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    terminate_after: Option<u64>,

    /// Stats groups to which the query belongs
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    stats: Option<Vec<String>>,

    /// Minimum score to use
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    min_score: Option<f64>,

    /// Sort fields
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    sort: Option<&'b Sort>,

    /// Track scores
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    track_scores: Option<bool>,

    /// Source filtering
    #[serde(rename = "_source", skip_serializing_if = "ShouldSkip::should_skip")]
    source: Option<Source<'b>>,

    /// Aggregations
    #[serde(
        rename = "aggregations",
        skip_serializing_if = "ShouldSkip::should_skip"
    )]
    aggs: Option<&'b aggregations::Aggregations<'b>>,

    /// Highlight
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    highlight: Option<&'b highlight::Highlight>,

    /// Version
    #[serde(skip_serializing_if = "ShouldSkip::should_skip")]
    version: Option<bool>,
}

#[derive(Debug)]
pub struct SearchQueryOperation<'a, 'b> {
    /// The HTTP client
    client: &'a mut Client,

    /// The indexes to which this query applies
    indexes: &'b [&'b str],

    /// The types to which the query applies
    doc_types: &'b [&'b str],

    /// Optionals
    options: Options<'b>,

    /// The query body
    body: SearchQueryOperationBody<'b>,
}

impl<'a, 'b> SearchQueryOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> Self {
        SearchQueryOperation {
            client,
            indexes: &[],
            doc_types: &[],
            options: Options::new(),
            body: Default::default(),
        }
    }

    pub fn with_indexes(&mut self, indexes: &'b [&'b str]) -> &mut Self {
        self.indexes = indexes;
        self
    }

    pub fn with_types(&mut self, doc_types: &'b [&'b str]) -> &mut Self {
        self.doc_types = doc_types;
        self
    }

    pub fn with_query(&mut self, query: &'b Query) -> &mut Self {
        self.body.query = Some(query);
        self
    }

    pub fn with_timeout(&mut self, timeout: &'b str) -> &mut Self {
        self.body.timeout = Some(timeout);
        self
    }

    pub fn with_from(&mut self, from: u64) -> &mut Self {
        self.body.from = Some(from);
        self
    }

    pub fn with_size(&mut self, size: u64) -> &mut Self {
        self.body.size = Some(size);
        self
    }

    pub fn with_version(&mut self, version: bool) -> &mut Self {
        self.body.version = Some(version);
        self
    }

    pub fn with_terminate_after(&mut self, terminate_after: u64) -> &mut Self {
        self.body.terminate_after = Some(terminate_after);
        self
    }

    pub fn with_stats<S>(&mut self, stats: &[S]) -> &mut Self
    where
        S: ToString,
    {
        self.body.stats = Some(stats.iter().map(ToString::to_string).collect());
        self
    }

    pub fn with_min_score(&mut self, min_score: f64) -> &mut Self {
        self.body.min_score = Some(min_score);
        self
    }

    pub fn with_sort(&mut self, sort: &'b Sort) -> &mut Self {
        self.body.sort = Some(sort);
        self
    }

    pub fn with_track_scores(&mut self, track_scores: bool) -> &mut Self {
        self.body.track_scores = Some(track_scores);
        self
    }

    /// Specify source filtering, by default full source will be returned in a hit
    ///
    /// To switch-off source document in each hit: `with_source(Source::Off)`.
    /// To include fields: `with_source(Source::include(&["field_name"]))`,
    /// To exclude fields: `with_source(Source::exclude(&["field_name"]))`,
    /// To include and exclude: `with_source(Source::filter(&["include"], &["exclude"]))`
    pub fn with_source(&mut self, source: Source<'b>) -> &mut Self {
        self.body.source = Some(source);
        self
    }

    /// Specify any aggregations
    pub fn with_aggs(&mut self, aggs: &'b aggregations::Aggregations) -> &mut Self {
        self.body.aggs = Some(aggs);
        self
    }

    /// Specify fields to highlight
    pub fn with_highlight(&mut self, highlight: &'b highlight::Highlight) -> &mut Self {
        self.body.highlight = Some(highlight);
        self
    }

    add_option!(with_routing, "routing");
    add_option!(with_search_type, "search_type");
    add_option!(with_query_cache, "query_cache");
    add_option!(with_ignore_unavailable, "ignore_unavailable");
    add_option!(with_allow_no_indices, "allow_no_indices");
    add_option!(with_expand_wildcards, "expand_wildcards");
    add_option!(with_explain, "explain");

    /// Performs the search with the specified query and options
    pub fn send<T>(&'b mut self) -> Result<SearchResult<T>, EsError>
    where
        T: DeserializeOwned,
    {
        let url = format!(
            "/{}/_search{}",
            format_indexes_and_types(&self.indexes, &self.doc_types),
            self.options
        );
        let response = self.client.post_body_op(&url, &self.body)?;
        match response.status_code() {
            StatusCode::OK => {
                let interim: SearchResultInterim<T> = response.read_response()?;
                let aggs = match &interim.aggs {
                    Some(ref raw_aggs) => {
                        let req_aggs = match &self.body.aggs {
                            Some(ref aggs) => aggs,
                            None => {
                                return Err(EsError::EsError(
                                    "No aggs despite being in results".to_owned(),
                                ));
                            }
                        };
                        Some(AggregationsResult::from(req_aggs, raw_aggs)?)
                    }
                    None => None,
                };
                let mut result = interim.finalize();
                result.aggs = aggs;
                Ok(result)
            }
            status_code => Err(EsError::EsError(format!(
                "Unexpected status: {}",
                status_code
            ))),
        }
    }

    #[cfg(feature = "es5")]
    pub fn scan<T>(&'b mut self, scroll: &'b Duration) -> Result<ScanResult<T>, EsError>
    where
        T: DeserializeOwned + Serialize,
    {
        self.options.push("scroll", scroll);

        // FIXME: Raise if already having a sort option
        // let sort_by_doc = Sort::field("_doc");
        // self.with_sort(&sort_by_doc);

        let serialized = serde_json::to_string(&self.send::<T>()?)?;
        Ok(serde_json::from_str(&serialized)?)
    }

    /// Begins a scan with the specified query and options
    #[cfg(not(feature = "es5"))]
    pub fn scan<T>(&'b mut self, scroll: &'b Duration) -> Result<ScanResult<T>, EsError>
    where
        T: DeserializeOwned,
    {
        self.options.push("search_type", "scan");
        self.options.push("scroll", scroll);
        let url = format!(
            "/{}/_search{}",
            format_indexes_and_types(&self.indexes, &self.doc_types),
            self.options
        );
        let response = self.client.post_body_op(&url, &self.body)?;
        match response.status_code() {
            StatusCode::OK => {
                let interim: ScanResultInterim<T> = response.read_response()?;
                let aggs = match &interim.aggs {
                    Some(ref raw_aggs) => {
                        let req_aggs = match &self.body.aggs {
                            Some(ref aggs) => aggs,
                            None => {
                                return Err(EsError::EsError(
                                    "No aggs despite being in results".to_owned(),
                                ));
                            }
                        };
                        Some(AggregationsResult::from(req_aggs, raw_aggs)?)
                    }
                    None => None,
                };
                let mut result = interim.finalize();
                result.aggs = aggs;
                Ok(result)
            }
            StatusCode::NOT_FOUND => {
                Err(EsError::EsServerError(format!("Not found: {:?}", response)))
            }
            status_code => Err(EsError::EsError(format!(
                "Unexpected status: {}",
                status_code
            ))),
        }
    }
}

impl Client {
    /// Search via the query parameter
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/1.x/search-uri-request.html
    pub fn search_uri(&mut self) -> SearchURIOperation {
        SearchURIOperation::new(self)
    }

    /// Search via the query DSL
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/1.x/search-request-body.html
    pub fn search_query(&mut self) -> SearchQueryOperation {
        SearchQueryOperation::new(self)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchHitsHitsResult<T> {
    #[serde(rename = "_index")]
    pub index: String,
    #[serde(rename = "_type")]
    pub doc_type: String,
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "_score")]
    pub score: Option<f64>,
    #[serde(rename = "_version")]
    pub version: Option<u64>,
    #[serde(rename = "_source")]
    pub source: Option<Box<T>>,
    #[serde(rename = "_explanation")]
    pub explanation: Option<Value>,
    #[serde(rename = "_timestamp")]
    pub timestamp: Option<f64>,
    #[serde(rename = "_routing")]
    pub routing: Option<String>,
    pub fields: Option<Value>,
    pub highlight: Option<HighlightResult>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchHitsResult<T> {
    pub total: u64,
    pub hits: Vec<SearchHitsHitsResult<T>>,
}

impl<T> SearchHitsResult<T>
where
    T: DeserializeOwned,
{
    pub fn hits(self) -> Option<Vec<Box<T>>> {
        let mut r = Vec::with_capacity(self.hits.len());
        for b in self.hits.into_iter() {
            match b.source {
                Some(val) => r.push(val),
                None => return None,
            }
        }
        Some(r)
    }

    pub fn hits_ref(&self) -> Option<Vec<&T>> {
        let mut r = Vec::with_capacity(self.hits.len());
        for b in self.hits.iter() {
            match b.source {
                Some(ref v) => r.push(v.as_ref()),
                None => return None,
            }
        }
        Some(r)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchResultInterim<T> {
    pub took: u64,
    pub timed_out: bool,

    #[serde(rename = "_shards")]
    pub shards: ShardCountResult,
    pub hits: SearchHitsResult<T>,

    /// Optional field populated if aggregations are specified
    #[serde(rename = "aggregations")]
    pub aggs: Option<Value>,

    /// Optional field populated during scanning and scrolling
    #[serde(rename = "_scroll_id")]
    pub scroll_id: Option<String>,
}

impl<T> SearchResultInterim<T>
where
    T: DeserializeOwned,
{
    fn finalize(self) -> SearchResult<T> {
        SearchResult {
            took: self.took,
            timed_out: self.timed_out,
            shards: self.shards,
            hits: self.hits,
            aggs: None,
            scroll_id: self.scroll_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResult<T> {
    pub took: u64,
    pub timed_out: bool,
    pub shards: ShardCountResult,
    pub hits: SearchHitsResult<T>,
    pub aggs: Option<AggregationsResult>,
    pub scroll_id: Option<String>,
}

impl<T> SearchResult<T>
where
    T: DeserializeOwned,
{
    /// Take a reference to any aggregations in this result
    pub fn aggs_ref(&self) -> Option<&AggregationsResult> {
        self.aggs.as_ref()
    }
}

#[derive(Debug)]
pub struct ScanIterator<'a, T: DeserializeOwned + Debug> {
    scan_result: ScanResult<T>,
    scroll: Duration,
    client: &'a mut Client,
    page: Vec<SearchHitsHitsResult<T>>,
}

#[derive(Debug, Serialize)]
struct ScanBody<'a> {
    scroll: String,
    scroll_id: &'a str,
}

impl<'a, T> ScanIterator<'a, T>
where
    T: DeserializeOwned + Debug,
{
    /// Fetch the next page and return the first hit, or None if there are no hits
    fn next_page(&mut self) -> Option<Result<SearchHitsHitsResult<T>, EsError>> {
        match self.scan_result.scroll(self.client, &self.scroll) {
            Ok(scroll_page) => {
                self.page = scroll_page.hits.hits;
                if !self.page.is_empty() {
                    Some(Ok(self.page.remove(0)))
                } else {
                    None
                }
            }
            Err(err) => Some(Err(err)),
        }
    }
}

impl<'a, T> Drop for ScanIterator<'a, T>
where
    T: DeserializeOwned + Debug,
{
    fn drop(&mut self) {
        match self.scan_result.close(self.client) {
            Ok(_) => (),
            Err(e) => {
                log::error!("Cannot close scroll: {}", e);
            }
        }
    }
}

impl<'a, T> Iterator for ScanIterator<'a, T>
where
    T: DeserializeOwned + Debug,
{
    type Item = Result<SearchHitsHitsResult<T>, EsError>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.page.is_empty() {
            Some(Ok(self.page.remove(0)))
        } else {
            self.next_page()
        }
    }
}

/// Used when scanning and scrolling through results, a `ScanResult` is returned
/// when the scan is opened.  To scroll through the results an application has
/// two options:
///
/// 1. Call `scroll` repeatedly until the returned results have zero hits.  If
/// this approach is taken, the caller is also responsible for calling `close`
/// when finished, to prevent any server-side resources being held open.
///
/// 2. Call 'iter' to create an iterator from which the hits can be read.  If
/// this approach is taken, there is no need to call `close`, it will be called
/// automatically when iteration ends.
///
/// See also the [official ElasticSearch documentation](https://www.elastic.co/guide/en/elasticsearch/guide/current/scan-scroll.html)
/// for proper use of this functionality.
#[derive(Debug, Deserialize)]
pub struct ScanResultInterim<T> {
    #[serde(rename = "_scroll_id")]
    scroll_id: String,
    took: u64,
    timed_out: bool,
    #[serde(rename = "_shards")]
    shards: ShardCountResult,
    hits: SearchHitsResult<T>,
    #[serde(rename = "aggregations")]
    aggs: Option<Value>,
}

impl<T> ScanResultInterim<T>
where
    T: DeserializeOwned,
{
    #[cfg(not(feature = "es5"))]
    fn finalize(self) -> ScanResult<T> {
        ScanResult {
            scroll_id: self.scroll_id,
            took: self.took,
            timed_out: self.timed_out,
            shards: self.shards,
            hits: self.hits,
            aggs: None,
        }
    }
}

#[derive(Debug, Deserialize )]
pub struct ScanResult<T> {
    pub scroll_id: String,
    pub took: u64,
    pub timed_out: bool,
    pub shards: ShardCountResult,
    pub hits: SearchHitsResult<T>,
    pub aggs: Option<AggregationsResult>,
}

impl<T> ScanResult<T>
where
    T: DeserializeOwned + Debug,
{
    /// Returns an iterator from which hits can be read
    pub fn iter(self, client: &mut Client, scroll: Duration) -> ScanIterator<T> {
        ScanIterator {
            scan_result: self,
            scroll,
            client,
            page: vec![],
        }
    }

    /// Calls the `/_search/scroll` ES end-point for the next page
    pub fn scroll(
        &mut self,
        client: &mut Client,
        scroll: &Duration,
    ) -> Result<SearchResult<T>, EsError> {
        let url = "/_search/scroll";

        let response = {
            let body = ScanBody {
                scroll: scroll.to_string(),
                scroll_id: &self.scroll_id,
            };
            client.post_body_op(url, &body)?
        };

        match response.status_code() {
            StatusCode::OK => {
                let search_result: SearchResultInterim<T> = response.read_response()?;
                self.scroll_id = match search_result.scroll_id {
                    Some(ref id) => id.clone(),
                    None => return Err(EsError::EsError("Expecting scroll_id".to_owned())),
                };
                log::debug!("Scrolled: {:?}", search_result);
                Ok(search_result.finalize())
            }
            status_code => Err(EsError::EsError(format!(
                "Unexpected status: {}",
                status_code
            ))),
        }
    }

    /// Calls ES to close the server-side part of the scan/scroll operation
    pub fn close(&self, client: &mut Client) -> Result<(), EsError> {
        let url = format!("/_search/scroll?scroll_id={}", self.scroll_id);
        let response = client.delete_op(&url)?;
        match response.status_code() {
            StatusCode::OK => Ok(()),        // closed
            StatusCode::NOT_FOUND => Ok(()), // previously closed
            status_code => Err(EsError::EsError(format!(
                "Unexpected status: {}",
                status_code
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::Client;

    use crate::tests::{clean_db, make_client, setup_test_data, TestDocument};

    use crate::operations::bulk::Action;
    use crate::query::Query;
    use crate::units::{Duration, JsonVal};

    use super::ScanResult;
    use super::SearchHitsHitsResult;
    use super::SearchResult;
    use super::Sort;
    use super::Source;

    use super::aggregations::bucket::{Order, OrderKey, Terms};
    use super::aggregations::metrics::Min;
    use super::aggregations::Aggregations;

    use super::highlight::{Highlight, HighlightResult, Setting, SettingTypes};

    fn make_document(idx: i64) -> TestDocument {
        TestDocument::new()
            .with_str_field(&format!("BulkDoc: {}", idx))
            .with_int_field(idx)
    }

    fn setup_scan_data(client: &mut Client, index_name: &str) {
        let actions: Vec<Action<TestDocument>> = (0..1000)
            .map(|idx| Action::index(make_document(idx)))
            .collect();

        client
            .bulk(&actions)
            .with_index(index_name)
            .with_doc_type("doc_type")
            .send()
            .unwrap();

        client.refresh().with_indexes(&[index_name]).send().unwrap();
    }

    #[test]
    fn test_search_uri() {
        let index_name = "test_search_uri";
        let mut client = make_client();

        clean_db(&mut client, index_name);
        setup_test_data(&mut client, index_name);

        let all_results: SearchResult<TestDocument> = client
            .search_uri()
            .with_indexes(&[index_name])
            .send()
            .unwrap();
        assert_eq!(3, all_results.hits.total);

        let doc_a: SearchResult<TestDocument> = client
            .search_uri()
            .with_indexes(&[index_name])
            .with_query("A123")
            .send()
            .unwrap();
        assert_eq!(1, doc_a.hits.total);
        // TODO - add assertion for document contents

        let doc_1: SearchResult<TestDocument> = client
            .search_uri()
            .with_indexes(&[index_name])
            .with_query("str_field:1ABC")
            .send()
            .unwrap();
        assert_eq!(1, doc_1.hits.total);
        // TODO - add assertion for document contents

        #[cfg(not(feature = "es5"))]
        let limited_fields: SearchResult<Value> = client
            .search_uri()
            .with_indexes(&[index_name])
            .with_query("str_field:B456")
            .with_fields(&["int_field"])
            .send()
            .unwrap();

        #[cfg(not(feature = "es5"))]
        assert_eq!(1, limited_fields.hits.total);
        // TODO - add assertion for document contents

    }

    #[test]
    fn test_search_body() {
        let index_name = "test_search_body";
        let mut client = make_client();
        clean_db(&mut client, index_name);
        setup_test_data(&mut client, index_name);

        let all_results: SearchResult<TestDocument> = client
            .search_query()
            .with_indexes(&[index_name])
            .with_query(&Query::build_match_all().build())
            .send()
            .unwrap();
        assert_eq!(3, all_results.hits.total);
        // TODO - add assertion for document content

        let within_range: SearchResult<TestDocument> = client
            .search_query()
            .with_indexes(&[index_name])
            .with_query(
                &Query::build_range("int_field")
                    .with_gte(2)
                    .with_lte(3)
                    .build(),
            )
            .send()
            .unwrap();
        assert_eq!(2, within_range.hits.total);
        // TODO - add assertion for document content
    }

    #[test]
    fn test_close() {
        let mut client = make_client();
        let index_name = "tests_test_close";
        crate::tests::clean_db(&mut client, index_name);
        setup_scan_data(&mut client, index_name);

        let indexes = [index_name];

        let scroll = Duration::minutes(1);
        let mut scan_result: ScanResult<TestDocument> = client
            .search_query()
            .with_indexes(&indexes)
            .with_size(100)
            .scan(&scroll)
            .unwrap();

        scan_result.scroll(&mut client, &scroll).unwrap();

        scan_result.close(&mut client).unwrap();
    }


    #[test]
    #[cfg(feature = "es5")]
    fn test_scan_and_scroll() {
        let mut client = make_client();
        let index_name = "tests_test_scan_and_scroll";
        crate::tests::clean_db(&mut client, index_name);
        setup_scan_data(&mut client, index_name);

        let indexes = [index_name];

        let scroll = Duration::minutes(1);
        let mut scan_result: ScanResult<TestDocument> = client
            .search_query()
            .with_indexes(&indexes)
            .with_sort(&Sort::field("_doc"))
            .with_size(100)
            .scan(&scroll)
            .unwrap();

        assert_eq!(1000, scan_result.hits.total);
        let mut total = 0;
        let mut page_total = scan_result.hits.hits.len();

        loop {
            assert!(page_total > 0);
            assert!(total <= 1000);

            total += page_total;

            let page = scan_result.scroll(&mut client, &scroll).unwrap();
            page_total = page.hits.hits.len();

            if page_total == 0 && total == 1000 {
                break;
            }
        }

        scan_result.close(&mut client).unwrap();
    }

    #[test]
    #[cfg(not(feature = "es5"))]
    fn test_scan_and_scroll() {
        let mut client = make_client();
        let index_name = "tests_test_scan_and_scroll";
        crate::tests::clean_db(&mut client, index_name);
        setup_scan_data(&mut client, index_name);

        let indexes = [index_name];

        let scroll = Duration::minutes(1);
        let mut scan_result: ScanResult<TestDocument> = client
            .search_query()
            .with_indexes(&indexes)
            .with_size(100)
            .scan(&scroll)
            .unwrap();

        assert_eq!(1000, scan_result.hits.total);
        let mut total = 0;

        loop {
            let page = scan_result.scroll(&mut client, &scroll).unwrap();
            let page_total = page.hits.hits.len();
            total += page_total;
            if page_total == 0 && total == 1000 {
                break;
            }

            assert!(page_total > 0);
            assert!(total <= 1000);
        }

        scan_result.close(&mut client).unwrap();
    }

    #[test]
    fn test_with_version() {
        let mut client = make_client();
        let index_name = "test_version";
        crate::tests::clean_db(&mut client, index_name);
        setup_test_data(&mut client, index_name);

        let indexes = [index_name];

        // Version: true
        {
            let results: SearchResult<TestDocument> = client
                .search_query()
                .with_indexes(&indexes)
                .with_version(true)
                .send()
                .unwrap();

            assert_eq!(3, results.hits.total);

            let result_versions: Vec<u64> = results
                .hits
                .hits
                .into_iter()
                .map(|doc| doc.version.unwrap())
                .collect();

            // Update a document when the update API is implemented to verify that the version comes back correctly
            let expected_result_versions: Vec<u64> =
                vec![1, 1, 1].into_iter().map(|x| x.to_owned()).collect();

            assert_eq!(expected_result_versions, result_versions);
        }

        // Version: false
        {
            let results: SearchResult<TestDocument> = client
                .search_query()
                .with_indexes(&indexes)
                .with_version(false)
                .send()
                .unwrap();

            let result_versions: Vec<Option<u64>> = results
                .hits
                .hits
                .into_iter()
                .map(|doc| doc.version)
                .collect();

            for maybe_version in &result_versions {
                assert!(maybe_version.is_none())
            }
        }

        // Version: not set
        {
            let results: SearchResult<TestDocument> =
                client.search_query().with_indexes(&indexes).send().unwrap();

            let result_versions: Vec<Option<u64>> = results
                .hits
                .hits
                .into_iter()
                .map(|doc| doc.version)
                .collect();

            for maybe_version in &result_versions {
                assert!(maybe_version.is_none())
            }
        }
    }

    #[test]
    fn test_scan_and_iterate() {
        let mut client = make_client();
        let index_name = "tests_test_scan_and_iterate";
        crate::tests::clean_db(&mut client, index_name);
        setup_scan_data(&mut client, index_name);

        let indexes = [index_name];

        let scroll = Duration::minutes(1);
        let scan_result: ScanResult<TestDocument> = client
            .search_query()
            .with_indexes(&indexes)
            .with_size(10)
            .scan(&scroll)
            .unwrap();

        assert_eq!(1000, scan_result.hits.total);

        let hits: Vec<SearchHitsHitsResult<TestDocument>> = scan_result
            .iter(&mut client, scroll)
            .take(200)
            .map(Result::unwrap)
            .collect();

        assert_eq!(200, hits.len());
    }

    #[test]
    fn test_source_filter() {
        let mut client = make_client();
        let index_name = "test_source_filter";
        crate::tests::clean_db(&mut client, index_name);

        client
            .index(index_name, "test")
            .with_doc(&make_document(100))
            .send()
            .unwrap();
        client.refresh().with_indexes(&[index_name]).send().unwrap();

        // Use of `Value` is necessary as the JSON returned is an arbitrary format
        // determined by the source filter
        let mut result: SearchResult<Value> = client
            .search_query()
            .with_indexes(&[index_name])
            .with_source(Source::include(&["str_field"]))
            .send()
            .unwrap();

        assert_eq!(1, result.hits.hits.len());
        let json = result.hits.hits.remove(0).source.unwrap();

        assert_eq!(true, json.get("str_field").is_some());
        assert_eq!(false, json.get("int_field").is_some());
    }

    #[test]
    fn test_highlight() {
        let mut client = make_client();
        let index_name = "test_highlight";
        crate::tests::clean_db(&mut client, index_name);

        client
            .bulk(&[
                Action::index(TestDocument::new().with_str_field("C++ and Java")),
                Action::index(TestDocument::new().with_str_field("Rust and Java")),
                Action::index(TestDocument::new().with_str_field("Rust is nice")),
            ])
            .with_index(index_name)
            .with_doc_type("doc_type")
            .send()
            .unwrap();

        client.refresh().with_indexes(&[index_name]).send().unwrap();

        let mut highlight = Highlight::new();
        highlight.add_setting(
            "str_field".to_owned(),
            Setting::new().with_type(SettingTypes::Plain).to_owned(),
        );

        let query = Query::build_match("str_field", "Rust").build();

        let results: SearchResult<TestDocument> = client
            .search_query()
            .with_indexes(&[index_name])
            .with_highlight(&highlight)
            .with_query(&query)
            .with_sort(&Sort::field("str_field"))
            .send()
            .unwrap();

        let highlights: Vec<HighlightResult> = results
            .hits
            .hits
            .into_iter()
            .map(|doc| doc.highlight.unwrap())
            .collect();

        assert_eq!(highlights.len(), 2);
        assert_eq!(
            highlights[1].get("str_field"),
            Some(&vec!["<em>Rust</em> is nice".to_owned()])
        );
    }

    #[test]
    fn test_bucket_aggs() {
        let mut client = make_client();
        let index_name = "test_bucket_aggs";
        crate::tests::clean_db(&mut client, index_name);

        client
            .bulk(&[
                Action::index(TestDocument::new().with_str_field("A").with_int_field(2)),
                Action::index(TestDocument::new().with_str_field("B").with_int_field(3)),
                Action::index(TestDocument::new().with_str_field("A").with_int_field(1)),
                Action::index(TestDocument::new().with_str_field("B").with_int_field(2)),
            ])
            .with_index(index_name)
            .with_doc_type("doc_type")
            .send()
            .unwrap();

        client.refresh().with_indexes(&[index_name]).send().unwrap();

        let aggs = Aggregations::from((
            "str",
            (
                Terms::field("str_field").with_order(Order::asc(OrderKey::Term)),
                Aggregations::from(("int", Min::field("int_field"))),
            ),
        ));

        let result: SearchResult<TestDocument> = client
            .search_query()
            .with_indexes(&[index_name])
            .with_aggs(&aggs)
            .send()
            .unwrap();

        let buckets = &result
            .aggs_ref()
            .unwrap()
            .get("str")
            .unwrap()
            .as_terms()
            .unwrap()
            .buckets;

        let bucket_a = &buckets[0];
        let bucket_b = &buckets[1];

        assert_eq!(2, bucket_a.doc_count);
        assert_eq!(2, bucket_b.doc_count);

        let min_a = &bucket_a
            .aggs_ref()
            .unwrap()
            .get("int")
            .unwrap()
            .as_min()
            .unwrap()
            .value;

        let min_b = &bucket_b
            .aggs_ref()
            .unwrap()
            .get("int")
            .unwrap()
            .as_min()
            .unwrap()
            .value;

        match min_a {
            JsonVal::Number(ref i) => assert_eq!(Some(1.0), i.as_f64()),
            _ => panic!("Not an integer"),
        }
        match min_b {
            JsonVal::Number(ref i) => assert_eq!(Some(2.0), i.as_f64()),
            _ => panic!("Not an integer"),
        }
    }

    #[test]
    fn test_aggs() {
        let mut client = make_client();
        let index_name = "test_aggs";
        crate::tests::clean_db(&mut client, index_name);

        client
            .bulk(&[
                Action::index(TestDocument::new().with_int_field(10)),
                Action::index(TestDocument::new().with_int_field(1)),
            ])
            .with_index(index_name)
            .with_doc_type("doc_type")
            .send()
            .unwrap();

        client.refresh().with_indexes(&[index_name]).send().unwrap();

        let result: SearchResult<TestDocument> = client
            .search_query()
            .with_indexes(&[index_name])
            .with_aggs(&Aggregations::from((
                "min_int_field",
                Min::field("int_field"),
            )))
            .send()
            .unwrap();

        let min = &result
            .aggs_ref()
            .unwrap()
            .get("min_int_field")
            .unwrap()
            .as_min()
            .unwrap()
            .value;

        match min {
            JsonVal::Number(ref i) => assert_eq!(Some(1.0), i.as_f64()),
            _ => panic!("Not an integer"),
        }
    }

    #[test]
    fn test_sort() {
        let mut client = make_client();
        let index_name = "test_sort";
        crate::tests::clean_db(&mut client, index_name);

        client
            .bulk(&[
                Action::index(TestDocument::new().with_str_field("B").with_int_field(10)),
                Action::index(TestDocument::new().with_str_field("C").with_int_field(4)),
                Action::index(TestDocument::new().with_str_field("A").with_int_field(99)),
            ])
            .with_index(index_name)
            .with_doc_type("doc_type")
            .send()
            .unwrap();

        client.refresh().with_indexes(&[index_name]).send().unwrap();

        {
            let result: SearchResult<TestDocument> = client
                .search_uri()
                .with_indexes(&[index_name])
                .with_sort(&Sort::field("str_field"))
                .send()
                .unwrap();

            let result_str: Vec<String> = result
                .hits
                .hits()
                .unwrap()
                .into_iter()
                .map(|doc| doc.str_field)
                .collect();

            let expected_result_str: Vec<String> = vec!["A", "B", "C"]
                .into_iter()
                .map(ToOwned::to_owned)
                .collect();

            assert_eq!(expected_result_str, result_str);
        }
        {
            let result: SearchResult<TestDocument> = client
                .search_query()
                .with_indexes(&[index_name])
                .with_sort(&Sort::field("str_field"))
                .send()
                .unwrap();

            let result_str: Vec<String> = result
                .hits
                .hits()
                .unwrap()
                .into_iter()
                .map(|doc| doc.str_field)
                .collect();

            let expected_result_str: Vec<String> = vec!["A", "B", "C"]
                .into_iter()
                .map(ToOwned::to_owned)
                .collect();

            assert_eq!(expected_result_str, result_str);
        }
        {
            let result: SearchResult<TestDocument> = client
                .search_query()
                .with_indexes(&[index_name])
                .with_sort(&Sort::field("int_field"))
                .send()
                .unwrap();

            let result_str: Vec<String> = result
                .hits
                .hits()
                .unwrap()
                .into_iter()
                .map(|doc| doc.str_field)
                .collect();

            let expected_result_str: Vec<String> = vec!["C", "B", "A"]
                .into_iter()
                .map(ToOwned::to_owned)
                .collect();

            assert_eq!(expected_result_str, result_str);
        }
    }
}
