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

//! Various re-occuring types that are used by the ElasticSearch API.
//!
//! E.g. `Duration`
//!
//! This isn't all types. Types that are specific to one API are defined in the
//! appropriate place, e.g. types only used by the Query DSL are in `query.rs`

use std::collections::{BTreeMap, HashMap};

use rustc_serialize::json::{Json, ToJson};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de;
use serde_json::Value;

use ::error::EsError;
use ::operations::common::OptionVal;

/// The units by which duration is measured.
///
/// TODO - this list is incomplete, see: https://www.elastic.co/guide/en/elasticsearch/reference/current/common-options.html#time-units
/// TODO - ensure deserialization works correctly
#[derive(Debug, Serialize, Deserialize)]
pub enum DurationUnit {
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
    Millisecond
}

impl ToString for DurationUnit {
    fn to_string(&self) -> String {
        match *self {
            DurationUnit::Month  => "M",
            DurationUnit::Week   => "w",
            DurationUnit::Day    => "d",
            DurationUnit::Hour   => "h",
            DurationUnit::Minute => "m",
            DurationUnit::Second => "s",
            DurationUnit::Millisecond => "ms"
        }.to_owned()
    }
}

/// A time-period unit, will be formatted into the ElasticSearch standard format
///
/// # Examples
///
/// ```
/// use rs_es::units::{Duration, DurationUnit};
///
/// assert_eq!("100d", Duration::new(100, DurationUnit::Day).to_string());
/// ```
///
/// TODO - implement Deserialize correctly
#[derive(Debug, Deserialize, Serialize)]
pub struct Duration {
    amt: i64,
    unit: DurationUnit
}

impl Duration {
    pub fn new(amt: i64, unit: DurationUnit) -> Duration {
        Duration {
            amt: amt,
            unit: unit
        }
    }

    pub fn months(amt: i64) -> Duration {
        Duration::new(amt, DurationUnit::Month)
    }

    pub fn weeks(amt: i64) -> Duration {
        Duration::new(amt, DurationUnit::Week)
    }

    pub fn days(amt: i64) -> Duration {
        Duration::new(amt, DurationUnit::Day)
    }

    pub fn hours(amt: i64) -> Duration {
        Duration::new(amt, DurationUnit::Hour)
    }

    pub fn minutes(amt: i64) -> Duration {
        Duration::new(amt, DurationUnit::Minute)
    }

    pub fn seconds(amt: i64) -> Duration {
        Duration::new(amt, DurationUnit::Second)
    }

    pub fn milliseconds(amt: i64) -> Duration {
        Duration::new(amt, DurationUnit::Millisecond)
    }
}

impl ToString for Duration {
    fn to_string(&self) -> String {
        format!("{}{}", self.amt, self.unit.to_string())
    }
}

impl ToJson for Duration {
    fn to_json(&self) -> Json {
        Json::String(self.to_string())
    }
}

impl<'a> From<&'a Duration> for OptionVal {
    fn from(from: &'a Duration) -> OptionVal {
        OptionVal(from.to_string())
    }
}

from_exp!(Duration, OptionVal, from, OptionVal(from.to_string()));

/// Representing a geographic location
#[derive(Debug)]
pub enum Location {
    LatLon(f64, f64),
    GeoHash(String)
}

impl Default for Location {
    fn default() -> Location {
        Location::LatLon(0f64, 0f64)
    }
}

impl Deserialize for Location {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer {

        // TODO - maybe use a specific struct?
        let mut raw_location = try!(HashMap::<String, f64>::deserialize(deserializer));
        Ok(Location::LatLon(
            raw_location.remove("lat").unwrap(),
            raw_location.remove("lon").unwrap()
        ))
    }
}

// TODO - deprecated
// impl<'a> From<&'a Json> for Location {
//     fn from(from: &'a Json) -> Location {
//         Location::LatLon(
//             get_json_f64!(from, "lat"),
//             get_json_f64!(from, "lon"))
//     }
// }

from_exp!((f64, f64), Location, from, Location::LatLon(from.0, from.1));
from!(String, Location, GeoHash);

impl Serialize for Location {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        match self {
            &Location::LatLon(lat, lon) => {
                let mut d = BTreeMap::new();
                d.insert("lat", lat);
                d.insert("lon", lon);
                d.serialize(serializer)
            },
            &Location::GeoHash(ref geo_hash) => {
                geo_hash.serialize(serializer)
            }
        }
    }
}

// TODO - deprecated
// impl ToJson for Location {
//     fn to_json(&self) -> Json {
//         match self {
//             &Location::LatLon(lat, lon) => {
//                 let mut d = BTreeMap::new();
//                 d.insert("lat".to_owned(), Json::F64(lat));
//                 d.insert("lon".to_owned(), Json::F64(lon));
//                 Json::Object(d)
//             },
//             &Location::GeoHash(ref geo_hash) => {
//                 Json::String(geo_hash.clone())
//             }
//         }
//     }
// }

/// Representing a geographic box
#[derive(Debug)]
pub enum GeoBox {
    Corners(Location, Location),
    Vertices(f64, f64, f64, f64)
}

