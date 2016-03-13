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

use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

use serde::ser::{Serialize, Serializer};
use serde_json::{to_value, Value};

use ::error::EsError;
use ::json::ShouldSkip;
use ::query;
use ::units::{JsonVal, OneOrMany};

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
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
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
pub struct Filter<'a>(&'a query::Query);

impl<'a> Filter<'a> {
    pub fn new(filter: &'a query::Query) -> Filter<'a> {
        Filter(filter)
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
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
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

#[derive(Debug, Default, Serialize)]
pub struct TermsInner<'a> {
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    size: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    shard_size: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    order: Option<OneOrMany<Order<'a>>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    min_doc_count: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    shard_min_doc_count: Option<u64>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    include: Option<OneOrMany<&'a str>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
    exclude: Option<OneOrMany<&'a str>>,
    #[serde(skip_serializing_if="ShouldSkip::should_skip")]
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

fos_bucket_agg!(Terms);

// Range aggs and dependencies

/// A specific range, there will be many of these making up a range aggregation
#[derive(Debug)]
pub struct RangeInst<'a> {
    from: Option<JsonVal>,
    to:   Option<JsonVal>,
    key:  Option<&'a str>
}

impl<'a> RangeInst<'a> {
    pub fn new() -> RangeInst<'a> {
        RangeInst {
            from: None,
            to:   None,
            key:  None
        }
    }

    add_field!(with_from, from, JsonVal);
    add_field!(with_to, to, JsonVal);
    add_field!(with_key, key, &'a str);
}

// TODO - deprecated
// impl<'a> ToJson for RangeInst<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();

//         optional_add!(self, d, from);
//         optional_add!(self, d, to);
//         optional_add!(self, d, key);

//         Json::Object(d)
//     }
// }

// /// Range aggregations
// ///
// /// The keyed option will always be used.
// ///
// /// https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-range-aggregation.html
// #[derive(Debug)]
// pub struct Range<'a> {
//     field: FieldOrScript<'a>,
//     keyed: bool,
//     ranges: Vec<RangeInst<'a>>
// }

// impl<'a> Range<'a> {
//     pub fn new<FOS: Into<FieldOrScript<'a>>>(field: FOS,
//                                              ranges: Vec<RangeInst<'a>>) -> Range<'a> {
//         Range {
//             field:  field.into(),
//             keyed:  true,
//             ranges: ranges
//         }
//     }

//     pub fn inst() -> RangeInst<'a> {
//         RangeInst::new()
//     }
// }

// impl<'a> ToJson for Range<'a> {
//     fn to_json(&self) -> Json {
//         let mut json = BTreeMap::new();
//         self.field.add_to_object(&mut json);
//         json.insert("keyed".to_owned(), Json::Boolean(self.keyed));
//         json.insert("ranges".to_owned(), self.ranges.to_json());
//         Json::Object(json)
//     }
// }

// bucket_agg!(Range);

/// A specific element of a range for a `DateRange` aggregation
#[derive(Debug)]
pub struct DateRangeInst<'a> {
    from: Option<&'a str>,
    to:   Option<&'a str>
}

impl<'a> DateRangeInst<'a> {
    pub fn new() -> DateRangeInst<'a> {
        DateRangeInst {
            from: None,
            to:   None
        }
    }

