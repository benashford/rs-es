/*
 * Copyright 2015 Ben Ashford
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

use std::collections::BTreeMap;

use rustc_serialize::json::{Json, ToJson};

use ::operations::common::OptionVal;

/// The units by which duration is measured.
///
/// TODO - this list is incomplete
#[derive(Debug)]
pub enum DurationUnit {
    Week,
    Day,
    Hour,
    Minute
}

impl ToString for DurationUnit {
    fn to_string(&self) -> String {
        match *self {
            DurationUnit::Week   => "w",
            DurationUnit::Day    => "d",
            DurationUnit::Hour   => "h",
            DurationUnit::Minute => "m"
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
#[derive(Debug)]
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
pub enum Location {
    LatLon(f64, f64),
    GeoHash(String)
}

from_exp!((f64, f64), Location, from, Location::LatLon(from.0, from.1));
from!(String, Location, GeoHash);

impl ToJson for Location {
    fn to_json(&self) -> Json {
        match self {
            &Location::LatLon(lat, lon) => {
                let mut d = BTreeMap::new();
                d.insert("lat".to_owned(), Json::F64(lat));
                d.insert("lon".to_owned(), Json::F64(lon));
                Json::Object(d)
            },
            &Location::GeoHash(ref geo_hash) => {
                Json::String(geo_hash.clone())
            }
        }
    }
}

/// A non-specific holder for an option which can either be a single thing, or
/// multiple instances of that thing.
pub enum OneOrMany<T: ToJson> {
    One(T),
    Many(Vec<T>)
}

impl<T: ToJson> From<T> for OneOrMany<T> {
    fn from(from: T) -> OneOrMany<T> {
        OneOrMany::One(from)
    }
}

impl<T: ToJson> From<Vec<T>> for OneOrMany<T> {
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
pub enum DistanceType {
    SloppyArc,
    Arc,
    Plane
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

impl ToJson for DistanceUnit {
    fn to_json(&self) -> Json {
        Json::String(self.to_string())
    }
}

/// A Json value that's not a structural thing - i.e. just String, i64 and f64,
/// no array or object
#[derive(Debug)]
pub enum JsonVal {
    String(String),
    I64(i64),
    U64(u64),
    F64(f64)
}

impl ToJson for JsonVal {
    fn to_json(&self) -> Json {
        match self {
            &JsonVal::String(ref str) => str.to_json(),
            &JsonVal::I64(i)          => Json::I64(i),
            &JsonVal::U64(u)          => Json::U64(u),
            &JsonVal::F64(f)          => Json::F64(f)
        }
    }
}

from!(String, JsonVal, String);

impl<'a> From<&'a str> for JsonVal {
    fn from(from: &'a str) -> JsonVal {
        JsonVal::String(from.to_owned())
    }
}

from!(f64, JsonVal, F64);
from_exp!(i32, JsonVal, from, JsonVal::I64(from as i64));
from!(i64, JsonVal, I64);
from_exp!(u32, JsonVal, from, JsonVal::U64(from as u64));
from!(u64, JsonVal, U64);

impl<'a> From<&'a Json> for JsonVal {
    fn from(from: &'a Json) -> JsonVal {
        match from {
            &Json::String(ref s) => JsonVal::String(s.clone()),
            &Json::F64(f)        => JsonVal::F64(f),
            &Json::I64(f)        => JsonVal::I64(f),
            &Json::U64(f)        => JsonVal::U64(f),
            _                    => panic!("Not a String, F64, I64 or U64")
        }
    }
}
