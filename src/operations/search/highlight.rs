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

//! Implementation of ElasticSearch [highlight](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-request-highlighting.html)

use std::collections::HashMap;

use serde::ser;
use serde::ser::{Serialize, Serializer};
use serde_json::{self, Value};

use ::error::EsError;

#[derive(Debug)]
pub enum SettingTypes {
    Plain
}

impl ToString for SettingTypes {
    fn to_string(&self) -> String {
        match self {
            &SettingTypes::Plain => "plain"
        }.to_owned()
    }
}

impl Serialize for SettingTypes {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        self.to_string().serialize(serializer)
    }
}

#[derive(Debug, Serialize)]
pub struct Setting {
    #[serde(rename="type")]
    pub setting_type: SettingTypes
}

impl Setting {
    pub fn with_type(setting_type: SettingTypes) -> Setting {
        Setting { setting_type: setting_type }
    }
}

#[derive(Debug, Serialize)]
pub struct Highlight {
    pub fields: HashMap<String, Setting>
}

impl Highlight {
    /// Create an Highlight entity without any field or setting
    /// specified as they are supposed to be added via the `add`
    /// method.
    ///
    /// # Examples
    ///
    /// ```
    /// use rs_es::operations::search::highlight::{Highlight, Setting, SettingTypes};
    ///
    /// let mut highlight = Highlight::new();
    /// highlight.add("first_name".to_owned(), Setting::with_type(SettingTypes::Plain));
    /// ```
    pub fn new() -> Highlight {
        Highlight { fields: HashMap::new() }
    }

    /// Add a field to highlight to the set
    pub fn add(&mut self, name: String, setting: Setting) {
        self.fields.insert(name, setting);
    }
}

/// The fields containing found terms
pub type HighlightResult = HashMap<String, Vec<String>>;
