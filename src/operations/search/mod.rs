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

//! Implementations of both Search-by-URI and Search-by-Query operations

pub mod aggregations;

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;

use hyper::status::StatusCode;

use rustc_serialize::Decodable;
use rustc_serialize::json::{Json, ToJson};

use serde::de::Deserialize;
use serde::ser::{Serialize, Serializer};
use serde_json::Value;

use ::{Client, EsResponse};
use ::error::EsError;
use ::json::ShouldSkip;
use ::query::Query;
use ::units::{DistanceType, DistanceUnit, Duration, JsonVal, Location, OneOrMany};
use ::util::StrJoin;
use super::common::{Options, OptionVal};
use super::decode_json;
use super::format_indexes_and_types;
use super::ShardCountResult;

use self::aggregations::AggregationsResult;

/// Representing a search-by-uri option
pub struct SearchURIOperation<'a, 'b> {
    client: &'a mut Client,
    indexes: &'b [&'b str],
    doc_types: &'b [&'b str],
    options: Options<'b>
}

/// Options for the various search_type parameters
pub enum SearchType {
    DFSQueryThenFetch,
    DFSQueryAndFetch,
    QueryThenFetch,
    QueryAndFetch
}

impl ToString for SearchType {
    fn to_string(&self) -> String {
        match self {
            &SearchType::DFSQueryThenFetch => "dfs_query_then_fetch",
            &SearchType::DFSQueryAndFetch  => "dfs_query_and_fetch",
            &SearchType::QueryThenFetch    => "query_then_fetch",
            &SearchType::QueryAndFetch     => "query_and_fetch"
        }.to_owned()
    }
}

/// Order of a sort
#[derive(Debug)]
pub enum Order {
    Asc,
    Desc
}

impl ToString for Order {
    fn to_string(&self) -> String {
        match self {
            &Order::Asc => "asc",
            &Order::Desc => "desc"
        }.to_owned()
    }
}

impl Serialize for Order {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        self.to_string().serialize(serializer)
    }
}

// TODO - deprecated
impl ToJson for Order {
    fn to_json(&self) -> Json {
        Json::String(self.to_string())
    }
}

/// The (Sort mode option)[https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-sort.html#_sort_mode_option].
pub enum Mode {
    Min,
    Max,
    Sum,
    Avg
}

impl Serialize for Mode {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        match self {
            &Mode::Min => "min",
            &Mode::Max => "max",
            &Mode::Sum => "sum",
            &Mode::Avg => "avg"
        }.serialize(serializer)
    }
}

// TODO - deprecated
impl ToJson for Mode {
    fn to_json(&self) -> Json {
        Json::String(match self {
            &Mode::Min => "min",
            &Mode::Max => "max",
            &Mode::Sum => "sum",
            &Mode::Avg => "avg"
        }.to_owned())
    }
}

/// Options for handling (missing values)[https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-sort.html#_missing_values]
pub enum Missing {
    First,
    Last,
    Custom(String)
}

impl Serialize for Missing {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        match self {
            &Missing::First => "first".serialize(serializer),
            &Missing::Last  => "last".serialize(serializer),
            &Missing::Custom(ref s) => s.serialize(serializer)
        }
    }
}

