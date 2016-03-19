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

//! Implementation of the Bulk API

use hyper::status::StatusCode;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, MapVisitor, Visitor};
use serde_json;

use ::{Client, EsResponse};
use ::do_req;
use ::error::EsError;
use ::json::{FieldBased, NoOuter, ShouldSkip};
use ::units::Duration;

use super::ShardCountResult;
use super::common::{Options, OptionVal, VersionType};

pub enum ActionType {
    Index,
    Create,
    Delete,
    /// WARNING - currently un-implemented
    Update
}

impl Serialize for ActionType {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        self.to_string().serialize(serializer)
    }
}

impl ToString for ActionType {
    fn to_string(&self) -> String {
        match *self {
            ActionType::Index => "index",
            ActionType::Create => "create",
            ActionType::Delete => "delete",
            ActionType::Update => "update"
        }.to_owned()
    }
}

#[derive(Default, Serialize)]
pub struct ActionOptions {
    #[serde(rename="_index", skip_serializing_if="ShouldSkip::should_skip")]
    index:             Option<String>,
    #[serde(rename="_type", skip_serializing_if="ShouldSkip::should_skip")]
    doc_type:          Option<String>,
    #[serde(rename="_id", skip_serializing_if="ShouldSkip::should_skip")]
    id:                Option<String>,
    #[serde(rename="_version", skip_serializing_if="ShouldSkip::should_skip")]
    version:           Option<u64>,
    #[serde(rename="_version_type", skip_serializing_if="ShouldSkip::should_skip")]
    version_type:      Option<VersionType>,
    #[serde(rename="_routing", skip_serializing_if="ShouldSkip::should_skip")]
    routing:           Option<String>,
    #[serde(rename="_parent", skip_serializing_if="ShouldSkip::should_skip")]
    parent:            Option<String>,
    #[serde(rename="_timestamp", skip_serializing_if="ShouldSkip::should_skip")]
    timestamp:         Option<String>,
    #[serde(rename="_ttl", skip_serializing_if="ShouldSkip::should_skip")]
    ttl:               Option<Duration>,
    #[serde(rename="_retry_on_conflict", skip_serializing_if="ShouldSkip::should_skip")]
    retry_on_conflict: Option<u64>,
}

#[derive(Serialize)]
pub struct Action<X>(FieldBased<ActionType, ActionOptions, NoOuter>, Option<X>);

impl<S> Action<S>
    where S: Serialize {
    /// An index action.
    ///
    /// Takes the document to be indexed, other parameters can be set as
    /// optional on the `Action` struct returned.
    pub fn index(document: S) -> Self {
        Action(FieldBased::new(ActionType::Index,
                               Default::default(),
                               NoOuter),
               Some(document))
    }

    /// Create action
    pub fn create(document: S) -> Self {
        Action(FieldBased::new(ActionType::Create,
                               Default::default(),
                               NoOuter),
               Some(document))
    }

    /// Add the serialized version of this action to the bulk `String`.
    fn add(&self, actstr: &mut String) -> Result<(), EsError> {
        let command_str = try!(serde_json::to_string(&self.0));

        actstr.push_str(&command_str);
        actstr.push_str("\n");

        match self.1 {
            Some(ref source) => {
                let payload_str = try!(serde_json::to_string(source));
                actstr.push_str(&payload_str);
                actstr.push_str("\n");
            },
            None             => ()
        }
        Ok(())
    }
}

impl<S> Action<S> {
    /// Delete a document based on ID.
    ///
    /// # Example
    ///
    /// ```
    /// use rs_es::operations::bulk::Action;
    ///
    /// let delete_action:Action<()> = Action::delete("doc_id");
    /// let delete_with_index:Action<()> = Action::delete("doc_id").with_index("index_name");
    /// ```
    pub fn delete<A: Into<String>>(id: A) -> Self {
        Action(FieldBased::new(ActionType::Delete,
                               ActionOptions {
                                   id: Some(id.into()),
                                   ..Default::default()
                               },
                               NoOuter),
               None)
    }

    // TODO - implement update

