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

//! Bucket-based aggregations

use std::collections::HashMap;
use std::marker::PhantomData;

use serde::ser::{Serialize, Serializer, SerializeMap};
use serde_json::Value;

use ::error::EsError;
use ::json::{MergeSerialize, serialize_map_optional_kv, ShouldSkip};
use ::query;
use ::units::{DistanceType, DistanceUnit, Duration, JsonVal, Location, OneOrMany};

use super::{Aggregation,
            Aggregations,
            AggregationResult,
            AggregationsResult,
            object_to_result};
use super::common::{Agg, Script};

// Some options

#[derive(Debug)]
pub enum ExecutionHint {
    Map,
    GlobalOrdinalsLowCardinality,
    GlobalOrdinals,
    GlobalOrdinalsHash
}

impl Serialize for ExecutionHint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        use self::ExecutionHint::*;
        match self {
            &Map => "map",
            &GlobalOrdinalsLowCardinality => "global_ordinals_low_cardinality",
            &GlobalOrdinals => "global_ordinals",
            &GlobalOrdinalsHash => "global_ordinals_hash"
        }.serialize(serializer)
    }
}

// Common features

macro_rules! bucket_agg {
    ($b:ident) => {
        impl<'a> From<($b<'a>, Aggregations<'a>)> for Aggregation<'a> {
            fn from(from: ($b<'a>, Aggregations<'a>)) -> Aggregation<'a> {
                Aggregation::Bucket(BucketAggregation::$b(from.0), Some(from.1))
            }
        }

        impl<'a> From<$b<'a>> for Aggregation<'a> {
            fn from(from: $b<'a>) -> Aggregation<'a> {
                Aggregation::Bucket(BucketAggregation::$b(from), None)
            }
        }
    }
}

macro_rules! fos_bucket_agg {
    ($b:ident) => {
        agg!($b);
        bucket_agg!($b);
    }
}

/// Global aggregation, defines a single global bucket.  Can only be used as a
/// top-level aggregation.  See: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-global-aggregation.html
#[derive(Debug, Serialize)]
pub struct Global<'a> {
    /// Needed for lifecycle reasons
    phantom: PhantomData<&'a str>
}

impl<'a> Global<'a> {
    pub fn new() -> Global<'a> {
        Global {
            phantom: PhantomData
        }
    }
}

bucket_agg!(Global);

/// Filter aggregation
// TODO - Query is now an enum with a `Box` it might be simpler, with little
// side-effects to own it instead
#[derive(Debug, Serialize)]
pub struct Filter<'a> {
    filter: &'a query::Query
}

impl<'a> Filter<'a> {
    pub fn new(filter: &'a query::Query) -> Filter<'a> {
        Filter {
            filter: filter
        }
    }
}

bucket_agg!(Filter);

/// Filters aggregation
#[derive(Debug, Serialize)]
pub struct Filters<'a> {
    filters: HashMap<&'a str, &'a query::Query>
}

impl<'a> Filters<'a> {
    pub fn new(filters: HashMap<&'a str, &'a query::Query>) -> Filters<'a> {
        Filters {
            filters: filters
        }
    }
}

impl<'a> From<Vec<(&'a str, &'a query::Query)>> for Filters<'a> {
    fn from(from: Vec<(&'a str, &'a query::Query)>) -> Filters<'a> {
        let mut filters = HashMap::with_capacity(from.len());
        for (k, v) in from {
            filters.insert(k, v);
        }
        Filters::new(filters)
    }
}

bucket_agg!(Filters);

/// Missing aggregation
#[derive(Debug, Serialize)]
pub struct Missing<'a> {
    pub field: &'a str
}

impl<'a> Missing<'a> {
    pub fn new(field: &'a str) -> Missing<'a> {
        Missing {
            field: field
        }
    }
}

bucket_agg!(Missing);

/// Nested aggregation
#[derive(Debug, Serialize)]
pub struct Nested<'a> {
    pub path: &'a str
}

impl<'a> Nested<'a> {
    pub fn new(path: &'a str) -> Nested<'a> {
        Nested {
            path: path
        }
    }
}

bucket_agg!(Nested);

/// Reverse nested aggregation, will produce an error if used anywhere other than
/// inside a nested aggregation.
///
/// See: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-reverse-nested-aggregation.html
#[derive(Debug, Serialize)]
pub struct ReverseNested<'a> {
    /// Needed for lifecycle reasons
    phantom: PhantomData<&'a str>
}

