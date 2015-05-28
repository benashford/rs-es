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

use rustc_serialize::json::{Json, ToJson};

/// The units by which duration is measured.
///
/// TODO - this list is incomplete
#[derive(Clone)]
pub enum DurationUnit {
    Week,
    Day,
    Hour
}

impl ToString for DurationUnit {
    fn to_string(&self) -> String {
        match *self {
            DurationUnit::Week => "w",
            DurationUnit::Day  => "d",
            DurationUnit::Hour => "h"
        }.to_string()
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
#[derive(Clone)]
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
