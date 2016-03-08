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

use serde::{Serialize, Serializer};
use serde::ser::MapVisitor;

// TODO: this contains Serde related functions, there are old rustc_serialize helpers
// elsewhere, they should be deleted or moved/updated here.

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

/// No outer options
#[derive(Debug)]
pub struct NoOuter;

impl Serialize for NoOuter {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

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
          O: Serialize {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        serializer.serialize_struct("FieldBasedQuery", FieldBasedMapVisitor {
            fbq: self,
            state: 0
        })
    }
}

struct FieldBasedMapVisitor<'a, F: 'a, I: 'a, O: 'a> {
    fbq: &'a FieldBased<F, I, O>,
    state: u8
}

impl<'a, F, I, O> MapVisitor for FieldBasedMapVisitor<'a, F, I, O>
    where F: Serialize,
          I: Serialize,
          O: Serialize {
    fn visit<S>(&mut self, serializer: &mut S) -> Result<Option<()>, S::Error>
        where S: Serializer {

        match self.state {
            0 => {
                self.state += 1;
                Ok(Some(try!(serializer.serialize_map_elt(&self.fbq.field, &self.fbq.inner))))
            },
            1 => {
                self.state += 1;
                Ok(Some(try!(self.fbq.outer.serialize(serializer))))
            },
            _ => {
                Ok(None)
            }
        }
    }
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

#[cfg(test)]
pub mod tests {
    use serde_json;

    use super::{FieldBased, NoOuter};

    #[derive(Serialize)]
    struct TestOptions {
        opt_a: i64,
        opt_b: f64
    }

    #[derive(Serialize)]
    struct TestStruct(FieldBased<String, TestOptions, NoOuter>);

    impl TestStruct {
        fn new(key: String, options: TestOptions) -> TestStruct {
            TestStruct(FieldBased::new(key, options, NoOuter))
        }
    }

    #[test]
    fn test_simple_field_based() {
        let t = TestStruct::new("key".to_owned(),
                                TestOptions {opt_a: 4i64, opt_b: 3.0f64});
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!("", s);
    }
}
