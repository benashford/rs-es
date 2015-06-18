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

//! Implementation of the Get API

use rustc_serialize::Decodable;
use rustc_serialize::json::Json;

use ::Client;
use ::error::EsError;
use ::util::StrJoin;
use super::common::{Options, OptionVal};
use super::decode_json;
use super::format_query_string;

/// Values for the `preference` query parameter
pub enum Preference {
    Primary,
    Local
}

impl From<Preference> for OptionVal {
    fn from(from: Preference) -> OptionVal {
        OptionVal(match from {
            Preference::Primary => "_primary",
            Preference::Local => "_local"
        }.to_owned())
    }
}

/// An ES GET operation, to get a document by ID
pub struct GetOperation<'a, 'b> {
    /// The HTTP connection
    client:   &'a mut Client,

    /// The index to load the document.
    index:    &'b str,

    /// Optional type
    doc_type: Option<&'b str>,

    /// The ID of the document.
    id:       &'b str,

    /// Optional options
    options:  Options<'b>
}

impl<'a, 'b> GetOperation<'a, 'b> {
    pub fn new(client:   &'a mut Client,
               index:    &'b str,
               id:       &'b str) -> GetOperation<'a, 'b> {
        GetOperation {
            client:   client,
            index:    index,
            doc_type: None,
            id:       id,
            options:  Options::new()
        }
    }

    pub fn with_all_types(&'b mut self) -> &'b mut Self {
        self.doc_type = Some("_all");
        self
    }

    pub fn with_doc_type(&'b mut self, doc_type: &'b str) -> &'b mut Self {
        self.doc_type = Some(doc_type);
        self
    }

    pub fn with_fields(&'b mut self, fields: &[&'b str]) -> &'b mut Self {
        self.options.push("fields", fields.iter().join(","));
        self
    }

    add_option!(with_realtime, "realtime");
    add_option!(with_source, "_source");
    add_option!(with_routing, "routing");
    add_option!(with_preference, "preference");
    add_option!(with_refresh, "refresh");
    add_option!(with_version, "version");

    pub fn send(&'b mut self) -> Result<GetResult, EsError> {
        let url = format!("/{}/{}/{}{}",
                          self.index,
                          self.doc_type.unwrap(),
                          self.id,
                          format_query_string(&self.options));
        // We're ignoring status_code as all valid codes should return a value,
        // so anything else is an error.
        let (_, result) = try!(self.client.get_op(&url));
        Ok(GetResult::from(&result.unwrap()))
    }
}

/// The result of a GET request
#[derive(Debug)]
pub struct GetResult {
    pub index:    String,
    pub doc_type: String,
    pub id:       String,
    pub version:  Option<i64>,
    pub found:    bool,
    pub source:   Option<Json>
}

impl GetResult {
    /// The result is a JSON document, this function will attempt to decode it
    /// to a struct.  If the raw JSON is required, it can accessed directly from
    /// the source field of the `GetResult` struct.
    pub fn source<T: Decodable>(self) -> Result<T, EsError> {
        match self.source {
            Some(doc) => decode_json(doc),
            None      => Err(EsError::EsError("No source".to_owned()))
        }
    }
}

impl<'a> From<&'a Json> for GetResult {
    fn from(r: &'a Json) -> GetResult {
        info!("GetResult FROM: {:?}", r);
        GetResult {
            index:    get_json_string!(r, "_index"),
            doc_type: get_json_string!(r, "_type"),
            id:       get_json_string!(r, "_id"),
            version:  r.search("_version").map(|v| v.as_i64().unwrap()),
            found:    get_json_bool!(r, "found"),
            source:   r.search("_source").map(|source| source.clone())
        }
    }
}