// TODO - deprecated
impl ToJson for Missing {
    fn to_json(&self) -> Json {
        Json::String(match self {
            &Missing::First         => "_first".to_owned(),
            &Missing::Last          => "_last".to_owned(),
            &Missing::Custom(ref s) => s.clone()
        })
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
// TODO - this has an outer and an inner structure, similar to query fields, should refactor
#[derive(Serialize)]
pub struct SortField {
    field:         String,
    order:         Option<Order>,
    mode:          Option<Mode>,
    nested_path:   Option<String>,
    nested_filter: Option<Query>,
    missing:       Option<Missing>,
    unmapped_type: Option<String>
}

impl SortField {
    /// Create a `SortField` for a given `field` and `order`
    pub fn new<S: Into<String>>(field: S, order: Option<Order>) -> SortField {
        SortField {
            field:         field.into(),
            order:         order,
            mode:          None,
            nested_path:   None,
            nested_filter: None,
            missing:       None,
            unmapped_type: None
        }
    }

    add_field!(with_mode, mode, Mode);
    add_field!(with_nested_path, nested_path, String);
    add_field!(with_nested_filter, nested_filter, Query);
    add_field!(with_missing, missing, Missing);
    add_field!(with_unmapped_type, unmapped_type, String);

    pub fn build(self) -> SortBy {
        SortBy::Field(self)
    }
}

impl ToString for SortField {
    fn to_string(&self) -> String {
        let mut s = String::new();
        s.push_str(&self.field);
        match self.order {
            Some(ref order) => {
                s.push_str(":");
                // TODO - find less clumsy way of implementing the following
                // line
                s.push_str(&order.to_string());
            },
            None            => ()
        }
        s
    }
}

impl ToJson for SortField {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();

        optional_add!(self, inner, order);
        optional_add!(self, inner, mode);
        optional_add!(self, inner, nested_path);
        optional_add!(self, inner, nested_filter);
        optional_add!(self, inner, missing);
        optional_add!(self, inner, unmapped_type);

        d.insert(self.field.clone(), Json::Object(inner));
        Json::Object(d)
    }
}

/// Representing sort options for sort by geodistance
// TODO - fix structure to represent reality
#[derive(Serialize)]
pub struct GeoDistance {
    field:         String,
    location:      OneOrMany<Location>,
    order:         Option<Order>,
    unit:          Option<DistanceUnit>,
    mode:          Option<Mode>,
    distance_type: Option<DistanceType>,
}

impl GeoDistance {
    pub fn new<S>(field: S) -> GeoDistance
        where S: Into<String>
    {
        GeoDistance {
            field: field.into(),
            location: OneOrMany::Many(vec![]),
            order: None,
            unit: None,
            mode: None,
            distance_type: None
        }
    }

    pub fn with_location<L: Into<Location>>(mut self, location: L) -> Self {
        self.location = OneOrMany::One(location.into());
        self
    }

    pub fn with_locations<L: Into<Location>>(mut self, location: Vec<L>) -> Self {
        self.location = OneOrMany::Many(location.into_iter().map(|l| l.into()).collect());
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

impl ToJson for GeoDistance {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();

        inner.insert(self.field.clone(), self.location.to_json());

        optional_add!(self, inner, order);
        optional_add!(self, inner, unit);
        optional_add!(self, inner, mode);
        optional_add!(self, inner, distance_type);

        d.insert("_geo_distance".to_owned(), Json::Object(inner));
        Json::Object(d)
    }
}

/// Representing options for sort by script
// TODO - fix structure
#[derive(Serialize)]
pub struct Script {
    script:      String,
    script_type: String,
    params:      HashMap<String, JsonVal>,
    order:       Option<Order>
}

impl Script {
    pub fn new<S, ST>(script: S, script_type: ST) -> Script
        where S: Into<String>,
              ST: Into<String>
    {
        Script {
            script: script.into(),
            script_type: script_type.into(),
            params: HashMap::new(),
            order: None
        }
    }

    add_field!(with_order, order, Order);

    pub fn add_param<K, V>(mut self, key: K, value: V) -> Self
        where K: Into<String>,
              V: Into<JsonVal>
    {
        self.params.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> SortBy {
        SortBy::Script(self)
    }
}

impl ToJson for Script {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();

        inner.insert("script".to_owned(), self.script.to_json());
        inner.insert("type".to_owned(), self.script_type.to_json());
        inner.insert("params".to_owned(), self.params.to_json());

        d.insert("_script".to_owned(), Json::Object(inner));
        Json::Object(d)
    }
}

pub enum SortBy {
    Field(SortField),
    Distance(GeoDistance),
    Script(Script)
}

impl Serialize for SortBy {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        match self {
            &SortBy::Field(ref f) => f.serialize(serializer),
            &SortBy::Distance(ref d) => d.serialize(serializer),
            &SortBy::Script(ref s) => s.serialize(serializer),
        }
    }
}

impl ToString for SortBy {
    fn to_string(&self) -> String {
        match self {
            &SortBy::Field(ref field) => field.to_string(),
            _                         => panic!("Can only convert field sorting ToString")
        }
    }
}

impl ToJson for SortBy {
    fn to_json(&self) -> Json {
        match self {
            &SortBy::Field(ref field)   => field.to_json(),
            &SortBy::Distance(ref dist) => dist.to_json(),
            &SortBy::Script(ref scr)    => scr.to_json()
        }
    }
}

/// A full sort clause
#[derive(Serialize)]
pub struct Sort {
    fields: Vec<SortBy>
}

impl Sort {
    pub fn new(fields: Vec<SortBy>) -> Self {
        Sort {
            fields: fields
        }
    }