impl<'a> ReverseNested<'a> {
    pub fn new() -> ReverseNested<'a> {
        ReverseNested {
            phantom: PhantomData
        }
    }
}

bucket_agg!(ReverseNested);

/// Children aggregation - sub-aggregations run against the child document
#[derive(Debug, Serialize)]
pub struct Children<'a> {
    #[serde(rename="type")]
    doc_type: &'a str
}

impl<'a> Children<'a> {
    pub fn new(doc_type: &'a str) -> Children<'a> {
        Children {
            doc_type: doc_type
        }
    }
}

bucket_agg!(Children);

/// Order - used for some bucketing aggregations to determine the order of
/// buckets
#[derive(Debug, Eq, Hash, PartialEq)]
pub enum OrderKey<'a> {
    Count,
    Key,
    Term,
    Expr(&'a str)
}

impl<'a> From<&'a str> for OrderKey<'a> {
    fn from(from: &'a str) -> OrderKey<'a> {
        OrderKey::Expr(from)
    }
}

impl<'a> AsRef<str> for OrderKey<'a> {
    fn as_ref(&self) -> &str {
        use self::OrderKey::*;
        match self {
            &Count   => "_count",
            &Key     => "_key",
            &Term    => "_term",
            &Expr(e) => e
        }
    }
}

/// Used to define the ordering of buckets in a some bucketted aggregations
///
/// # Examples
///
/// ```
/// use rs_es::operations::search::aggregations::bucket::{Order, OrderKey};
///
/// let order1 = Order::asc(OrderKey::Count);
/// let order2 = Order::desc("field_name");
/// ```
///
/// The first will produce a JSON fragment: `{"_count": "asc"}`; the second will
/// produce a JSON fragment: `{"field_name", "desc"}`
#[derive(Debug)]
pub struct Order<'a>(OrderKey<'a>, super::super::Order);

impl<'a> Serialize for Order<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {

        let mut d = HashMap::new();
        d.insert(self.0.as_ref(), &self.1);
        d.serialize(serializer)
    }
}

impl<'a> Order<'a> {
    /// Create an `Order` ascending
    pub fn asc<O: Into<OrderKey<'a>>>(key: O) -> Order<'a> {
        Order(key.into(), super::super::Order::Asc)
    }

    /// Create an `Order` descending
    pub fn desc<O: Into<OrderKey<'a>>>(key: O) -> Order<'a> {
        Order(key.into(), super::super::Order::Desc)
    }
}

/// Terms aggregation
#[derive(Debug)]
pub struct Terms<'a>(Agg<'a, TermsInner<'a>>);

#[derive(Debug, Default)]
pub struct TermsInner<'a> {
    size: Option<u64>,
    shard_size: Option<u64>,
    order: Option<OneOrMany<Order<'a>>>,
    min_doc_count: Option<u64>,
    shard_min_doc_count: Option<u64>,
    include: Option<OneOrMany<&'a str>>,
    exclude: Option<OneOrMany<&'a str>>,
    execution_hint: Option<ExecutionHint>
}

