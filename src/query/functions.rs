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

impl Weight {
    pub fn build(self) -> Function {
        Function::Weight(self)
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

impl RandomScore {
    pub fn build(self) -> Function {
        Function::RandomScore(self)
    }
}

impl ToJson for RandomScore {
    fn to_json(&self) -> Json {
        self.0.to_json()
    }
}

/// Field value factor function
#[derive(Debug, Default)]
pub struct FieldValueFactor {
    field: String,
    factor: Option<f64>,
    modifier: Option<Modifier>,
    missing: Option<JsonVal>
}

impl Function {
    pub fn build_field_value_factor<A>(field: A) -> FieldValueFactor
        where A: Into<String> {

        FieldValueFactor {
            field: field.into(),
            ..Default::default()
        }
    }
}

impl FieldValueFactor {
    add_option!(with_factor, factor, f64);
    add_option!(with_modifier, modifier, Modifier);
    add_option!(with_missing, missing, JsonVal);

    pub fn build(self) -> Function {
        Function::FieldValueFactor(self)
    }
}

impl ToJson for FieldValueFactor {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("field".to_owned(), self.field.to_json());
        optional_add!(self, d, factor);
        optional_add!(self, d, modifier);
        optional_add!(self, d, missing);
        Json::Object(d)
    }
}

/// Modifier for the FieldValueFactor function
#[derive(Debug)]
pub enum Modifier {
    None,
    Log,
    Log1p,
    Log2p,
    Ln,
    Ln1p,
    Ln2p,
    Square,
    Sqrt,
    Reciprocal,
}

impl ToJson for Modifier {
    fn to_json(&self) -> Json {
        match self {
            &Modifier::None => "none".to_json(),
            &Modifier::Log => "log".to_json(),
            &Modifier::Log1p => "log1p".to_json(),
            &Modifier::Log2p => "log2p".to_json(),
            &Modifier::Ln => "ln".to_json(),
            &Modifier::Ln1p => "ln1p".to_json(),
            &Modifier::Ln2p => "ln2p".to_json(),
            &Modifier::Square => "square".to_json(),
            &Modifier::Sqrt => "sqrt".to_json(),
            &Modifier::Reciprocal => "reciprocal".to_json(),
        }
    }
}
