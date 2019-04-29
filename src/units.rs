/*
 * Copyright 2015-2018 Ben Ashford
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
use std::fmt;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Number, Value};

use crate::{error::EsError, operations::common::OptionVal};

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
    Millisecond,
}

impl ToString for DurationUnit {
    fn to_string(&self) -> String {
        match *self {
            DurationUnit::Month => "M",
            DurationUnit::Week => "w",
            DurationUnit::Day => "d",
            DurationUnit::Hour => "h",
            DurationUnit::Minute => "m",
            DurationUnit::Second => "s",
            DurationUnit::Millisecond => "ms",
        }
        .to_owned()
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
    unit: DurationUnit,
}

impl Duration {
    pub fn new(amt: i64, unit: DurationUnit) -> Duration {
        Duration {
            amt: amt,
            unit: unit,
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
    GeoHash(String),
}

impl Default for Location {
    fn default() -> Location {
        Location::LatLon(0f64, 0f64)
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // TODO - maybe use a specific struct?
        let mut raw_location = HashMap::<String, f64>::deserialize(deserializer)?;
        Ok(Location::LatLon(
            raw_location.remove("lat").unwrap(),
            raw_location.remove("lon").unwrap(),
        ))
    }
}

from_exp!((f64, f64), Location, from, Location::LatLon(from.0, from.1));
from!(String, Location, GeoHash);

impl Serialize for Location {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Location::LatLon(lat, lon) => {
                let mut d = BTreeMap::new();
                d.insert("lat", lat);
                d.insert("lon", lon);
                d.serialize(serializer)
            }
            Location::GeoHash(ref geo_hash) => geo_hash.serialize(serializer),
        }
    }
}

/// Representing a geographic box
// TODO - this could probably refactored in a way that makes serialization easier
#[derive(Debug)]
pub enum GeoBox {
    Corners(Location, Location),
    Vertices(f64, f64, f64, f64),
}

impl Default for GeoBox {
    fn default() -> Self {
        GeoBox::Vertices(0f64, 0f64, 0f64, 0f64)
    }
}

impl<'de> Deserialize<'de> for GeoBox {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // TODO - maybe use a specific struct?
        let mut raw_geo_box = HashMap::<String, Location>::deserialize(deserializer)?;
        Ok(GeoBox::Corners(
            raw_geo_box.remove("top_left").unwrap(),
            raw_geo_box.remove("bottom_right").unwrap(),
        ))
    }
}

from_exp!(
    (Location, Location),
    GeoBox,
    from,
    GeoBox::Corners(from.0, from.1)
);
from_exp!(
    ((f64, f64), (f64, f64)),
    GeoBox,
    from,
    GeoBox::Corners(
        Location::LatLon({ from.0 }.0, { from.0 }.1),
        Location::LatLon({ from.1 }.0, { from.1 }.1)
    )
);
from_exp!(
    (f64, f64, f64, f64),
    GeoBox,
    from,
    GeoBox::Vertices(from.0, from.1, from.2, from.3)
);

impl Serialize for GeoBox {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use self::GeoBox::*;
        match self {
            Corners(ref top_left, ref bottom_right) => {
                let mut d = BTreeMap::new();
                d.insert("top_left", top_left);
                d.insert("bottom_right", bottom_right);
                d.serialize(serializer)
            }
            Vertices(top, left, bottom, right) => {
                let mut d = BTreeMap::new();
                d.insert("top", top);
                d.insert("left", left);
                d.insert("bottom", bottom);
                d.insert("right", right);
                d.serialize(serializer)
            }
        }
    }
}

/// A non-specific holder for an option which can either be a single thing, or
/// multiple instances of that thing.
#[derive(Debug)]
pub enum OneOrMany<T> {
    One(T),
    Many(Vec<T>),
}

impl<T: Default> Default for OneOrMany<T> {
    fn default() -> Self {
        OneOrMany::One(Default::default())
    }
}

impl<T> Serialize for OneOrMany<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            OneOrMany::One(ref t) => t.serialize(serializer),
            OneOrMany::Many(ref t) => t.serialize(serializer),
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

/// DistanceType
#[derive(Debug)]
pub enum DistanceType {
    SloppyArc,
    Arc,
    Plane,
}

impl Serialize for DistanceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            DistanceType::SloppyArc => "sloppy_arc",
            DistanceType::Arc => "arc",
            DistanceType::Plane => "plane",
        }
        .serialize(serializer)
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
    NauticalMile,
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
            DistanceUnit::NauticalMile => "NM",
        }
        .to_owned()
    }
}

impl Serialize for DistanceUnit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

/// Distance, both an amount and a unit
#[derive(Debug, Default)]
pub struct Distance {
    amt: f64,
    unit: DistanceUnit,
}

impl Distance {
    pub fn new(amt: f64, unit: DistanceUnit) -> Distance {
        Distance {
            amt: amt,
            unit: unit,
        }
    }
}

impl Serialize for Distance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{}{}", self.amt, self.unit.to_string()).serialize(serializer)
    }
}

/// A trait for types that can become JsonVals
pub trait JsonPotential {
    fn to_json_val(&self) -> JsonVal;
}

macro_rules! json_potential {
    ($t:ty) => {
        impl JsonPotential for $t {
            fn to_json_val(&self) -> JsonVal {
                (*self).into()
            }
        }
    };
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
    Number(Number),
    Boolean(bool),
}

impl JsonVal {
    pub fn from(from: &Value) -> Result<Self, EsError> {
        use serde_json::Value::*;
        Ok(match from {
            String(ref string) => JsonVal::String(string.clone()),
            Bool(b) => JsonVal::Boolean(*b),
            Number(ref i) => JsonVal::Number(i.clone()),
            _ => return Err(EsError::EsError(format!("Not a JsonVal: {:?}", from))),
        })
    }
}

impl Default for JsonVal {
    fn default() -> Self {
        JsonVal::String(Default::default())
    }
}

impl Serialize for JsonVal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            JsonVal::String(ref s) => s.serialize(serializer),
            JsonVal::Number(ref i) => i.serialize(serializer),
            JsonVal::Boolean(b) => b.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for JsonVal {
    fn deserialize<D>(deserializer: D) -> Result<JsonVal, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(JsonValVisitor)
    }
}

struct JsonValVisitor;

impl<'de> de::Visitor<'de> for JsonValVisitor {
    type Value = JsonVal;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a json value")
    }

    fn visit_string<E>(self, s: String) -> Result<JsonVal, E>
    where
        E: de::Error,
    {
        Ok(JsonVal::String(s))
    }

    fn visit_str<E>(self, s: &str) -> Result<JsonVal, E>
    where
        E: de::Error,
    {
        Ok(JsonVal::String(s.to_owned()))
    }

    fn visit_i64<E>(self, i: i64) -> Result<JsonVal, E>
    where
        E: de::Error,
    {
        Ok(JsonVal::Number(i.into()))
    }

    fn visit_u64<E>(self, u: u64) -> Result<JsonVal, E>
    where
        E: de::Error,
    {
        Ok(JsonVal::Number(u.into()))
    }

    fn visit_f64<E>(self, f: f64) -> Result<JsonVal, E>
    where
        E: de::Error,
    {
        Ok(JsonVal::Number(
            Number::from_f64(f).ok_or_else(|| de::Error::custom("not a float"))?,
        ))
    }

    fn visit_bool<E>(self, b: bool) -> Result<JsonVal, E>
    where
        E: de::Error,
    {
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

from_exp!(
    f32,
    JsonVal,
    from,
    #[allow(clippy::cast_lossless)]
    JsonVal::Number(Number::from_f64(from as f64).unwrap())
);
from_exp!(
    f64,
    JsonVal,
    from,
    JsonVal::Number(Number::from_f64(from).unwrap())
);
from_exp!(i32, JsonVal, from, JsonVal::Number(from.into()));
from_exp!(i64, JsonVal, from, JsonVal::Number(from.into()));
from_exp!(u32, JsonVal, from, JsonVal::Number(from.into()));
from_exp!(u64, JsonVal, from, JsonVal::Number(from.into()));
from!(bool, JsonVal, Boolean);

impl<'a> From<&'a Value> for JsonVal {
    fn from(from: &'a Value) -> Self {
        use serde_json::Value::*;
        match from {
            String(ref s) => JsonVal::String(s.clone()),
            Number(ref f) => JsonVal::Number(f.clone()),
            Bool(b) => JsonVal::Boolean(*b),
            _ => panic!("Not a String, F64, I64, U64 or Boolean"),
        }
    }
}
