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

//! Implementation of the Bulk API

use std::collections::BTreeMap;

use hyper::status::StatusCode;

use rustc_serialize::json;
use rustc_serialize::json::{Json, ToJson};

use ::Client;
use ::do_req;
use ::error::EsError;
use ::units::Duration;

use super::common::{Options, VersionType};
use super::format_query_string;

pub struct ActionSource {
    doc:           Option<Json>,
    upsert:        Option<Json>,
    doc_as_upsert: Option<bool>,
    script:        Option<String>,
    params:        Option<Json>,
    lang:          Option<String>
}

impl ActionSource {
    pub fn new() -> ActionSource {
        ActionSource {
            doc:           None,
            upsert:        None,
            doc_as_upsert: None,
            script:        None,
            params:        None,
            lang:          None
        }
    }

    add_field!(with_doc, doc, Json);
    add_field!(with_upsert, upsert, Json);
    add_field!(with_doc_as_upsert, doc_as_upsert, bool);
    add_field!(with_script, script, String);
    add_field!(with_params, params, Json);
    add_field!(with_lang, lang, String);
}

impl ToJson for ActionSource {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();

        optional_add!(d, self.doc, "doc");
        optional_add!(d, self.upsert, "upsert");
        optional_add!(d, self.doc_as_upsert, "doc_as_upsert");
        optional_add!(d, self.script, "script");
        optional_add!(d, self.params, "params");
        optional_add!(d, self.lang, "lang");

        Json::Object(d)
    }
}

pub enum ActionType {
    Index,
    Create,
    Delete,
    Update
}

impl<'a> From<&'a String> for ActionType {
    fn from(from: &'a String) -> ActionType {
        if from == "index" {
            ActionType::Index
        } else if from == "create" {
            ActionType::Create
        } else if from == "delete" {
            ActionType::Delete
        } else if from == "update" {
            ActionType::Update
        } else {
            panic!("Unknown action type: {}", from)
        }
    }
}

impl ToString for ActionType {
    fn to_string(&self) -> String {
        match *self {
            ActionType::Index => "index",
            ActionType::Create => "create",
            ActionType::Delete => "delete",
            ActionType::Update => "update"
        }.to_string()
    }
}

/// A bulk operation consists of one or more `Action`s.
pub struct Action {
    action:            ActionType,
    index:             Option<String>,
    doc_type:          Option<String>,
    id:                Option<String>,
    version:           Option<i64>,
    version_type:      Option<VersionType>,
    routing:           Option<String>,
    parent:            Option<String>,
    timestamp:         Option<String>,
    ttl:               Option<Duration>,
    retry_on_conflict: Option<i64>,
    source:            Option<Json>
}

impl Action {
    /// An index action.
    ///
    /// Takes the document to be indexed, other parameters can be set as
    /// optional on the `Action` struct returned.
    ///
    /// # Example
    ///
    /// ```
    /// use rs_es::operations::bulk::Action;
    ///
    /// let delete_action = Action::delete("doc_id");
    /// ```
    pub fn index<E: ToJson>(document: E) -> Action {
        Action {
            action:            ActionType::Index,
            index:             None,
            doc_type:          None,
            id:                None,
            version:           None,
            version_type:      None,
            routing:           None,
            parent:            None,
            timestamp:         None,
            ttl:               None,
            retry_on_conflict: None,
            source:            Some(document.to_json())
        }
    }

    /// Create action
    pub fn create<E: ToJson>(document: E) -> Action {
        Action {
            action:            ActionType::Create,
            index:             None,
            doc_type:          None,
            id:                None,
            version:           None,
            version_type:      None,
            routing:           None,
            parent:            None,
            timestamp:         None,
            ttl:               None,
            retry_on_conflict: None,
            source:            Some(document.to_json())
        }
    }

    pub fn delete<S: Into<String>>(id: S) -> Action {
        Action {
            action:            ActionType::Delete,
            index:             None,
            doc_type:          None,
            id:                Some(id.into()),
            version:           None,
            version_type:      None,
            routing:           None,
            parent:            None,
            timestamp:         None,
            ttl:               None,
            retry_on_conflict: None,
            source:            None
        }
    }

    pub fn update<S: Into<String>>(id: S, update: &ActionSource) -> Action {
        Action {
            action:            ActionType::Delete,
            index:             None,
            doc_type:          None,
            id:                Some(id.into()),
            version:           None,
            version_type:      None,
            routing:           None,
            parent:            None,
            timestamp:         None,
            ttl:               None,
            retry_on_conflict: None,
            source:            Some(update.to_json())
        }
    }

    add_field!(with_index, index, String);
    add_field!(with_doc_type, doc_type, String);
    add_field!(with_id, id, String);
    add_field!(with_version, version, i64);
    add_field!(with_version_type, version_type, VersionType);
    add_field!(with_routing, routing, String);
    add_field!(with_parent, parent, String);
    add_field!(with_timestamp, timestamp, String);
    add_field!(with_ttl, ttl, Duration);
    add_field!(with_retry_on_conflict, retry_on_conflict, i64);