    /// Convenience function for a single field default
    pub fn field<S: Into<String>>(fieldname: S) -> Self {
        Sort {
            fields: vec![SortField::new(fieldname, None).build()]
        }
    }

    pub fn field_order<S: Into<String>>(fieldname: S, order: Order) -> Self {
        Sort {
            fields: vec![SortField::new(fieldname, Some(order)).build()]
        }
    }

    pub fn fields<S: Into<String>>(fieldnames: Vec<S>) -> Self {
        Sort {
            fields: fieldnames.into_iter().map(|fieldname| {
                SortField::new(fieldname, None).build()
            }).collect()
        }
    }

    pub fn field_orders<S: Into<String>>(fields: Vec<(S, Order)>) -> Self {
        Sort {
            fields: fields.into_iter().map(|(fieldname, order)| {
                SortField::new(fieldname, Some(order)).build()
            }).collect()
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
        OptionVal(from.fields.iter().map(|f| f.to_string()).join(","))
    }
}

impl ToJson for Sort {
    fn to_json(&self) -> Json {
        self.fields.to_json()
    }
}

impl<'a, 'b> SearchURIOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> SearchURIOperation<'a, 'b> {
        SearchURIOperation {
            client:    client,
            indexes:   &[],
            doc_types: &[],
            options:   Options::new()
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
    add_option!(with_sort, "sort");
    add_option!(with_routing, "routing");
    add_option!(with_track_scores, "track_scores");
    add_option!(with_timeout, "timeout");
    add_option!(with_terminate_after, "terminate_after");
    add_option!(with_from, "from");
    add_option!(with_size, "size");
    add_option!(with_search_type, "search_type");

    pub fn with_fields(&'b mut self, fields: &[&str]) -> &'b mut Self {
        self.options.push("fields", fields.iter().join(","));
        self
    }

    pub fn send<T>(&'b mut self) -> Result<SearchResult<T>, EsError>
        where T: Deserialize {

        let url = format!("/{}/_search{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
                          self.options);
        info!("Searching with: {}", url);
        let response = try!(self.client.get_op(&url));
        match response.status_code() {
            &StatusCode::Ok => {
                let interim:SearchResultInterim<T> = try!(response.read_response());
                Ok(interim.finalize())
            },
            _ => Err(EsError::EsError(format!("Unexpected status: {}",
                                              response.status_code())))
        }
    }
}

/// Options for source filtering
pub enum Source<'a> {
    /// Disable source documents
    Off,

    /// Filtering
    Filter(Option<&'a [&'a str]>, Option<&'a [&'a str]>)
}

impl<'a> Serialize for Source<'a> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        match self {
            &Source::Off => false.serialize(serializer),
            &Source::Filter(incl, excl) => {
                let mut d = BTreeMap::new();
                match incl {
                    Some(val) => { d.insert("include".to_owned(), val); },
                    None      => (),
                }
                match excl {
                    Some(val) => { d.insert("exclude".to_owned(), val); },
                    None      => (),
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

/// Convenience function to Json-ify a reference to a slice of references of
/// items that can be converted to Json
// TODO - deprecated
fn slice_to_json<J: ToJson + ?Sized>(slice: &[&J]) -> Json {
    Json::Array(slice.iter().map(|e| {
        e.to_json()
    }).collect())
}

// TODO - deprecated
impl<'a> ToJson for Source<'a> {
    fn to_json(&self) -> Json {
        match self {
            &Source::Off                => Json::Boolean(false),
            &Source::Filter(incl, excl) => {
                let mut d = BTreeMap::new();
                match incl {
                    Some(val) => { d.insert("include".to_owned(), slice_to_json(val)); },
                    None      => (),
                }
                match excl {
                    Some(val) => { d.insert("exclude".to_owned(), slice_to_json(val)); },
                    None      => (),
                }
                Json::Object(d)
            }
        }
    }
}

#[derive(Default, Serialize)]
struct SearchQueryOperationBody<'b> {
    /// The query
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    query: Option<&'b Query>,

    /// Timeout
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    timeout: Option<&'b str>,

    /// From
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    from: Option<u64>,

    /// Size
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    size: Option<u64>,

    /// Terminate early (marked as experimental in the ES docs)
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    terminate_after: Option<u64>,

    /// Stats groups to which the query belongs
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    stats: Option<Vec<String>>,

    /// Minimum score to use
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    min_score: Option<f64>,

    /// Sort fields
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    sort: Option<&'b Sort>,

    /// Track scores
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    track_scores: Option<bool>,

    /// Source filtering
    #[serde(rename="_source", skip_serializing_if="ShouldSkip::should_skip")]
    source: Option<Source<'b>>,

    /// Aggregations
    #[serde(rename="aggregations", skip_serializing_if="ShouldSkip::should_skip")]
    aggs: Option<&'b aggregations::Aggregations<'b>>
}

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
    body: SearchQueryOperationBody<'b>
}

impl <'a, 'b> SearchQueryOperation<'a, 'b> {
    pub fn new(client: &'a mut Client) -> SearchQueryOperation<'a, 'b> {
        SearchQueryOperation {
            client:    client,
            indexes:   &[],
            doc_types: &[],
            options:   Options::new(),
            body:      Default::default()
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

    pub fn with_query(&'b mut self, query: &'b Query) -> &'b mut Self {
        self.body.query = Some(query);
        self
    }

    pub fn with_timeout(&'b mut self, timeout: &'b str) -> &'b mut Self {
        self.body.timeout = Some(timeout);
        self
    }