impl Default for GeoBox {
    fn default() -> Self {
        GeoBox::Vertices(0f64, 0f64, 0f64, 0f64)
    }
}

impl Deserialize for GeoBox {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer {

        // TODO - maybe use a specific struct?
        let mut raw_geo_box = try!(HashMap::<String, Location>::deserialize(deserializer));
        Ok(GeoBox::Corners(
            raw_geo_box.remove("top_left").unwrap(),
            raw_geo_box.remove("bottom_right").unwrap()
        ))
    }
}

// TODO - deprecated
// impl<'a> From<&'a Json> for GeoBox {
//     fn from(from: &'a Json) -> GeoBox {
//         GeoBox::Corners(
//             Location::from(from.find("top_left").expect("No 'top_left' field")),
//             Location::from(from.find("bottom_right").expect("No 'bottom_right' field")))
//     }
// }

from_exp!((Location, Location), GeoBox, from, GeoBox::Corners(from.0, from.1));
from_exp!(((f64, f64), (f64, f64)),
          GeoBox,
          from,
          GeoBox::Corners(Location::LatLon({from.0}.0, {from.0}.1),
                          Location::LatLon({from.1}.0, {from.1}.1)));
from_exp!((f64, f64, f64, f64),
          GeoBox,
          from,
          GeoBox::Vertices(from.0, from.1, from.2, from.3));

// TODO - deprecated
// impl ToJson for GeoBox {
//     fn to_json(&self) -> Json {
//         let mut d = BTreeMap::new();
//         match self {
//             &GeoBox::Corners(ref top_left, ref bottom_right) => {
//                 d.insert("top_left".to_owned(), top_left.to_json());
//                 d.insert("bottom_right".to_owned(), bottom_right.to_json());
//             },
//             &GeoBox::Vertices(ref top, ref left, ref bottom, ref right) => {
//                 d.insert("top".to_owned(), top.to_json());
//                 d.insert("left".to_owned(), left.to_json());
//                 d.insert("bottom".to_owned(), bottom.to_json());
//                 d.insert("right".to_owned(), right.to_json());
//             }
//         }
//         Json::Object(d)
//     }
// }

/// A non-specific holder for an option which can either be a single thing, or
/// multiple instances of that thing.
#[derive(Debug)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>)
}

impl<T: Default> Default for OneOrMany<T> {
    fn default() -> Self {
        OneOrMany::One(Default::default())
    }
}

impl<T> Serialize for OneOrMany<T>
    where T: Serialize {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        match self {
            &OneOrMany::One(ref t) => t.serialize(serializer),
            &OneOrMany::Many(ref t) => t.serialize(serializer)
        }
    }
}

impl<T> From<T> for OneOrMany<T> {
    fn from(from: T) -> OneOrMany<T> {
        OneOrMany::One(from)
    }
}

impl<T> From<Vec<T>> for OneOrMany<T> {
    fn from(from: Vec<T>) -> OneOrMany<T> {
        OneOrMany::Many(from)
    }
}

impl<T: ToJson> ToJson for OneOrMany<T> {
    fn to_json(&self) -> Json {
        match self {
            &OneOrMany::One(ref t)  => t.to_json(),
            &OneOrMany::Many(ref t) => t.to_json()
        }
    }
}

/// DistanceType
#[derive(Debug)]
pub enum DistanceType {
    SloppyArc,
    Arc,
    Plane
}

impl Serialize for DistanceType {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        match self {
            &DistanceType::SloppyArc => "sloppy_arc",
            &DistanceType::Arc       => "arc",
            &DistanceType::Plane     => "plane"
        }.serialize(serializer)
    }
}

impl ToJson for DistanceType {
    fn to_json(&self) -> Json {
        Json::String(match self {
            &DistanceType::SloppyArc => "sloppy_arc",
            &DistanceType::Arc       => "arc",
            &DistanceType::Plane     => "plane"
        }.to_owned())
    }
}

/// DistanceUnit
#[derive(Debug)]
pub enum DistanceUnit {
    Mile,
    Yard,
    Feet,
    Inch,
    Kilometer,
    Meter,
    Centimeter,
    Millimeter,
    NauticalMile
}

impl Default for DistanceUnit {
    fn default() -> DistanceUnit {
        DistanceUnit::Kilometer
    }
}

impl ToString for DistanceUnit {
    fn to_string(&self) -> String {
        match *self {
            DistanceUnit::Mile => "mi",
            DistanceUnit::Yard => "yd",
            DistanceUnit::Feet => "ft",
            DistanceUnit::Inch => "in",
            DistanceUnit::Kilometer => "km",
            DistanceUnit::Meter => "m",
            DistanceUnit::Centimeter => "cm",
            DistanceUnit::Millimeter => "mm",
            DistanceUnit::NauticalMile => "NM"
        }.to_owned()
    }
}

impl Serialize for DistanceUnit {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        self.to_string().serialize(serializer)
    }
}

impl ToJson for DistanceUnit {
    fn to_json(&self) -> Json {
        Json::String(self.to_string())
    }
}