    add_field!(with_from, from, &'a str);
    add_field!(with_to, to, &'a str);
}

// TODO - deprecated
// impl<'a> ToJson for DateRangeInst<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         optional_add!(self, d, from);
//         optional_add!(self, d, to);

//         Json::Object(d)
//     }
// }

// /// Date range aggregation.  See: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-daterange-aggregation.html
// #[derive(Debug)]
// pub struct DateRange<'a> {
//     field: FieldOrScript<'a>,
//     format: Option<&'a str>,
//     ranges: Vec<DateRangeInst<'a>>
// }

// impl<'a> DateRange<'a> {
//     pub fn new<FOS: Into<FieldOrScript<'a>>>(field: FOS,
//                                              ranges: Vec<DateRangeInst<'a>>) -> DateRange<'a> {
//         DateRange {
//             field: field.into(),
//             format: None,
//             ranges: ranges
//         }
//     }

//     pub fn inst() -> DateRangeInst<'a> {
//         DateRangeInst::new()
//     }
// }

// impl<'a> ToJson for DateRange<'a> {
//     fn to_json(&self) -> Json {
//         let mut json = BTreeMap::new();
//         self.field.add_to_object(&mut json);
//         optional_add!(self, json, format);
//         json.insert("ranges".to_owned(), self.ranges.to_json());
//         Json::Object(json)
//     }
// }

// bucket_agg!(DateRange);

/// Histogram aggregation.
#[derive(Debug)]
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

// TODO - deprecated
// impl ToJson for ExtendedBounds {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("min".to_owned(), Json::I64(self.min));
//         d.insert("max".to_owned(), Json::I64(self.max));

//         Json::Object(d)
//     }
// }

impl From<(i64, i64)> for ExtendedBounds {
    fn from(from: (i64, i64)) -> ExtendedBounds {
        ExtendedBounds::new(from.0, from.1)
    }
}

// #[derive(Debug)]
// pub struct Histogram<'a> {
//     field:           &'a str,
//     interval:        Option<u64>,
//     min_doc_count:   Option<u64>,
//     extended_bounds: Option<ExtendedBounds>,
//     order:           Option<Order<'a>>
// }

// impl<'a> Histogram<'a> {
//     pub fn new(field: &'a str) -> Histogram<'a> {
//         Histogram {
//             field: field,
//             interval: None,
//             min_doc_count: None,
//             extended_bounds: None,
//             order: None
//         }
//     }

//     add_field!(with_interval, interval, u64);
//     add_field!(with_min_doc_count, min_doc_count, u64);
//     add_field!(with_extended_bounds, extended_bounds, ExtendedBounds);
//     add_field!(with_order, order, Order<'a>);
// }

// impl<'a> ToJson for Histogram<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("field".to_owned(), self.field.to_json());
//         optional_add!(self, d, interval);
//         optional_add!(self, d, min_doc_count);
//         optional_add!(self, d, extended_bounds);
//         optional_add!(self, d, order);

//         Json::Object(d)
//     }
// }

// bucket_agg!(Histogram);

/// Date histogram and related fields
#[derive(Debug)]
pub enum TimeZone<'a> {
    Offset(u64),
    Str(&'a str)
}

// TODO - deprecated
// impl<'a> ToJson for TimeZone<'a> {
//     fn to_json(&self) -> Json {
//         match self {
//             &TimeZone::Offset(offset) => Json::U64(offset),
//             &TimeZone::Str(tz_str)    => tz_str.to_json()
//         }
//     }
// }

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

// TODO - deprecated
// impl ToJson for Interval {
//     fn to_json(&self) -> Json {
//         use self::Interval::*;
//         match *self {
//             Year => "year",
//             Quarter => "quarter",
//             Month => "month",
//             Week => "week",
//             Day => "day",
//             Hour => "hour",
//             Minute => "minute",
//             Second => "second"
//         }.to_json()
//     }
// }

// #[derive(Debug)]
// pub struct DateHistogram<'a> {
//     field: &'a str,
//     interval: Interval,
//     time_zone: Option<TimeZone<'a>>,
//     offset: Option<Duration>,
//     format: Option<&'a str>,
// }

// impl<'a> DateHistogram<'a> {
//     pub fn new<I>(field: &'a str, interval: I) -> DateHistogram<'a>
//     where I: Into<Interval> {
//         DateHistogram {
//             field: field,
//             interval: interval.into(),
//             time_zone: None,
//             offset: None,
//             format: None
//         }
//     }

//     add_field!(with_time_zone, time_zone, TimeZone<'a>);
//     add_field!(with_offset, offset, Duration);
//     add_field!(with_format, format, &'a str);
// }

// impl<'a> ToJson for DateHistogram<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("field".to_owned(), self.field.to_json());
//         d.insert("interval".to_owned(), self.interval.to_json());
//         optional_add!(self, d, time_zone);
//         optional_add!(self, d, offset);
//         optional_add!(self, d, format);

//         Json::Object(d)
//     }
// }

// bucket_agg!(DateHistogram);

// #[derive(Debug)]
// pub struct GeoDistanceInst {
//     from: Option<f64>,
//     to:   Option<f64>
// }

// impl GeoDistanceInst {
//     pub fn new() -> GeoDistanceInst {
//         GeoDistanceInst {
//             from: None,
//             to:   None
//         }
//     }

//     add_field!(with_from, from, f64);
//     add_field!(with_to, to, f64);
// }

// impl ToJson for GeoDistanceInst {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         optional_add!(self, d, from);
//         optional_add!(self, d, to);

//         Json::Object(d)
//     }
// }

// #[derive(Debug)]
// pub struct GeoDistance<'a> {
//     field:         &'a str,
//     origin:        &'a Location,
//     unit:          Option<DistanceUnit>,
//     distance_type: Option<DistanceType>,
//     ranges:        &'a [GeoDistanceInst]
// }

// impl<'a> GeoDistance<'a> {
//     pub fn new(field: &'a str,
//                origin: &'a Location,
//                ranges: &'a [GeoDistanceInst]) -> GeoDistance<'a> {
//         GeoDistance {
//             field:         field,
//             origin:        origin,
//             unit:          None,
//             distance_type: None,
//             ranges:        ranges
//         }
//     }

//     add_field!(with_unit, unit, DistanceUnit);
//     add_field!(with_distance_type, distance_type, DistanceType);

//     pub fn inst() -> GeoDistanceInst {
//         GeoDistanceInst::new()
//     }
// }

// impl<'a> ToJson for GeoDistance<'a> {
//     fn to_json(&self) -> Json {
//         let mut json = BTreeMap::new();

//         json.insert("field".to_owned(), self.field.to_json());
//         json.insert("origin".to_owned(), self.origin.to_json());
//         json.insert("ranges".to_owned(), self.ranges.to_json());

//         optional_add!(self, json, unit);
//         optional_add!(self, json, distance_type);

//         Json::Object(json)
//     }
// }

// bucket_agg!(GeoDistance);

// /// Geohash aggregation
// #[derive(Debug)]
// pub struct GeoHash<'a> {
//     field:      &'a str,
//     precision:  Option<u64>,
//     size:       Option<u64>,
//     shard_size: Option<u64>
// }

// impl<'a> GeoHash<'a> {
//     pub fn new(field: &'a str) -> GeoHash<'a> {
//         GeoHash {
//             field: field,
//             precision: None,
//             size: None,
//             shard_size: None
//         }
//     }

//     add_field!(with_precision, precision, u64);
//     add_field!(with_size, size, u64);
//     add_field!(with_shard_size, shard_size, u64);
// }

// impl<'a> ToJson for GeoHash<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();

//         d.insert("field".to_owned(), self.field.to_json());

//         optional_add!(self, d, precision);
//         optional_add!(self, d, size);
//         optional_add!(self, d, shard_size);

//         Json::Object(d)
//     }
// }

// bucket_agg!(GeoHash);

/// The set of bucket aggregations
#[derive(Debug)]
// TODO - make sure all these are uncommented
pub enum BucketAggregation<'a> {
    Global(Global<'a>),
    Filter(Filter<'a>),
    Filters(Filters<'a>),
    Missing(Missing<'a>),
    Nested(Nested<'a>),
    ReverseNested(ReverseNested<'a>),
    Children(Children<'a>),
    Terms(Terms<'a>),
    // Range(Range<'a>),
    // DateRange(DateRange<'a>),
    // Histogram(Histogram<'a>),
    // DateHistogram(DateHistogram<'a>),
    // GeoDistance(GeoDistance<'a>),
    // GeoHash(GeoHash<'a>)
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
        }
    }
}

impl<'a> Serialize for BucketAggregation<'a> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
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
            &Terms(ref t) => t.serialize(serializer)
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
    Terms(TermsResult)
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
            // &BucketAggregation::Range(_) => {
            //     AggregationResult::Range(RangeResult::from(json, aggs))
            // },
            // &BucketAggregation::DateRange(_) => {
            //     AggregationResult::DateRange(DateRangeResult::from(json, aggs))
            // },
            // &BucketAggregation::Histogram(_) => {
            //     AggregationResult::Histogram(HistogramResult::from(json, aggs))
            // },
            // &BucketAggregation::DateHistogram(_) => {
            //     AggregationResult::DateHistogram(DateHistogramResult::from(json,
            //                                                                aggs))
            // },
            // &BucketAggregation::GeoDistance(_) => {
            //     AggregationResult::GeoDistance(GeoDistanceResult::from(json,
            //                                                            aggs))
            // },
            // &BucketAggregation::GeoHash(_) => {
            //     AggregationResult::GeoHash(GeoHashResult::from(json, aggs))
            // }
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
    // agg_as!(as_filters, Filters, FiltersResult);
    // agg_as!(as_missing, Missing, MissingResult);
    // agg_as!(as_nested, Nested, NestedResult);
    // agg_as!(as_reverse_nested, ReverseNested, ReverseNestedResult);
    // agg_as!(as_children, Children, ChildrenResult);
    bucket_agg_as!(as_terms, Terms, TermsResult);
    // agg_as!(as_range, Range, RangeResult);
    // agg_as!(as_date_range, DateRange, DateRangeResult);
    // agg_as!(as_histogram, Histogram, HistogramResult);
    // agg_as!(as_date_histogram, DateHistogram, DateHistogramResult);
    // agg_as!(as_geo_distance, GeoDistance, GeoDistanceResult);
    // agg_as!(as_geo_hash, GeoHash, GeoHashResult);
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

macro_rules! from_json {
    ($j:ident, $f:expr, $a:ident) => {
        match $j.find($f) {
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
            buckets: {
                let mut r = Vec::new();
                for bucket in from_json!(json, "buckets", as_array).iter() {
                    r.push(try!(TermsBucketResult::from(bucket, aggs)));
                }
                r
            }
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
            key: try!(JsonVal::from(match json.find("key") {
                Some(key) => key,
                None => return_error!("No 'key'".to_owned())
            })),
            doc_count: from_json!(json, "doc_count", as_u64),
            aggs: extract_aggs!(json, aggs)
        })
    }

    add_aggs_ref!();
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