    pub fn with_from(&'b mut self, from: u64) -> &'b mut Self {
        self.body.from = Some(from);
        self
    }

    pub fn with_size(&'b mut self, size: u64) -> &'b mut Self {
        self.body.size = Some(size);
        self
    }

    pub fn with_terminate_after(&'b mut self, terminate_after: u64) -> &'b mut Self {
        self.body.terminate_after = Some(terminate_after);
        self
    }

    pub fn with_stats<S>(&'b mut self, stats: &[S]) -> &'b mut Self
        where S: ToString
    {
        self.body.stats = Some(stats.iter().map(|s| s.to_string()).collect());
        self
    }

    pub fn with_min_score(&'b mut self, min_score: f64) -> &'b mut Self {
        self.body.min_score = Some(min_score);
        self
    }

    pub fn with_sort(&'b mut self, sort: &'b Sort) -> &'b mut Self {
        self.body.sort = Some(sort);
        self
    }

    pub fn with_track_scores(&'b mut self, track_scores: bool) -> &'b mut Self {
        self.body.track_scores = Some(track_scores);
        self
    }

    /// Specify source filtering, by default full source will be returned in a hit
    ///
    /// To switch-off source document in each hit: `with_source(Source::Off)`.
    /// To include fields: `with_source(Source::include(&["field_name"]))`,
    /// To exclude fields: `with_source(Source::exclude(&["field_name"]))`,
    /// To include and exclude: `with_source(Source::filter(&["include"], &["exclude"]))`
    pub fn with_source(&'b mut self, source: Source<'b>) -> &'b mut Self {
        self.body.source = Some(source);
        self
    }

    /// Specify any aggregations
    pub fn with_aggs(&'b mut self, aggs: &'b aggregations::Aggregations) -> &'b mut Self {
        self.body.aggs = Some(aggs);
        self
    }

    add_option!(with_routing, "routing");
    add_option!(with_search_type, "search_type");
    add_option!(with_query_cache, "query_cache");

    /// Performs the search with the specified query and options
    pub fn send<T>(&'b mut self) -> Result<SearchResult<T>, EsError>
        where T: Deserialize {

        let url = format!("/{}/_search{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
                          self.options);
        let response = try!(self.client.post_body_op(&url, &self.body));
        match response.status_code() {
            &StatusCode::Ok => {
                let interim:SearchResultInterim<T> = try!(response.read_response());
                let aggs = match &interim.aggs {
                    &Some(ref raw_aggs) => {
                        let req_aggs = match &self.body.aggs {
                            &Some(ref aggs) => aggs,
                            &None => return Err(EsError::EsError("No aggs despite being in results".to_owned()))
                        };
                        Some(try!(AggregationsResult::from(req_aggs,
                                                           raw_aggs)))
                    },
                    &None => None
                };
                let mut result = interim.finalize();
                result.aggs = aggs;
                Ok(result)
            },
            _ => Err(EsError::EsError(format!("Unexpected status: {}",
                                              response.status_code())))
        }
    }

    /// Begins a scan with the specified query and options
    pub fn scan<T>(&'b mut self, scroll: &'b Duration) -> Result<ScanResult<T>, EsError>
        where T: Deserialize {

        self.options.push("search_type", "scan");
        self.options.push("scroll", scroll);
        let url = format!("/{}/_search{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
                          self.options);
        let response = try!(self.client.post_body_op(&url, &self.body));
        match response.status_code() {
            &StatusCode::Ok => {
                let interim:ScanResultInterim<T> = try!(response.read_response());
                let aggs = match &interim.aggs {
                    &Some(ref raw_aggs) => {
                        let req_aggs = match &self.body.aggs {
                            &Some(ref aggs) => aggs,
                            &None => return Err(EsError::EsError("No aggs despite being in results".to_owned()))
                        };
                        Some(try!(AggregationsResult::from(req_aggs,
                                                           raw_aggs)))
                    },
                    &None => None
                };
                let mut result = interim.finalize();
                result.aggs = aggs;
                Ok(result)
            },
            &StatusCode::NotFound => {
                Err(EsError::EsServerError(format!("Not found: {:?}", response)))
            },
            _ => Err(EsError::EsError(format!("Unexpected status: {}", response.status_code())))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchHitsHitsResult<T: Deserialize> {
    #[serde(rename="_index")]
    pub index:    String,
    #[serde(rename="_type")]
    pub doc_type: String,
    #[serde(rename="_id")]
    pub id:       String,
    #[serde(rename="_score")]
    pub score:    Option<f64>,
    #[serde(rename="_source")]
    pub source:   Option<T>,
    // TODO - check this isn't deprecated
    //#[serde(rename="_fields")]
    // pub fields:   Option<Json>
}

#[derive(Debug, Deserialize)]
pub struct SearchHitsResult<T: Deserialize> {
    pub total: u64,
    pub hits:  Vec<SearchHitsHitsResult<T>>
}

#[derive(Debug, Deserialize)]
pub struct SearchResultInterim<T: Deserialize> {
    pub took:      u64,
    pub timed_out: bool,

    #[serde(rename="_shards")]
    pub shards:    ShardCountResult,
    pub hits:      SearchHitsResult<T>,

    /// Optional field populated if aggregations are specified
    pub aggs:      Option<Value>,

    /// Optional field populated during scanning and scrolling
    #[serde(rename="_scroll_id")]
    pub scroll_id: Option<String>
}

impl<T> SearchResultInterim<T>
    where T: Deserialize {

    fn finalize(self) -> SearchResult<T> {
        SearchResult {
            took: self.took,
            timed_out: self.timed_out,
            shards: self.shards,
            hits: self.hits,
            aggs: None,
            scroll_id: self.scroll_id
        }
    }
}

#[derive(Debug)]
pub struct SearchResult<T: Deserialize> {
    pub took:      u64,
    pub timed_out: bool,
    pub shards:    ShardCountResult,
    pub hits:      SearchHitsResult<T>,
    pub aggs:      Option<AggregationsResult>,
    pub scroll_id: Option<String>
}

impl<T> SearchResult<T>
    where T: Deserialize {

    /// Take a reference to any aggregations in this result
    pub fn aggs_ref<'a>(&'a self) -> Option<&'a AggregationsResult> {
        self.aggs.as_ref()
    }
}

pub struct ScanIterator<'a, T: Deserialize + Debug> {
    scan_result: ScanResult<T>,
    scroll:      Duration,
    client:      &'a mut Client,
    page:        Vec<SearchHitsHitsResult<T>>
}

impl<'a, T> ScanIterator<'a, T>
    where T: Deserialize + Debug {

    /// Fetch the next page and return the first hit, or None if there are no hits
    fn next_page(&mut self) -> Option<Result<SearchHitsHitsResult<T>, EsError>> {
        match self.scan_result.scroll(self.client, &self.scroll) {
            Ok(scroll_page) => {
                self.page = scroll_page.hits.hits;
                if self.page.len() > 0 {
                    Some(Ok(self.page.remove(0)))
                } else {
                    None
                }
            },
            Err(err)        => Some(Err(EsError::from(err)))
        }
    }
}

impl<'a, T> Drop for ScanIterator<'a, T>
    where T: Deserialize + Debug {

    fn drop(&mut self) {
        match self.scan_result.close(self.client) {
            Ok(_)  => (),
            Err(e) => {
                error!("Cannot close scroll: {}", e);
            }
        }
    }
}

impl<'a, T> Iterator for ScanIterator<'a, T>
    where T: Deserialize + Debug {

    type Item = Result<SearchHitsHitsResult<T>, EsError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.page.len() > 0 {
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
#[derive(Deserialize)]
pub struct ScanResultInterim<T: Deserialize> {
    #[serde(rename="_scroll_id")]
    scroll_id:     String,
    took:      u64,
    timed_out: bool,
    #[serde(rename="_shards")]
    shards:    ShardCountResult,
    hits:      SearchHitsResult<T>,
    aggs:      Option<Value>
}

impl<T> ScanResultInterim<T>
    where T: Deserialize {

    fn finalize(self) -> ScanResult<T> {
        ScanResult {
            scroll_id: self.scroll_id,
            took: self.took,
            timed_out: self.timed_out,
            shards: self.shards,
            hits: self.hits,
            aggs: None
        }
    }
}

pub struct ScanResult<T: Deserialize> {
    pub scroll_id: String,
    pub took: u64,
    pub timed_out: bool,
    pub shards: ShardCountResult,
    pub hits: SearchHitsResult<T>,
    pub aggs: Option<AggregationsResult>
}

impl<T> ScanResult<T>
    where T: Deserialize + Debug {

    /// Returns an iterator from which hits can be read
    pub fn iter(self, client: &mut Client, scroll: Duration) -> ScanIterator<T> {
        ScanIterator {
            scan_result: self,
            scroll:      scroll,
            client:      client,
            page:        vec![],
        }
    }

    /// Calls the `/_search/scroll` ES end-point for the next page
    pub fn scroll(&mut self,
                  client: &mut Client,
                  scroll: &Duration) -> Result<SearchResult<T>, EsError> {
        let url = format!("/_search/scroll?scroll={}&scroll_id={}",
                          scroll.to_string(),
                          self.scroll_id);
        let response = try!(client.get_op(&url));
        match response.status_code() {
            &StatusCode::Ok => {
                let search_result:SearchResultInterim<T> = try!(response.read_response());
                self.scroll_id = match search_result.scroll_id {
                    Some(ref id) => id.clone(),
                    None     => {
                        return Err(EsError::EsError("Expecting scroll_id".to_owned()))
                    }
                };
                println!("Scrolled: {:?}", search_result);
                Ok(search_result.finalize())
            },
            _               => {
                Err(EsError::EsError(format!("Unexpected status: {}",
                                             response.status_code())))
            }
        }
    }

    /// Calls ES to close the server-side part of the scan/scroll operation
    pub fn close(&self, client: &mut Client) -> Result<(), EsError> {
        let url = format!("/_search/scroll?scroll_id={}", self.scroll_id);
        let response = try!(client.delete_op(&url));
        match response.status_code() {
            &StatusCode::Ok       => Ok(()), // closed
            &StatusCode::NotFound => Ok(()), // previously closed
            _                     => Err(EsError::EsError(format!("Unexpected status: {}",
                                                                  response.status_code())))
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;
    extern crate regex;

    use ::Client;
    use ::tests::TestDocument;

    use ::operations::bulk::Action;
    use ::units::{Duration, JsonVal};

    use super::ScanResult;
    use super::SearchHitsHitsResult;
    use super::Sort;
    use super::Source;

//    use super::aggregations::{Aggregations, Min, Order, OrderKey, Terms};

    fn make_document(idx: i64) -> TestDocument {
        TestDocument::new()
            .with_str_field(&format!("BulkDoc: {}", idx))
            .with_int_field(idx)
    }

    fn setup_scan_data(client: &mut Client, index_name: &str) {
        let actions:Vec<Action<TestDocument>> = (0..1000).map(|idx| {
            Action::index(make_document(idx))
        }).collect();

        client.bulk(&actions)
            .with_index(index_name)
            .with_doc_type("doc_type")
            .send()
            .unwrap();

        client.refresh().with_indexes(&[index_name]).send().unwrap();
    }

    #[test]
    fn test_close() {
        let mut client = ::tests::make_client();
        let index_name = "tests_test_close";
        ::tests::clean_db(&mut client, index_name);
        setup_scan_data(&mut client, index_name);

        let indexes = [index_name];

        let scroll = Duration::minutes(1);
        let mut scan_result:ScanResult<TestDocument> = client.search_query()
            .with_indexes(&indexes)
            .with_size(100)
            .scan(&scroll)
            .unwrap();

        scan_result.scroll(&mut client, &scroll).unwrap();

        scan_result.close(&mut client).unwrap();
    }

    // #[test]
    // fn test_scan_and_scroll() {
    //     let mut client = ::tests::make_client();
    //     let index_name = "tests_test_scan_and_scroll";
    //     ::tests::clean_db(&mut client, index_name);
    //     setup_scan_data(&mut client, index_name);

    //     let indexes = [index_name];

    //     let mut scan_result = client.search_query()
    //         .with_indexes(&indexes)
    //         .with_size(100)
    //         .scan(Duration::minutes(1))
    //         .unwrap();

    //     assert_eq!(1000, scan_result.hits.total);
    //     let mut total = 0;

    //     loop {
    //         let page = scan_result.scroll(&mut client).unwrap();
    //         let page_total = page.hits.hits.len();
    //         total += page_total;
    //         if page_total == 0 && total == 1000 {
    //             break;
    //         }
    //         assert!(total <= 1000);
    //     }

    //     scan_result.close(&mut client).unwrap();
    // }

    // #[test]
    // fn test_scan_and_iterate() {
    //     let mut client = ::tests::make_client();
    //     let index_name = "tests_test_scan_and_iterate";
    //     ::tests::clean_db(&mut client, index_name);
    //     setup_scan_data(&mut client, index_name);

    //     let indexes = [index_name];

    //     let scan_result = client.search_query()
    //         .with_indexes(&indexes)
    //         .with_size(10)
    //         .scan(Duration::minutes(1))
    //         .unwrap();

    //     assert_eq!(1000, scan_result.hits.total);

    //     let hits:Vec<SearchHitsHitsResult> = scan_result.iter(&mut client)
    //         .take(200)
    //         .map(|hit| hit.unwrap())
    //         .collect();

    //     assert_eq!(200, hits.len());
    // }

    // #[test]
    // fn test_source_filter() {
    //     let mut client = ::tests::make_client();
    //     let index_name = "test_source_filter";
    //     ::tests::clean_db(&mut client, index_name);

    //     client.index(index_name, "test").with_doc(&make_document(100)).send().unwrap();
    //     client.refresh().with_indexes(&[index_name]).send().unwrap();

    //     let mut result = client.search_query()
    //         .with_indexes(&[index_name])
    //         .with_source(Source::include(&["str_field"]))
    //         .send()
    //         .unwrap();

    //     assert_eq!(1, result.hits.hits.len());
    //     let json = result.hits.hits.remove(0).source.unwrap();

    //     assert_eq!(true, json.find("str_field").is_some());
    //     assert_eq!(false, json.find("int_field").is_some());
    // }

    // #[test]
    // fn test_bucket_aggs() {
    //     let mut client = ::tests::make_client();
    //     let index_name = "test_bucket_aggs";
    //     ::tests::clean_db(&mut client, index_name);

    //     client.bulk(&[Action::index(TestDocument::new().with_str_field("A").with_int_field(2)),
    //                   Action::index(TestDocument::new().with_str_field("B").with_int_field(3)),
    //                   Action::index(TestDocument::new().with_str_field("A").with_int_field(1)),
    //                   Action::index(TestDocument::new().with_str_field("B").with_int_field(2))])
    //         .with_index(index_name)
    //         .with_doc_type("doc_type")
    //         .send()
    //         .unwrap();

    //     client.refresh().with_indexes(&[index_name]).send().unwrap();

    //     let aggs = Aggregations::from(("str",
    //                                    (Terms::new("str_field").with_order(Order::asc(OrderKey::Term)),
    //                                     Aggregations::from(("int",
    //                                                         Min::new("int_field"))))));

    //     let result = client.search_query()
    //         .with_indexes(&[index_name])
    //         .with_aggs(&aggs)
    //         .send()
    //         .unwrap();

    //     let buckets = &result.aggs_ref()
    //         .unwrap()
    //         .get("str")
    //         .unwrap()
    //         .as_terms()
    //         .unwrap()
    //         .buckets;

    //     let bucket_a = &buckets[0];
    //     let bucket_b = &buckets[1];

    //     assert_eq!(2, bucket_a.doc_count);
    //     assert_eq!(2, bucket_b.doc_count);

    //     let min_a = &bucket_a.aggs_ref()
    //         .unwrap()
    //         .get("int")
    //         .unwrap()
    //         .as_min()
    //         .unwrap()
    //         .value;

    //     let min_b = &bucket_b.aggs_ref()
    //         .unwrap()
    //         .get("int")
    //         .unwrap()
    //         .as_min()
    //         .unwrap()
    //         .value;

    //     match min_a {
    //         &JsonVal::F64(i) => assert_eq!(1.0, i),
    //         _                => panic!("Not an integer")
    //     }
    //     match min_b {
    //         &JsonVal::F64(i) => assert_eq!(2.0, i),
    //         _                => panic!("Not an integer")
    //     }
    // }

    // #[test]
    // fn test_aggs() {
    //     let mut client = ::tests::make_client();
    //     let index_name = "test_aggs";
    //     ::tests::clean_db(&mut client, index_name);

    //     client.bulk(&[Action::index(TestDocument::new().with_int_field(10)),
    //                   Action::index(TestDocument::new().with_int_field(1))])
    //         .with_index(index_name)
    //         .with_doc_type("doc_type")
    //         .send()
    //         .unwrap();

    //     client.refresh().with_indexes(&[index_name]).send().unwrap();

    //     let result = client.search_query()
    //         .with_indexes(&[index_name])
    //         .with_aggs(&Aggregations::from(("min_int_field", Min::new("int_field"))))
    //         .send()
    //         .unwrap();

    //     let min = &result.aggs_ref()
    //         .unwrap()
    //         .get("min_int_field")
    //         .unwrap()
    //         .as_min()
    //         .unwrap()
    //         .value;

    //     match min {
    //         &JsonVal::F64(i) => assert_eq!(1.0, i),
    //         _                => panic!("Not an integer")
    //     }
    // }

    // #[test]
    // fn test_sort() {
    //     let mut client = ::tests::make_client();
    //     let index_name = "test_sort";
    //     ::tests::clean_db(&mut client, index_name);

    //     client.bulk(&[Action::index(TestDocument::new().with_str_field("B").with_int_field(10)),
    //                   Action::index(TestDocument::new().with_str_field("C").with_int_field(4)),
    //                   Action::index(TestDocument::new().with_str_field("A").with_int_field(99))])
    //         .with_index(index_name)
    //         .with_doc_type("doc_type")
    //         .send()
    //         .unwrap();

    //     client.refresh().with_indexes(&[index_name]).send().unwrap();

    //     {
    //         let result = client.search_uri()
    //             .with_indexes(&[index_name])
    //             .with_sort(&Sort::field("str_field"))
    //             .send()
    //             .unwrap();

    //         let result_str:Vec<String> = result.hits.hits()
    //             .unwrap()
    //             .into_iter()
    //             .map(|doc:TestDocument| doc.str_field)
    //             .collect();

    //         let expected_result_str:Vec<String> = vec!["A", "B", "C"].into_iter()
    //             .map(|x| x.to_owned())
    //             .collect();

    //         assert_eq!(expected_result_str, result_str);
    //     }
    //     {
    //         let result = client.search_query()
    //             .with_indexes(&[index_name])
    //             .with_sort(&Sort::field("str_field"))
    //             .send()
    //             .unwrap();

    //         let result_str:Vec<String> = result.hits.hits()
    //             .unwrap()
    //             .into_iter()
    //             .map(|doc:TestDocument| doc.str_field)
    //             .collect();

    //         let expected_result_str:Vec<String> = vec!["A", "B", "C"].into_iter()
    //             .map(|x| x.to_owned())
    //             .collect();

    //         assert_eq!(expected_result_str,
    //                    result_str);
    //     }
    //     {
    //         let result = client.search_query()
    //             .with_indexes(&[index_name])
    //             .with_sort(&Sort::field("int_field"))
    //             .send()
    //             .unwrap();

    //         let result_str:Vec<String> = result.hits.hits()
    //             .unwrap()
    //             .into_iter()
    //             .map(|doc:TestDocument| doc.str_field)
    //             .collect();

    //         let expected_result_str:Vec<String> = vec!["C", "B", "A"].into_iter()
    //             .map(|x| x.to_owned())
    //             .collect();

    //         assert_eq!(expected_result_str,
    //                    result_str);
    //     }
    // }
}