    add_inner_field!(with_index, index, String);
    add_inner_field!(with_doc_type, doc_type, String);
    add_inner_field!(with_id, id, String);
    add_inner_field!(with_version, version, u64);
    add_inner_field!(with_version_type, version_type, VersionType);
    add_inner_field!(with_routing, routing, String);
    add_inner_field!(with_parent, parent, String);
    add_inner_field!(with_timestamp, timestamp, String);
    add_inner_field!(with_ttl, ttl, Duration);
    add_inner_field!(with_retry_on_conflict, retry_on_conflict, u64);
}

pub struct BulkOperation<'a, 'b, S: 'b> {
    client:   &'a mut Client,
    index:    Option<&'b str>,
    doc_type: Option<&'b str>,
    actions:  &'b [Action<S>],
    options:  Options<'b>
}

impl<'a, 'b, S> BulkOperation<'a, 'b, S>
    where S: Serialize {

    pub fn new(client: &'a mut Client, actions: &'b [Action<S>]) -> Self {
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
        url.push_str(&self.options.to_string());
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
        debug!("Sending: {}", body);
        // Doesn't use the standard macros as it's not standard JSON
        let result = try!(self.client.http_client
                          .post(&full_url)
                          .body(&body)
                          .send());

        let response = try!(do_req(result));

        match response.status_code() {
            &StatusCode::Ok => Ok(try!(response.read_response())),
            _              => Err(EsError::EsError(format!("Unexpected status: {}", response.status_code())))
        }
    }
}

impl Client {
    /// Bulk
    ///
    /// See: https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html
    pub fn bulk<'a, 'b, S>(&'a mut self,
                           actions: &'b [Action<S>]) -> BulkOperation<'a, 'b, S>
        where S: Serialize {

        BulkOperation::new(self, actions)
    }
}

/// The result of specific actions
pub struct ActionResult {
    pub action: ActionType,
    pub inner: ActionResultInner
}

impl Deserialize for ActionResult {
    fn deserialize<D>(deserializer: &mut D) -> Result<ActionResult, D::Error>
        where D: Deserializer {

        struct ActionResultVisitor;

        impl Visitor for ActionResultVisitor {
            type Value = ActionResult;

            fn visit_map<V>(&mut self, mut visitor: V) -> Result<ActionResult, V::Error>
                where V: MapVisitor {

                let visited:Option<(String, ActionResultInner)> = try!(visitor.visit());
                let (key, value) = match visited {
                    Some((key, value)) => (key, value),
                    None               => return Err(V::Error::custom("expecting at least one field"))
                };
                try!(visitor.end());

                let result = ActionResult {
                    action: match key.as_ref() {
                        "index" => ActionType::Index,
                        "create" => ActionType::Create,
                        "delete" => ActionType::Delete,
                        "update" => ActionType::Update,
                        _ => {
                            return Err(V::Error::custom(format!("Unrecognised key: {}", key)))
                        }
                    },
                    inner:  value
                };

                Ok(result)
            }
        }

        deserializer.deserialize(ActionResultVisitor)
    }
}

#[derive(Debug, Deserialize)]
pub struct ActionResultInner {
    #[serde(rename="_index")]
    pub index:    String,
    #[serde(rename="_type")]
    pub doc_type: String,
    #[serde(rename="_version")]
    pub version:  u64,
    pub status:   u64,
    #[serde(rename="_shards")]
    pub shards:   ShardCountResult,
    pub found:    Option<bool>
}

/// The result of a bulk operation
#[derive(Deserialize)]
pub struct BulkResult {
    pub errors: bool,
    pub items:  Vec<ActionResult>,
    pub took:   u64
}

#[cfg(test)]
pub mod tests {
    use ::tests::{clean_db, TestDocument, make_client};

    use super::Action;

    #[test]
    fn test_bulk() {
        let index_name = "test_bulk";
        let mut client = make_client();

        clean_db(&mut client, index_name);

        let actions:Vec<Action<TestDocument>> = (1..10).map(|i| {
            let doc = TestDocument::new().with_str_field("bulk_doc").with_int_field(i);
            Action::index(doc)
        }).collect();

        let result = client.bulk(&actions)
            .with_index(index_name)
            .with_doc_type("bulk_type")
            .send()
            .unwrap();

        assert_eq!(false, result.errors);
        assert_eq!(9, result.items.len());
    }
}