    /// Add the serialized version of this action to the bulk `String`.
    fn add(&self, actstr: &mut String) -> Result<(), EsError> {
        let command_str = try!(json::encode(&self.to_json()));

        actstr.push_str(&command_str);
        actstr.push_str("\n");

        match self.source {
            Some(ref source) => {
                let payload_str = try!(json::encode(source));
                actstr.push_str(&payload_str);
                actstr.push_str("\n");
            },
            None             => ()
        }
        Ok(())
    }
}

impl ToJson for Action {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        let mut inner = BTreeMap::new();

        optional_add!(inner, self.index, "_index");
        optional_add!(inner, self.doc_type, "_type");
        optional_add!(inner, self.id, "_id");
        optional_add!(inner, self.version, "_version");
        optional_add!(inner, self.version_type, "_version_type");
        optional_add!(inner, self.routing, "_routing");
        optional_add!(inner, self.parent, "_parent");
        optional_add!(inner, self.timestamp, "_timestamp");
        optional_add!(inner, self.ttl, "_ttl");
        optional_add!(inner, self.retry_on_conflict, "_retry_on_conflict");

        d.insert(self.action.to_string(), Json::Object(inner));
        Json::Object(d)
    }
}

pub struct BulkOperation<'a, 'b> {
    client:   &'a mut Client,
    index:    Option<&'b str>,
    doc_type: Option<&'b str>,
    actions:  &'b [Action],
    options:  Options<'b>
}

impl<'a, 'b> BulkOperation<'a, 'b> {
    pub fn new(client: &'a mut Client, actions: &'b [Action]) -> BulkOperation<'a, 'b> {
        BulkOperation {
            client:   client,
            index:    None,
            doc_type: None,
            actions:  actions,
            options:  Options::new()
        }
    }

    pub fn with_index(&'b mut self, index: &'b str) -> &'b mut Self {
        self.index = Some(index);
        self
    }

    pub fn with_doc_type(&'b mut self, doc_type: &'b str) -> &'b mut Self {
        self.doc_type = Some(doc_type);
        self
    }

    add_option!(with_consistency, "consistency");
    add_option!(with_refresh, "refresh");

    fn format_url(&self) -> String {
        let mut url = String::new();
        url.push_str("/");
        match self.index {
            Some(index) => {
                url.push_str(index);
                url.push_str("/");
            },
            None        => ()
        }
        match self.doc_type {
            Some(doc_type) => {
                url.push_str(doc_type);
                url.push_str("/");
            },
            None           => ()
        }
        url.push_str("_bulk");
        url.push_str(&format_query_string(&self.options));
        url
    }

    fn format_actions(&self) -> String {
        let mut actstr = String::new();
        for action in self.actions {
            action.add(&mut actstr).unwrap();
        }
        actstr
    }

    pub fn send(&'b mut self) -> Result<BulkResult, EsError> {
        //
        // This function does not use the standard GET/POST/DELETE functions of
        // the client, as they serve the happy path of JSON-in/JSON-out, this
        // function does send send JSON in.
        //
        // Various parts of the client are reused where it makes sense.
        //
        let full_url = {
            let url = self.format_url();
            self.client.full_url(&url)
        };
        let body = self.format_actions();

        let mut result = try!(self.client.http_client
                              .post(&full_url)
                              .body(&body)
                              .send());

        let (status_code, json_opt) = try!(do_req(&mut result));

        match status_code {
            StatusCode::Ok => Ok(BulkResult::from(&json_opt.unwrap())),
            _              => Err(EsError::EsError(format!("Unexpected status: {}", status_code)))
        }
    }
}

/// The result of specific actions
pub struct ActionResult {
    pub action:   ActionType,
    pub index:    String,
    pub doc_type: String,
    pub version:  u64,
    pub status:   u64
}

impl<'a> From<&'a Json> for ActionResult {
    fn from(from: &'a Json) -> ActionResult {
        info!("ActionResult from: {:?}", from);

        let d = from.as_object().unwrap();
        assert_eq!(1, d.len());
        let (key, inner) = d.iter().next().unwrap();

        ActionResult {
            action:   ActionType::from(key),
            index:    get_json_string!(inner, "_index"),
            doc_type: get_json_string!(inner, "_type"),
            version:  get_json_u64!(inner, "_version"),
            status:   get_json_u64!(inner, "status")
        }
    }
}

/// The result of a bulk operation
pub struct BulkResult {
    pub errors: bool,
    pub items:  Vec<ActionResult>,
    pub took:   u64
}

impl<'a> From<&'a Json> for BulkResult {
    fn from(from: &'a Json) -> BulkResult {
        info!("Bulk result, result: {:?}", from);
        BulkResult {
            errors: get_json_bool!(from, "errors"),
            items:  from.find("items")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|item| {
                    ActionResult::from(item)
                })
                .collect(),
            took:   get_json_u64!(from, "took")
        }
    }
}