impl<'a> Terms<'a> {
    add_extra_option!(with_size, size, u64);
    add_extra_option!(with_shard_size, shard_size, u64);
    add_extra_option!(with_order, order, OneOrMany<Order<'a>>);
    add_extra_option!(with_min_doc_count, min_doc_count, u64);
    add_extra_option!(with_shard_min_doc_count, shard_min_doc_count, u64);
    add_extra_option!(with_include, include, OneOrMany<&'a str>);
    add_extra_option!(with_exclude, exclude, OneOrMany<&'a str>);
    add_extra_option!(with_execution_hint, execution_hint, ExecutionHint);
}

impl<'a> MergeSerialize for TermsInner<'a> {
    fn merge_serialize<S>(&self,
                          serializer: &mut S) -> Result<(), S::Error>
        where S: SerializeMap {

        try!(serialize_map_optional_kv(serializer, "size", &self.size));
        try!(serialize_map_optional_kv(serializer, "shard_size", &self.shard_size));
        try!(serialize_map_optional_kv(serializer, "order", &self.order));
        try!(serialize_map_optional_kv(serializer, "min_doc_count", &self.min_doc_count));
        try!(serialize_map_optional_kv(serializer,
                                       "shard_min_doc_count",
                                       &self.shard_min_doc_count));
        try!(serialize_map_optional_kv(serializer, "include", &self.include));
        try!(serialize_map_optional_kv(serializer, "exclude", &self.exclude));
        try!(serialize_map_optional_kv(serializer, "execution_hint", &self.execution_hint));
        Ok(())
    }
}

fos_bucket_agg!(Terms);

// Range aggs and dependencies

/// A specific range, there will be many of these making up a range aggregation
#[derive(Debug, Default, Serialize)]
pub struct RangeInst<'a> {
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    from: Option<JsonVal>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    to:   Option<JsonVal>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    key:  Option<&'a str>
}

impl<'a> RangeInst<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    add_field!(with_from, from, JsonVal);
    add_field!(with_to, to, JsonVal);
    add_field!(with_key, key, &'a str);
}

/// Range aggregations
///
/// The keyed option will always be used.
///
/// https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-range-aggregation.html
#[derive(Debug)]
pub struct Range<'a>(Agg<'a, RangeInner<'a>>);

#[derive(Debug, Serialize)]
pub struct RangeInner<'a> {
    keyed: bool,
    ranges: Vec<RangeInst<'a>>
}

impl<'a> Range<'a> {
    pub fn with_keyed<B: Into<bool>>(mut self, keyed: B) -> Self {
        self.0.extra.keyed = keyed.into();
        self
    }

    pub fn with_ranges<R>(mut self, ranges: R) -> Self
        where R: Into<Vec<RangeInst<'a>>> {

        self.0.extra.ranges = ranges.into();
        self
    }
}

impl<'a> MergeSerialize for RangeInner<'a> {
    fn merge_serialize<S>(&self,
                          serializer: &mut S) -> Result<(), S::Error>
        where S: SerializeMap {
        serializer.serialize_entry("keyed", &self.keyed)?;
        serializer.serialize_entry("ranges", &self.ranges)
    }
}

impl<'a> Default for RangeInner<'a> {
    fn default() -> Self {
        RangeInner {
            keyed: true,
            ranges: Default::default()
        }
    }
}

fos_bucket_agg!(Range);

/// A specific element of a range for a `DateRange` aggregation
#[derive(Debug, Default, Serialize)]
pub struct DateRangeInst<'a> {
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    from: Option<&'a str>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    to:   Option<&'a str>
}

impl<'a> DateRangeInst<'a> {
    pub fn new() -> DateRangeInst<'a> {
        Default::default()
    }

    add_field!(with_from, from, &'a str);
    add_field!(with_to, to, &'a str);
}

/// Date range aggregation.  See: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-daterange-aggregation.html
#[derive(Debug)]
pub struct DateRange<'a>(Agg<'a, DateRangeInner<'a>>);

#[derive(Debug, Default)]
pub struct DateRangeInner<'a> {
    format: Option<&'a str>,
    ranges: Vec<DateRangeInst<'a>>
}

impl<'a> DateRange<'a> {
    add_extra_option!(with_format, format, &'a str);

    pub fn with_ranges<A: Into<Vec<DateRangeInst<'a>>>>(mut self, ranges: A) -> Self {
        self.0.extra.ranges = ranges.into();
        self
    }
}

impl<'a> MergeSerialize for DateRangeInner<'a> {
    fn merge_serialize<S>(&self,
                          serializer: &mut S) -> Result<(), S::Error>
        where S: SerializeMap {

        try!(serialize_map_optional_kv(serializer, "format", &self.format));
        serializer.serialize_entry("ranges", &self.ranges)
    }
}

fos_bucket_agg!(DateRange);

/// Histogram aggregation.
#[derive(Debug, Serialize)]
pub struct ExtendedBounds {
    min: i64,
    max: i64
}

impl ExtendedBounds {
    pub fn new(min: i64, max: i64) -> ExtendedBounds {
        ExtendedBounds {
            min: min,
            max: max
        }
    }
}

