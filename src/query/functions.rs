/*
 * Copyright 2016 Ben Ashford
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

//! Specific options for the Function option of various queries

use std::collections::{BTreeMap, HashMap};

use rustc_serialize::json::{Json, ToJson};

use ::units::JsonVal;

use super::Function;

/// ScriptScore function
#[derive(Debug, Default)]
pub struct ScriptScore {
    lang: Option<String>,
    params: HashMap<String, JsonVal>,
    inline: String
}

impl Function {
    pub fn build_script_score<A>(script: A) -> ScriptScore
        where A: Into<String> {

        ScriptScore {
            inline: script.into(),
            ..Default::default()
        }
    }
}

impl ScriptScore {
    add_option!(with_lang, lang, String);

    pub fn with_params<A>(mut self, params: A) -> Self
        where A: IntoIterator<Item=(String, JsonVal)> {

        self.params.extend(params);
        self
    }

    pub fn add_param<A, B>(mut self, key: A, value: B) -> Self
        where A: Into<String>,
              B: Into<JsonVal> {
        self.params.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> Function {
        Function::ScriptScore(self)
    }
}

impl ToJson for ScriptScore {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("inline".to_owned(), self.inline.to_json());
        d.insert("params".to_owned(), self.params.to_json());
        optional_add!(self, d, lang);
        Json::Object(d)
    }
}

/// Weight function
#[derive(Debug, Default)]
pub struct Weight(f64);

impl Function {
    pub fn build_weight<A>(weight: A) -> Weight
        where A: Into<f64> {

        Weight(weight.into())
    }
}

impl ToJson for Weight {
    fn to_json(&self) -> Json {
        self.0.to_json()
    }
}

/// Random score function
#[derive(Debug, Default)]
pub struct RandomScore(i64);

impl Function {
    pub fn build_random_score<A>(seed: A) -> RandomScore
        where A: Into<i64> {

        RandomScore(seed.into())
    }
}

impl ToJson for RandomScore {
    fn to_json(&self) -> Json {
        self.0.to_json()
    }
}