/// Distance, both an amount and a unit
#[derive(Debug, Default)]
pub struct Distance {
     amt: f64,
     unit: DistanceUnit
}

impl Distance {
    pub fn new(amt: f64, unit: DistanceUnit) -> Distance {
        Distance {
            amt:  amt,
            unit: unit
        }
    }
}

impl Serialize for Distance {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        format!("{}{}", self.amt, self.unit.to_string()).serialize(serializer)
    }
}

/// A trait for types that can become JsonVals
pub trait JsonPotential {
    fn to_json_val(&self) -> JsonVal;
}

macro_rules! json_potential {
    ($t:ty) => (
        impl JsonPotential for $t {
            fn to_json_val(&self) -> JsonVal {
                (*self).into()
            }
        }
    )
}

impl<'a> JsonPotential for &'a str {
    fn to_json_val(&self) -> JsonVal {
        (*self).into()
    }
}

json_potential!(i64);
json_potential!(i32);
json_potential!(u64);
json_potential!(u32);
json_potential!(f64);
json_potential!(f32);
json_potential!(bool);

/// A Json value that's not a structural thing - i.e. just String, i64 and f64,
/// no array or object
#[derive(Debug)]
pub enum JsonVal {
    String(String),
    I64(i64),
    U64(u64),
    F64(f64),
    Boolean(bool)
}

impl JsonVal {
    pub fn from(from: &Value) -> Result<Self, EsError> {
        use serde_json::Value::*;
        Ok(match from {
            &String(ref string) => JsonVal::String(string.clone()),
            &Bool(b) => JsonVal::Boolean(b),
            &I64(i) => JsonVal::I64(i),
            &U64(u) => JsonVal::U64(u),
            &F64(f) => JsonVal::F64(f),
            _ => return Err(EsError::EsError(format!("Not a JsonVal: {:?}",
                                                     from)))
        })
    }
}

impl Default for JsonVal {
    fn default() -> Self {
        JsonVal::String(Default::default())
    }
}

impl Serialize for JsonVal {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        match self {
            &JsonVal::String(ref s) => s.serialize(serializer),
            &JsonVal::I64(i) => i.serialize(serializer),
            &JsonVal::U64(u) => u.serialize(serializer),
            &JsonVal::F64(f) => f.serialize(serializer),
            &JsonVal::Boolean(b) => b.serialize(serializer)
        }
    }
}

impl Deserialize for JsonVal {
    fn deserialize<D>(deserializer: &mut D) -> Result<JsonVal, D::Error>
        where D: Deserializer {

        deserializer.deserialize(JsonValVisitor)
    }
}

struct JsonValVisitor;

impl de::Visitor for JsonValVisitor {
    type Value = JsonVal;

    fn visit_string<E>(&mut self, s: String) -> Result<JsonVal, E>
        where E: de::Error {
        Ok(JsonVal::String(s))
    }

    fn visit_i64<E>(&mut self, i: i64) -> Result<JsonVal, E>
        where E: de::Error {
        Ok(JsonVal::I64(i))
    }

    fn visit_u64<E>(&mut self, u: u64) -> Result<JsonVal, E>
        where E: de::Error {
        Ok(JsonVal::U64(u))
    }

    fn visit_f64<E>(&mut self, f: f64) -> Result<JsonVal, E>
        where E: de::Error {
        Ok(JsonVal::F64(f))
    }

    fn visit_bool<E>(&mut self, b: bool) -> Result<JsonVal, E>
        where E: de::Error {
        Ok(JsonVal::Boolean(b))
    }
}

// TODO - deprecated
// impl ToJson for JsonVal {
//     fn to_json(&self) -> Json {
//         match self {
//             &JsonVal::String(ref str) => str.to_json(),
//             &JsonVal::I64(i)          => Json::I64(i),
//             &JsonVal::U64(u)          => Json::U64(u),
//             &JsonVal::F64(f)          => Json::F64(f),
//             &JsonVal::Boolean(b)      => Json::Boolean(b)
//         }
//     }
// }

from!(String, JsonVal, String);

impl<'a> From<&'a str> for JsonVal {
    fn from(from: &'a str) -> JsonVal {
        JsonVal::String(from.to_owned())
    }
}

from_exp!(f32, JsonVal, from, JsonVal::F64(from as f64));
from!(f64, JsonVal, F64);
from_exp!(i32, JsonVal, from, JsonVal::I64(from as i64));
from!(i64, JsonVal, I64);
from_exp!(u32, JsonVal, from, JsonVal::U64(from as u64));
from!(u64, JsonVal, U64);
from!(bool, JsonVal, Boolean);

impl<'a> From<&'a Value> for JsonVal {
    fn from(from: &'a Value) -> Self {
        use serde_json::Value::*;
        match from {
            &String(ref s) => JsonVal::String(s.clone()),
            &F64(f) => JsonVal::F64(f),
            &I64(f) => JsonVal::I64(f),
            &U64(f) => JsonVal::U64(f),
            &Bool(b) => JsonVal::Boolean(b),
            _ => panic!("Not a String, F64, I64, U64 or Boolean")
        }
    }
}
