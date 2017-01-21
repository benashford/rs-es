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

#![crate_type = "lib"]
#![crate_name = "rs_es"]

//! A client for ElasticSearch's REST API
//!
//! The `Client` itself is used as the central access point, from which numerous
//! operations are defined implementing each of the specific ElasticSearch APIs.
//!
//! Warning: at the time of writing the majority of such APIs are currently
//! unimplemented.

#[cfg(feature = "serde_derive")]
#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate log;
extern crate hyper;

#[macro_use]
extern crate maplit;

extern crate url;

#[cfg(feature = "serde_derive")]
include!("lib.rs.in");

#[cfg(not(feature = "serde_derive"))]
include!(concat!(env!("OUT_DIR"), "/lib.rs"));