impl From<(i64, i64)> for ExtendedBounds {
    fn from(from: (i64, i64)) -> ExtendedBounds {
        ExtendedBounds::new(from.0, from.1)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Histogram<'a> {
    field:           &'a str,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    interval:        Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    min_doc_count:   Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    extended_bounds: Option<ExtendedBounds>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    order:           Option<Order<'a>>
}

impl<'a> Histogram<'a> {
    pub fn new(field: &'a str) -> Histogram<'a> {
        Histogram {
            field: field,
            ..Default::default()
        }
    }

    add_field!(with_interval, interval, u64);
    add_field!(with_min_doc_count, min_doc_count, u64);
    add_field!(with_extended_bounds, extended_bounds, ExtendedBounds);
    add_field!(with_order, order, Order<'a>);
}

bucket_agg!(Histogram);

/// Date histogram and related fields
#[derive(Debug)]
pub enum TimeZone<'a> {
    Offset(u64),
    Str(&'a str)
}

impl<'a> Serialize for TimeZone<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        use self::TimeZone::*;
        match self {
            &Offset(offset) => offset.serialize(serializer),
            &Str(tz_str) => tz_str.serialize(serializer)
        }
    }
}

impl<'a> From<&'a str> for TimeZone<'a> {
    fn from(from: &'a str) -> TimeZone<'a> {
        TimeZone::Str(from)
    }
}

impl<'a> From<u64> for TimeZone<'a> {
    fn from(from: u64) -> TimeZone<'a> {
        TimeZone::Offset(from)
    }
}

#[derive(Debug)]
pub enum Interval {
    Year,
    Quarter,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second
}

impl Default for Interval {
    fn default() -> Self {
        Interval::Day
    }
}

impl Serialize for Interval {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        use self::Interval::*;
        match *self {
            Year => "year",
            Quarter => "quarter",
            Month => "month",
            Week => "week",
            Day => "day",
            Hour => "hour",
            Minute => "minute",
            Second => "second"
        }.serialize(serializer)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct DateHistogram<'a> {
    field: &'a str,
    interval: Interval,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    time_zone: Option<TimeZone<'a>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    offset: Option<Duration>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    format: Option<&'a str>,
}

