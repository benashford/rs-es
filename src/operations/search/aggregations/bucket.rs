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

use std::collections::BTreeMap;
use std::marker::PhantomData;

// TODO - deprecated
use rustc_serialize::json::{Json, ToJson};

use serde::ser::{Serialize, Serializer};
use serde_json::{to_value, Value};

use ::units::JsonVal;

use super::{Aggregation, Aggregations};

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

// TODO - deprecated
impl<'a> ToJson for Global<'a> {
    fn to_json(&self) -> Json {
        Json::Object(BTreeMap::new())
    }
}

bucket_agg!(Global);

// /// Filter aggregation
// #[derive(Debug)]
// pub struct Filter<'a> {
//     // TODO - Query is now an enum with a `Box` it might be simpler, with little
//     // side-effects to own it instead
//     filter: &'a query::Query
// }

// impl<'a> Filter<'a> {
//     pub fn new(filter: &'a query::Query) -> Filter<'a> {
//         Filter {
//             filter: filter
//         }
//     }
// }

// impl<'a> ToJson for Filter<'a> {
//     fn to_json(&self) -> Json {
//         self.filter.to_json()
//     }
// }

// bucket_agg!(Filter);

// /// Filters aggregation
// #[derive(Debug)]
// pub struct Filters<'a> {
//     filters: HashMap<&'a str, &'a query::Query>
// }

// impl<'a> Filters<'a> {
//     pub fn new(filters: HashMap<&'a str, &'a query::Query>) -> Filters<'a> {
//         Filters {
//             filters: filters
//         }
//     }
// }

// impl<'a> From<Vec<(&'a str, &'a query::Query)>> for Filters<'a> {
//     fn from(from: Vec<(&'a str, &'a query::Query)>) -> Filters<'a> {
//         let mut filters = HashMap::with_capacity(from.len());
//         for (k, v) in from {
//             filters.insert(k, v);
//         }
//         Filters::new(filters)
//     }
// }

// impl<'a> ToJson for Filters<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         let mut inner = BTreeMap::new();
//         for (&k, v) in self.filters.iter() {
//             inner.insert(k.to_owned(), v.to_json());
//         }
//         d.insert("filters".to_owned(), Json::Object(inner));
//         Json::Object(d)
//     }
// }

// bucket_agg!(Filters);

// /// Missing aggregation
// #[derive(Debug)]
// pub struct Missing<'a> {
//     pub field: &'a str
// }

// impl<'a> Missing<'a> {
//     pub fn new(field: &'a str) -> Missing<'a> {
//         Missing {
//             field: field
//         }
//     }
// }

// impl<'a> ToJson for Missing<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("field".to_owned(), self.field.to_json());
//         Json::Object(d)
//     }
// }

// bucket_agg!(Missing);

// /// Nested aggregation
// #[derive(Debug)]
// pub struct Nested<'a> {
//     pub path: &'a str
// }

// impl<'a> Nested<'a> {
//     pub fn new(path: &'a str) -> Nested<'a> {
//         Nested {
//             path: path
//         }
//     }
// }

// impl<'a> ToJson for Nested<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("path".to_owned(), self.path.to_json());
//         Json::Object(d)
//     }
// }

// bucket_agg!(Nested);

// /// Reverse nested aggregation, will produce an error if used anywhere other than
// /// inside a nested aggregation.
// ///
// /// See: https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations-bucket-reverse-nested-aggregation.html
// #[derive(Debug)]
// pub struct ReverseNested<'a> {
//     /// Needed for lifecycle reasons
//     phantom: PhantomData<&'a str>
// }

// impl<'a> ReverseNested<'a> {
//     pub fn new() -> ReverseNested<'a> {
//         ReverseNested {
//             phantom: PhantomData
//         }
//     }
// }

// impl<'a> ToJson for ReverseNested<'a> {
//     fn to_json(&self) -> Json {
//         Json::Object(BTreeMap::new())
//     }
// }

// bucket_agg!(ReverseNested);

// /// Children aggregation - sub-aggregations run against the child document
// #[derive(Debug)]
// pub struct Children<'a> {
//     doc_type: &'a str
// }

// impl<'a> Children<'a> {
//     pub fn new(doc_type: &'a str) -> Children<'a> {
//         Children {
//             doc_type: doc_type
//         }
//     }
// }

