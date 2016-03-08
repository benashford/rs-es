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

use hyper::status::StatusCode;

use rustc_serialize::Decodable;
use rustc_serialize::json::{Json, ToJson};

use serde::ser::{Serialize, Serializer};

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

    pub fn send(&'b mut self) -> Result<SearchResult, EsError> {
        let url = format!("/{}/_search{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
                          self.options);
        info!("Searching with: {}", url);
        // TODO - fix below
        // let (status_code, result) = try!(self.client.get_op(&url));
        // info!("Search result (status: {}, result: {:?})", status_code, result);
        // match status_code {
        //     StatusCode::Ok => Ok(result.expect("No Json payload")),
        //     _              => Err(EsError::EsError(format!("Unexpected status: {}", status_code)))
        // }
        unimplemented!()
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
    source: Option<Source<'b>>

    // Aggregations
    // TODO - re-enable
    //aggs: Option<&'b aggregations::Aggregations<'b>>
}

// TODO - deprecated
impl<'a> ToJson for SearchQueryOperationBody<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("from".to_owned(), self.from.to_json());
        d.insert("size".to_owned(), self.size.to_json());
        optional_add!(self, d, query);
        optional_add!(self, d, timeout);
        optional_add!(self, d, terminate_after);
        optional_add!(self, d, stats);
        optional_add!(self, d, min_score);
        optional_add!(self, d, sort);
        optional_add!(self, d, track_scores);
        optional_add!(self, d, source, "_source");
        //optional_add!(self, d, aggs);
        Json::Object(d)
    }
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

    // TODO - re-enable
    // /// Specify any aggregations
    // pub fn with_aggs(&'b mut self, aggs: &'b aggregations::Aggregations) -> &'b mut Self {
    //     self.body.aggs = Some(aggs);
    //     self
    // }

    add_option!(with_routing, "routing");
    add_option!(with_search_type, "search_type");
    add_option!(with_query_cache, "query_cache");

    /// Performs the search with the specified query and options
    pub fn send(&'b mut self) -> Result<SearchResult, EsError> {
        let url = format!("/{}/_search{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
                          self.options);
        // let (status_code, result) = try!(self.client.post_body_op(&url, &self.body.to_json()));
        // match status_code {
        //     StatusCode::Ok => {
        //         // let result_json = result.expect("No Json payload");
        //         // let mut search_result = SearchResult::from(&result_json);
        //         // match self.body.aggs {
        //         //     Some(ref aggs) => {
        //         //         search_result.aggs = Some(AggregationsResult::from(aggs, &result_json));
        //         //     },
        //         //     _              => ()
        //         // }
        //         // Ok(search_result)
        //         unimplemented!()
        //     },
        //     _              => Err(EsError::EsError(format!("Unexpected status: {}", status_code)))
        // }
        unimplemented!()
    }

    /// Begins a scan with the specified query and options
    pub fn scan(&'b mut self, scroll: &'b Duration) -> Result<ScanResult, EsError> {
        self.options.push("search_type", "scan");
        self.options.push("scroll", scroll);
        let url = format!("/{}/_search{}",
                          format_indexes_and_types(&self.indexes, &self.doc_types),
                          self.options);
        let response = try!(self.client.post_body_op(&url, &self.body));
        match response.status_code() {
            &StatusCode::Ok => {
                // TODO - re-enable this
                //
                // match self.body.aggs {
                //     Some(ref aggs) => {
                //         scan_result.aggs = Some(AggregationsResult::from(aggs, &result_json));
                //     },
                //     _              => ()
                // }
                Ok(try!(response.read_response()))
            },
            &StatusCode::NotFound => {
                Err(EsError::EsServerError(format!("Not found: {:?}", response)))
            },
            _ => Err(EsError::EsError(format!("Unexpected status: {}", response.status_code())))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchHitsHitsResult {
    #[serde(rename="_index")]
    pub index:    String,
    #[serde(rename="_type")]
    pub doc_type: String,
    #[serde(rename="_id")]
    pub id:       String,
    #[serde(rename="_score")]
    pub score:    Option<f64>,
    // TODO - re-enable
    // #[serde(rename="_source")]
    // pub source:   Option<Json>,
    // #[serde(rename="_fields")]
    // pub fields:   Option<Json>
}

impl SearchHitsHitsResult {
    /// Get the source document as a struct, the raw JSON version is available
    /// directly from the source field
    pub fn source<T: Decodable>(self) -> Result<T, EsError> {
        // TODO - replace with Serde equivalent
        // match self.source {
        //     Some(source) => decode_json(source),
        //     None         => Err(EsError::EsError("No source field".to_owned()))
        // }
        unimplemented!()
    }
}

// TODO - deprecated
impl<'a> From<&'a Json> for SearchHitsHitsResult {
    fn from(r: &'a Json) -> SearchHitsHitsResult {
        SearchHitsHitsResult {
            index:    get_json_string!(r, "_index"),
            doc_type: get_json_string!(r, "_type"),
            id:       get_json_string!(r, "_id"),
            score:    {
                let s = r.find("_score").expect("No field '_score'");
                if s.is_f64() {
                    s.as_f64()
                } else {
                    None
                }
            },
            // TODO - decommissioned, ensure it (and the whole function is removed)
            // source:   r.find("_source").map(|s| s.clone()),
            // fields:   r.find("fields").map(|s| s.clone())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchHitsResult {
    pub total: u64,
    pub hits:  Vec<SearchHitsHitsResult>
}

impl SearchHitsResult {
    pub fn hits<T: Decodable>(self) -> Result<Vec<T>, EsError> {
        let mut r = Vec::with_capacity(self.hits.len());
        for hit in self.hits {
            r.push(try!(hit.source()));
        }
        Ok(r)
    }
}

impl<'a> From<&'a Json> for SearchHitsResult {
    fn from(r: &'a Json) -> SearchHitsResult {
        SearchHitsResult {
            total: get_json_u64!(r, "total"),
            hits:  r.find("hits")
                .expect("No field 'hits'")
                .as_array()
                .expect("Field 'hits' is not an array")
                .iter()
                .map(|j| SearchHitsHitsResult::from(j))
                .collect()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub took:      u64,
    pub timed_out: bool,

    #[serde(rename="_shards")]
    pub shards:    ShardCountResult,
    pub hits:      SearchHitsResult,

    /// Optional field populated if aggregations are specified
    pub aggs:      Option<AggregationsResult>,

    /// Optional field populated during scanning and scrolling
    #[serde(rename="_scroll_id")]
    pub scroll_id: Option<String>
}

impl SearchResult {
    /// Take a reference to any aggregations in this result
    pub fn aggs_ref<'a>(&'a self) -> Option<&'a AggregationsResult> {
        self.aggs.as_ref()
    }
}

// TODO - deprecated
impl<'a> From<&'a Json> for SearchResult {
    fn from(r: &'a Json) -> SearchResult {
        SearchResult {
            took:      get_json_u64!(r, "took"),
            timed_out: get_json_bool!(r, "timed_out"),
            shards:    decode_json(r.find("_shards")
                                   .expect("No field '_shards'")
                                   .clone()).unwrap(),
            hits:      SearchHitsResult::from(r.find("hits")
                                              .expect("No field 'hits'")),
            aggs:      None,
            scroll_id: None
        }
    }
}

pub struct ScanIterator<'a> {
    scan_result: ScanResult,
    scroll:      Duration,
    client:      &'a mut Client,
    page:        Vec<SearchHitsHitsResult>
}

impl<'a> ScanIterator<'a> {
    /// Fetch the next page and return the first hit, or None if there are no hits
    fn next_page(&mut self) -> Option<Result<SearchHitsHitsResult, EsError>> {
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

impl<'a> Drop for ScanIterator<'a> {
    fn drop(&mut self) {
        match self.scan_result.close(self.client) {
            Ok(_)  => (),
            Err(e) => {
                error!("Cannot close scroll: {}", e);
            }
        }
    }
}

impl<'a> Iterator for ScanIterator<'a> {
    type Item = Result<SearchHitsHitsResult, EsError>;

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
pub struct ScanResult {
    #[serde(rename="_scroll_id")]
    scroll_id:     String,
    pub took:      u64,
    pub timed_out: bool,
    #[serde(rename="_shards")]
    pub shards:    ShardCountResult,
    pub hits:      SearchHitsResult,
    pub aggs:      Option<AggregationsResult>
}

impl ScanResult {
    // TODO - deprecated, replace with Serde
    fn from<'b>(r: &'b Json) -> ScanResult {
        ScanResult {
            scroll_id: get_json_string!(r, "_scroll_id"),
            took:      get_json_u64!(r, "took"),
            timed_out: get_json_bool!(r, "timed_out"),
            shards:    decode_json(r.find("_shards")
                                   .unwrap()
                                   .clone()).unwrap(),
            hits:      SearchHitsResult::from(r.find("hits")
                                              .unwrap()),
            aggs:      None
        }
    }

    /// Returns an iterator from which hits can be read
    pub fn iter(self, client: &mut Client, scroll: Duration) -> ScanIterator {
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
                  scroll: &Duration) -> Result<SearchResult, EsError> {
        let url = format!("/_search/scroll?scroll={}&scroll_id={}",
                          scroll.to_string(),
                          self.scroll_id);
        let response = try!(client.get_op(&url));
        match response.status_code() {
            &StatusCode::Ok => {
                let search_result:SearchResult = try!(response.read_response());
                self.scroll_id = match search_result.scroll_id {
                    Some(ref id) => id.clone(),
                    None     => {
                        return Err(EsError::EsError("Expecting scroll_id".to_owned()))
                    }
                };
                println!("Scrolled: {:?}", search_result);
                Ok(search_result)
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

    use super::SearchHitsHitsResult;
    use super::Sort;
    use super::Source;

    use super::aggregations::{Aggregations, Min, Order, OrderKey, Terms};

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

    // #[test]
    // fn test_close() {
    //     let mut client = ::tests::make_client();
    //     let index_name = "tests_test_close";
    //     ::tests::clean_db(&mut client, index_name);
    //     setup_scan_data(&mut client, index_name);

    //     let indexes = [index_name];

    //     let mut scan_result = client.search_query()
    //         .with_indexes(&indexes)
    //         .with_size(100)
    //         .scan(Duration::minutes(1))
    //         .unwrap();

    //     scan_result.scroll(&mut client).unwrap();

    //     scan_result.close(&mut client).unwrap();
    // }

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