impl<'a> DateHistogram<'a> {
    pub fn new<I>(field: &'a str, interval: I) -> DateHistogram<'a>
    where I: Into<Interval> {
        DateHistogram {
            field: field,
            interval: interval.into(),
            ..Default::default()
        }
    }

    add_field!(with_time_zone, time_zone, TimeZone<'a>);
    add_field!(with_offset, offset, Duration);
    add_field!(with_format, format, &'a str);
}

bucket_agg!(DateHistogram);

#[derive(Debug, Default, Serialize)]
pub struct GeoDistanceInst {
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    from: Option<f64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    to:   Option<f64>
}

impl GeoDistanceInst {
    pub fn new() -> GeoDistanceInst {
        Default::default()
    }

    add_field!(with_from, from, f64);
    add_field!(with_to, to, f64);
}

#[derive(Debug, Serialize)]
pub struct GeoDistance<'a> {
    field:         &'a str,
    origin:        &'a Location,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    unit:          Option<DistanceUnit>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    distance_type: Option<DistanceType>,
    ranges:        &'a [GeoDistanceInst]
}

impl<'a> GeoDistance<'a> {
    pub fn new(field: &'a str,
               origin: &'a Location,
               ranges: &'a [GeoDistanceInst]) -> GeoDistance<'a> {
        GeoDistance {
            field:         field,
            origin:        origin,
            unit:          None,
            distance_type: None,
            ranges:        ranges,
        }
    }

    add_field!(with_unit, unit, DistanceUnit);
    add_field!(with_distance_type, distance_type, DistanceType);

    pub fn inst() -> GeoDistanceInst {
        GeoDistanceInst::new()
    }
}

bucket_agg!(GeoDistance);

/// Geohash aggregation
#[derive(Debug, Default, Serialize)]
pub struct GeohashGrid<'a> {
    field:      &'a str,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    precision:  Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    size:       Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    shard_size: Option<u64>
}

impl<'a> GeohashGrid<'a> {
    pub fn new(field: &'a str) -> Self {
        GeohashGrid {
            field: field,
            ..Default::default()
        }
    }

    add_field!(with_precision, precision, u64);
    add_field!(with_size, size, u64);
    add_field!(with_shard_size, shard_size, u64);
}

bucket_agg!(GeohashGrid);

/// The set of bucket aggregations
#[derive(Debug)]
pub enum BucketAggregation<'a> {
    Global(Global<'a>),
    Filter(Filter<'a>),
    Filters(Filters<'a>),
    Missing(Missing<'a>),
    Nested(Nested<'a>),
    ReverseNested(ReverseNested<'a>),
    Children(Children<'a>),
    Terms(Terms<'a>),
    Range(Range<'a>),
    DateRange(DateRange<'a>),
    Histogram(Histogram<'a>),
    DateHistogram(DateHistogram<'a>),
    GeoDistance(GeoDistance<'a>),
    GeohashGrid(GeohashGrid<'a>)
}

impl<'a> BucketAggregation<'a> {
    pub fn details(&self) -> &'static str {
        use self::BucketAggregation::*;
        match self {
            &Global(_) => "global",
            &Filter(_) => "filter",
            &Filters(_) => "filters",
            &Missing(_) => "missing",
            &Nested(_) => "nested",
            &ReverseNested(_) => "reverse_nested",
            &Children(_) => "children",
            &Terms(_) => "terms",
            &Range(_) => "range",
            &DateRange(_) => "date_range",
            &Histogram(_) => "histogram",
            &DateHistogram(_) => "date_histogram",
            &GeoDistance(_) => "geo_distance",
            &GeohashGrid(_) => "geohash_grid"
        }
    }
}

impl<'a> Serialize for BucketAggregation<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {
        use self::BucketAggregation::*;
        match self {
            &Global(ref g) => g.serialize(serializer),
            &Filter(ref f) => f.serialize(serializer),
            &Filters(ref f) => f.serialize(serializer),
            &Missing(ref m) => m.serialize(serializer),
            &Nested(ref n) => n.serialize(serializer),
            &ReverseNested(ref r) => r.serialize(serializer),
            &Children(ref c) => c.serialize(serializer),
            &Terms(ref t) => t.serialize(serializer),
            &Range(ref r) => r.serialize(serializer),
            &DateRange(ref d) => d.serialize(serializer),
            &Histogram(ref h) => h.serialize(serializer),
            &DateHistogram(ref d) => d.serialize(serializer),
            &GeoDistance(ref g) => g.serialize(serializer),
            &GeohashGrid(ref g) => g.serialize(serializer)
        }
    }
}

// results
#[derive(Debug)]
pub enum BucketAggregationResult {
    Global(GlobalResult),
    Filter(FilterResult),
    Filters(FiltersResult),
    Missing(MissingResult),
    Nested(NestedResult),
    ReverseNested(ReverseNestedResult),
    Children(ChildrenResult),
    Terms(TermsResult),
    Range(RangeResult),
    DateRange(DateRangeResult),
    Histogram(HistogramResult),
    DateHistogram(DateHistogramResult),
    GeoDistance(GeoDistanceResult),
    GeohashGrid(GeohashGridResult)
}

impl BucketAggregationResult {
    pub fn from<'a>(ba: &BucketAggregation<'a>,
                    json: &Value,
                    aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        use self::BucketAggregation::*;
        Ok(match ba {
            &Global(_) => {
                BucketAggregationResult::Global(try!(GlobalResult::from(json, aggs)))
            },
            &BucketAggregation::Filter(_) => {
                BucketAggregationResult::Filter(try!(FilterResult::from(json, aggs)))
            },
            &BucketAggregation::Filters(_) => {
                BucketAggregationResult::Filters(try!(FiltersResult::from(json, aggs)))
            },
            &BucketAggregation::Missing(_) => {
                BucketAggregationResult::Missing(try!(MissingResult::from(json, aggs)))
            },
            &BucketAggregation::Nested(_) => {
                BucketAggregationResult::Nested(try!(NestedResult::from(json, aggs)))
            },
            &BucketAggregation::ReverseNested(_) => {
                BucketAggregationResult::ReverseNested(try!(ReverseNestedResult::from(json,
                                                                                      aggs)))
            },
            &BucketAggregation::Children(_) => {
                BucketAggregationResult::Children(try!(ChildrenResult::from(json, aggs)))
            },
            &BucketAggregation::Terms(_) => {
                BucketAggregationResult::Terms(try!(TermsResult::from(json, aggs)))
            },
            &BucketAggregation::Range(_) => {
                BucketAggregationResult::Range(try!(RangeResult::from(json, aggs)))
            },
            &BucketAggregation::DateRange(_) => {
                BucketAggregationResult::DateRange(try!(DateRangeResult::from(json, aggs)))
            },
            &BucketAggregation::Histogram(_) => {
                BucketAggregationResult::Histogram(try!(HistogramResult::from(json, aggs)))
            },
            &BucketAggregation::DateHistogram(_) => {
                BucketAggregationResult::DateHistogram(try!(DateHistogramResult::from(json,
                                                                                      aggs)))
            },
            &BucketAggregation::GeoDistance(_) => {
                BucketAggregationResult::GeoDistance(try!(GeoDistanceResult::from(json,
                                                                                  aggs)))
            },
            &BucketAggregation::GeohashGrid(_) => {
                BucketAggregationResult::GeohashGrid(try!(GeohashGridResult::from(json,
                                                                                  aggs)))
            }
        })
    }
}

macro_rules! bucket_agg_as {
    ($n:ident,$t:ident,$rt:ty) => {
        agg_as!($n,Bucket,BucketAggregationResult,$t,$rt);
    }
}

impl AggregationResult {
    bucket_agg_as!(as_global, Global, GlobalResult);
    bucket_agg_as!(as_filter, Filter, FilterResult);
    bucket_agg_as!(as_filters, Filters, FiltersResult);
    bucket_agg_as!(as_missing, Missing, MissingResult);
    bucket_agg_as!(as_nested, Nested, NestedResult);
    bucket_agg_as!(as_reverse_nested, ReverseNested, ReverseNestedResult);
    bucket_agg_as!(as_children, Children, ChildrenResult);
    bucket_agg_as!(as_terms, Terms, TermsResult);
    bucket_agg_as!(as_range, Range, RangeResult);
    bucket_agg_as!(as_date_range, DateRange, DateRangeResult);
    bucket_agg_as!(as_histogram, Histogram, HistogramResult);
    bucket_agg_as!(as_date_histogram, DateHistogram, DateHistogramResult);
    bucket_agg_as!(as_geo_distance, GeoDistance, GeoDistanceResult);
    bucket_agg_as!(as_geohash_grid, GeohashGrid, GeohashGridResult);
}

// Result reading

/// Macros for buckets to return a reference to the sub-aggregations
macro_rules! add_aggs_ref {
    () => {
        pub fn aggs_ref<'a>(&'a self) -> Option<&'a AggregationsResult> {
            self.aggs.as_ref()
        }
    }
}

macro_rules! return_error {
    ($e:expr) => {
        return Err(EsError::EsError($e))
    }
}

macro_rules! return_no_field {
    ($f:expr) => {
        return_error!(format!("No valid field: {}", $f))
    }
}

macro_rules! optional_json {
    ($j:ident, $f:expr, $a:ident) => {
        match $j.get($f) {
            Some(val) => {
                match val.$a() {
                    Some(field_val) => Some(field_val),
                    None => return_no_field!($f)
                }
            },
            None => None
        }
    }
}

macro_rules! from_json {
    ($j:ident, $f:expr, $a:ident) => {
        match $j.get($f) {
            Some(val) => {
                match val.$a() {
                    Some(field_val) => field_val,
                    None => return_no_field!($f)
                }
            },
            None => return_no_field!($f)
        }
    }
}

macro_rules! extract_aggs {
    ($j:ident, $a:ident) => {
        match $a {
            &Some(ref aggs) => {
                let obj = match $j.as_object() {
                    Some(field_val) => field_val,
                    None => return_error!("Not an object".to_owned())
                };
                Some(try!(object_to_result(aggs, obj)))
            },
            &None => None
        }
    }
}

macro_rules! from_bucket_vector {
    ($j:ident, $b:ident, $m:expr) => {
        {
            let raw_buckets = from_json!($j, "buckets", as_array);
            let mut buckets = Vec::with_capacity(raw_buckets.len());
            for $b in raw_buckets.iter() {
                buckets.push(try!($m))
            }
            buckets
        }
    }
}

/// Global result
#[derive(Debug)]
pub struct GlobalResult {
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl GlobalResult {
    fn from(json: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(GlobalResult {
            doc_count: from_json!(json, "doc_count", as_u64),
            aggs: extract_aggs!(json, aggs)
        })
    }

    add_aggs_ref!();
}

/// Filter result
#[derive(Debug)]
pub struct FilterResult {
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl FilterResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(FilterResult {
            doc_count: from_json!(from, "doc_count", as_u64),
            aggs: extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct FiltersBucketResult {
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl FiltersBucketResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(FiltersBucketResult {
            doc_count: from_json!(from, "doc_count", as_u64),
            aggs: extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct FiltersResult {
    pub buckets: HashMap<String, FiltersBucketResult>
}

impl FiltersResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(FiltersResult {
            buckets: {
                // In this case "buckets" is a JSON object, so our `from_bucket_vector`
                // macro is not helpful
                let raw_buckets = from_json!(from, "buckets", as_object);
                let mut buckets = HashMap::with_capacity(raw_buckets.len());
                for (k, v) in raw_buckets.iter() {
                    buckets.insert(k.clone(), try!(FiltersBucketResult::from(v, aggs)));
                }
                buckets
            }
        })
    }
}

#[derive(Debug)]
pub struct MissingResult {
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl MissingResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(MissingResult {
            doc_count: from_json!(from, "doc_count", as_u64),
            aggs: extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct NestedResult {
    pub aggs: Option<AggregationsResult>
}

impl NestedResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(NestedResult {
            aggs: extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct ReverseNestedResult {
    pub aggs: Option<AggregationsResult>
}

impl ReverseNestedResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(ReverseNestedResult {
            aggs: extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct ChildrenResult {
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl ChildrenResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(ChildrenResult {
            doc_count: from_json!(from, "doc_count", as_u64),
            aggs: extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

/// Terms result
#[derive(Debug)]
pub struct TermsResult {
    pub doc_count_error_upper_bound: u64,
    pub sum_other_doc_count: u64,
    pub buckets: Vec<TermsBucketResult>
}

impl TermsResult {
    fn from(json: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(TermsResult {
            doc_count_error_upper_bound: from_json!(json,
                                                    "doc_count_error_upper_bound",
                                                    as_u64),
            sum_other_doc_count: from_json!(json, "sum_other_doc_count", as_u64),
            buckets: from_bucket_vector!(json, bucket, TermsBucketResult::from(bucket,
                                                                               aggs))
        })
    }
}

#[derive(Debug)]
pub struct TermsBucketResult {
    pub key: JsonVal,
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl TermsBucketResult {
    fn from(json: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        info!("Creating TermsBucketResult from: {:?} with {:?}", json, aggs);

        Ok(TermsBucketResult {
            key: try!(JsonVal::from(match json.get("key") {
                Some(key) => key,
                None => return_error!("No 'key'".to_owned())
            })),
            doc_count: from_json!(json, "doc_count", as_u64),
            aggs: extract_aggs!(json, aggs)
        })
    }

    add_aggs_ref!();
}

// Range result objects

#[derive(Debug)]
pub struct RangeBucketResult {
    pub from:      Option<JsonVal>,
    pub to:        Option<JsonVal>,
    pub doc_count: u64,
    pub aggs:      Option<AggregationsResult>
}

impl RangeBucketResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(RangeBucketResult {
            from:      from.get("from").and_then(|from| Some(from.into())),
            to:        from.get("to").and_then(|to| Some(to.into())),
            doc_count: from_json!(from, "doc_count", as_u64),
            aggs:      extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct RangeResult {
    pub buckets: HashMap<String, RangeBucketResult>,
}

impl RangeResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        let bucket_obj = from_json!(from, "buckets", as_object);
        let mut buckets = HashMap::with_capacity(bucket_obj.len());

        for (k, v) in bucket_obj.into_iter() {
            buckets.insert(k.clone(), try!(RangeBucketResult::from(v, aggs)));
        }

        Ok(RangeResult {
            buckets: buckets
        })
    }
}

// Date range result objects

#[derive(Debug)]
pub struct DateRangeBucketResult {
    pub from:           Option<f64>,
    pub from_as_string: Option<String>,
    pub to:             Option<f64>,
    pub to_as_string:   Option<String>,
    pub doc_count:      u64,
    pub aggs:           Option<AggregationsResult>
}

impl DateRangeBucketResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(DateRangeBucketResult {
            from:           optional_json!(from, "from", as_f64),
            from_as_string: optional_json!(from, "from_as_string", as_str).map(|s| s.to_owned()),
            to:             optional_json!(from, "to", as_f64),
            to_as_string:   optional_json!(from, "to_as_string", as_str).map(|s| s.to_owned()),
            doc_count:      from_json!(from, "doc_count", as_u64),
            aggs:           extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct DateRangeResult {
    pub buckets: Vec<DateRangeBucketResult>
}

impl DateRangeResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(DateRangeResult {
            buckets: from_bucket_vector!(from, bucket, DateRangeBucketResult::from(bucket,
                                                                                   aggs))
        })
    }
}

/// Used for histogram results
#[derive(Debug)]
pub struct HistogramBucketResult {
    pub key: String,
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl HistogramBucketResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(HistogramBucketResult {
            key: from_json!(from, "key", as_str).to_owned(),
            doc_count: from_json!(from, "doc_count", as_u64),
            aggs: extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct HistogramResult {
    pub buckets: Vec<HistogramBucketResult>
}

impl HistogramResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(HistogramResult {
            buckets: from_bucket_vector!(from,
                                         bucket,
                                         HistogramBucketResult::from(bucket, aggs))
        })
    }
}

// Date histogram results
#[derive(Debug)]
pub struct DateHistogramBucketResult {
    pub key_as_string: String,
    pub key: u64,
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl DateHistogramBucketResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(DateHistogramBucketResult {
            key_as_string: from_json!(from, "key_as_string", as_str).to_owned(),
            key: from_json!(from, "key", as_u64),
            doc_count: from_json!(from, "doc_count", as_u64),
            aggs: extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct DateHistogramResult {
    pub buckets: Vec<DateHistogramBucketResult>
}

impl DateHistogramResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(DateHistogramResult {
            buckets: from_bucket_vector!(from,
                                         bucket,
                                         DateHistogramBucketResult::from(bucket, aggs))
        })
    }
}

// GeoDistance results
#[derive(Debug)]
pub struct GeoDistanceBucketResult {
    pub key: String,
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub doc_count: u64,
    pub aggs: Option<AggregationsResult>
}

impl GeoDistanceBucketResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(GeoDistanceBucketResult {
            key: from_json!(from, "key", as_str).to_owned(),
            from: optional_json!(from, "from", as_f64),
            to: optional_json!(from, "to", as_f64),
            doc_count: from_json!(from, "doc_count", as_u64),
            aggs: extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct GeoDistanceResult {
    pub buckets: Vec<GeoDistanceBucketResult>
}

impl GeoDistanceResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(GeoDistanceResult {
            buckets: from_bucket_vector!(from,
                                         bucket,
                                         GeoDistanceBucketResult::from(bucket, aggs))
        })
    }
}

#[derive(Debug)]
pub struct GeohashGridBucketResult {
    pub key:       String,
    pub doc_count: u64,
    pub aggs:      Option<AggregationsResult>
}

impl GeohashGridBucketResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(GeohashGridBucketResult {
            key: from_json!(from, "key", as_str).to_owned(),
            doc_count: from_json!(from, "doc_count", as_u64),
            aggs: extract_aggs!(from, aggs)
        })
    }

    add_aggs_ref!();
}

#[derive(Debug)]
pub struct GeohashGridResult {
    pub buckets: Vec<GeohashGridBucketResult>
}

impl GeohashGridResult {
    fn from(from: &Value, aggs: &Option<Aggregations>) -> Result<Self, EsError> {
        Ok(GeohashGridResult {
            buckets: from_bucket_vector!(from,
                                         bucket,
                                         GeohashGridBucketResult::from(bucket, aggs))
        })
    }
}

#[cfg(test)]
pub mod tests {
    use serde_json;

    use super::super::Aggregations;
    use super::Terms;

    #[test]
    fn test_terms_aggregation() {
        let aggs:Aggregations = ("term_test",
                                 Terms::field("blah").with_size(5u64)).into();

        assert_eq!("{\"term_test\":{\"terms\":{\"field\":\"blah\",\"size\":5}}}",
                   serde_json::to_string(&aggs).unwrap());
    }
}