// impl<'a> ToJson for Children<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert("type".to_owned(), self.doc_type.to_json());
//         Json::Object(d)
//     }
// }

// bucket_agg!(Children);

/// Order - used for some bucketing aggregations to determine the order of
/// buckets
#[derive(Debug)]
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

impl<'a> ToString for OrderKey<'a> {
    fn to_string(&self) -> String {
        match *self {
            OrderKey::Count   => "_count".to_owned(),
            OrderKey::Key     => "_key".to_owned(),
            OrderKey::Term    => "_term".to_owned(),
            OrderKey::Expr(e) => e.to_owned()
        }
    }
}

// /// Used to define the ordering of buckets in a some bucketted aggregations
// ///
// /// # Examples
// ///
// /// ```
// /// use rs_es::operations::search::aggregations::bucket::{Order, OrderKey};
// ///
// /// let order1 = Order::asc(OrderKey::Count);
// /// let order2 = Order::desc("field_name");
// /// ```
// ///
// /// The first will produce a JSON fragment: `{"_count": "asc"}`; the second will
// /// produce a JSON fragment: `{"field_name", "desc"}`
// #[derive(Debug)]
// pub struct Order<'a>(OrderKey<'a>, super::Order);

// impl<'a> Order<'a> {
//     /// Create an `Order` ascending
//     pub fn asc<O: Into<OrderKey<'a>>>(key: O) -> Order<'a> {
//         Order(key.into(), super::Order::Asc)
//     }

//     /// Create an `Order` descending
//     pub fn desc<O: Into<OrderKey<'a>>>(key: O) -> Order<'a> {
//         Order(key.into(), super::Order::Desc)
//     }
// }

// impl<'a> ToJson for Order<'a> {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         d.insert(self.0.to_string(), self.1.to_json());
//         Json::Object(d)
//     }
// }

// /// Terms aggregation
// #[derive(Debug)]
// pub struct Terms<'a> {
//     field:      FieldOrScript<'a>,
//     size:       Option<u64>,
//     shard_size: Option<u64>,
//     order:      Option<Order<'a>>
// }

// impl<'a> Terms<'a> {
//     pub fn new<FOS: Into<FieldOrScript<'a>>>(field: FOS) -> Terms<'a> {
//         Terms {
//             field:      field.into(),
//             size:       None,
//             shard_size: None,
//             order:      None
//         }
//     }

//     add_field!(with_size, size, u64);
//     add_field!(with_shard_size, shard_size, u64);
//     add_field!(with_order, order, Order<'a>);
// }

// impl<'a> ToJson for Terms<'a> {
//     fn to_json(&self) -> Json {
//         let mut json = BTreeMap::new();
//         self.field.add_to_object(&mut json);

//         optional_add!(self, json, size);
//         optional_add!(self, json, shard_size);
//         optional_add!(self, json, order);

//         Json::Object(json)
//     }
// }

// bucket_agg!(Terms);

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

impl<'a> ToJson for RangeInst<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();

        optional_add!(self, d, from);
        optional_add!(self, d, to);
        optional_add!(self, d, key);

        Json::Object(d)
    }
}

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

impl<'a> ToJson for DateRangeInst<'a> {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        optional_add!(self, d, from);
        optional_add!(self, d, to);

        Json::Object(d)
    }
}

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

impl ToJson for ExtendedBounds {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("min".to_owned(), Json::I64(self.min));
        d.insert("max".to_owned(), Json::I64(self.max));

        Json::Object(d)
    }
}

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
pub enum BucketAggregation<'a> {
    Global(Global<'a>),
    // Filter(Filter<'a>),
    // Filters(Filters<'a>),
    // Missing(Missing<'a>),
    // Nested(Nested<'a>),
    // ReverseNested(ReverseNested<'a>),
    // Children(Children<'a>),
    // Terms(Terms<'a>),
    // Range(Range<'a>),
    // DateRange(DateRange<'a>),
    // Histogram(Histogram<'a>),
    // DateHistogram(DateHistogram<'a>),
    // GeoDistance(GeoDistance<'a>),
    // GeoHash(GeoHash<'a>)
}

impl<'a> BucketAggregation<'a> {
    fn add_to_object<S>(&self, json: &mut BTreeMap<&'static str, Value>)
        where S: Serialize {

        let (key, value) = match self {
            &BucketAggregation::Global(ref a) => ("global", to_value(a))
        };

        json.insert(key, value);
    }
}
