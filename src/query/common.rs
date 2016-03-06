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

//! Common macros, utilities, etc. for the query crate

// Helper macros

/// This package is full of builder interfaces, with much repeated code for adding
/// optional fields.  This macro removes much of the repetition.
macro_rules! add_option {
    ($n:ident, $e:ident, $t:ty) => (
        pub fn $n<T: Into<$t>>(mut self, val: T) -> Self {
            self.$e = Some(val.into());
            self
        }
    )
}

/// Build the `build` function for each builder struct
macro_rules! build {
    ($t:ident) => (
        pub fn build(self) -> Query {
            Query::$t(Box::new(self))
        }
    )
}

/// No outer options
#[derive(Debug, Serialize)]
pub struct NoOuter;

/// Many QueryDSL objects are structured as {field_name: [map of options]}, with optional
/// options at the outer layer
#[derive(Debug, Serialize)]
pub struct FieldBasedQuery<I, O> {
    pub field: String,
    pub inner: I,
    pub outer: O
}

impl<I, O> FieldBasedQuery<I, O> {
    pub fn new(field: String, inner: I, outer: O) -> Self {
        FieldBasedQuery {
            field: field,
            inner: inner,
            outer: outer
        }
    }
}
