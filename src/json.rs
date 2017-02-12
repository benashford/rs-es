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

//! Helper for common requirements when producing/parsing JSON

use serde::ser::{Serialize, Serializer, SerializeMap};

/// To tell Serde to skip various fields
pub trait ShouldSkip {
    fn should_skip(&self) -> bool;
}

/// To indicate whether an optional field should be skipped if None
impl<T> ShouldSkip for Option<T> {
    fn should_skip(&self) -> bool {
        self.is_none()
    }
}

/// Useful serialization functions
pub fn serialize_map_optional_kv<S, K, V>(map_ser: &mut S,
                                          key: K,
                                          value: &Option<V>) -> Result<(), S::Error>
    where S: SerializeMap,
          K: Serialize,
          V: Serialize {
    match value {
        &Some(ref x) => {
            map_ser.serialize_entry(&key, &x)?;
        }
        &None => ()
    }
    Ok(())
}

/// No outer options
///
/// Literally serializes to nothing
#[derive(Debug, Default)]
pub struct NoOuter;

impl MergeSerialize for NoOuter {
    fn merge_serialize<S>(&self,
                          _: &mut S) -> Result<(), S::Error>
        where S: SerializeMap {

        // No-op
        Ok(())
    }
}

/// A recurring theme in ElasticSearch is for JSON to be `{"variable": {..map of options..}`
#[derive(Debug)]
pub struct FieldBased<F, I, O> {
    pub field: F,
    pub inner: I,
    pub outer: O
}

impl<F, I, O> FieldBased<F, I, O> {
    pub fn new(field: F, inner: I, outer: O) -> Self {
        FieldBased {
            field: field,
            inner: inner,
            outer: outer
        }
    }
}

impl<F, I, O> Serialize for FieldBased<F, I, O>
    where F: Serialize,
          I: Serialize,
          O: MergeSerialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer {

        let mut map = try!(serializer.serialize_map(None));

        map.serialize_entry(&self.field, &self.inner)?;
        self.outer.merge_serialize(&mut map)?;

        map.end()
    }
}

/// MergeSerialize, implemented by structs that want to add to an existing struct
pub trait MergeSerialize {
    fn merge_serialize<S>(&self,
                          serializer: &mut S) -> Result<(), S::Error>
        where S: SerializeMap;
}

/// Macro to allow access to the inner object, assumes FieldBased is wrapped in a newtype
macro_rules! add_inner_field {
    ($n:ident, $f:ident, $t:ty) => (
        pub fn $n<T: Into<$t>>(mut self, val: T) -> Self {
            self.0.inner.$f = Some(val.into());
            self
        }
    );
}

macro_rules! add_outer_field {
    ($n:ident, $e:ident, $t:ty) => (
        pub fn $n<T: Into<$t>>(mut self, val: T) -> Self {
            self.0.outer.$e = Some(val.into());
            self
        }
    )
}

#[cfg(test)]
pub mod tests {
    use serde_json;

    use serde::ser::SerializeMap;

    use super::{FieldBased, MergeSerialize, NoOuter};

    #[derive(Serialize)]
    struct TestOptions {
        opt_a: i64,
        opt_b: f64
    }

    impl MergeSerialize for TestOptions {
        fn merge_serialize<S>(&self,
                              serializer: &mut S) -> Result<(), S::Error>
            where S: SerializeMap {

            serializer.serialize_entry("opt_a", &self.opt_a)?;
            serializer.serialize_entry("opt_b", &self.opt_b)
        }
    }

    #[derive(Serialize)]
    struct TestStruct(FieldBased<String, TestOptions, NoOuter>);

    impl TestStruct {
        fn new(key: String, options: TestOptions) -> Self {
            TestStruct(FieldBased::new(key, options, NoOuter))
        }
    }

    #[derive(Serialize)]
    struct TestWithOuter(FieldBased<String, TestOptions, TestOptions>);

    impl TestWithOuter {
        fn new(key: String, options: TestOptions, outer: TestOptions) -> Self {
            TestWithOuter(FieldBased::new(key, options, outer))
        }
    }

    #[test]
    fn test_simple_field_based() {
        let t = TestStruct::new("key".to_owned(),
                                TestOptions {opt_a: 4i64, opt_b: 3.5f64});
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!("{\"key\":{\"opt_a\":4,\"opt_b\":3.5}}", s);
    }

    #[test]
    fn test_outer_field_based() {
        let t = TestWithOuter::new("key".to_owned(),
                                   TestOptions {opt_a: 8i64, opt_b: 2.5f64},
                                   TestOptions {opt_a: 9i64, opt_b: 1.5f64});
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!("{\"key\":{\"opt_a\":8,\"opt_b\":2.5},\"opt_a\":9,\"opt_b\":1.5}", s);
    }
}
