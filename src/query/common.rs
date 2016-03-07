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

use serde::{Serialize, Serializer};
use serde::ser::MapVisitor;

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

macro_rules! add_inner_option {
    ($n:ident, $e:ident, $t:ty) => (
        pub fn $n<T: Into<$t>>(mut self, val: T) -> Self {
            self.0.inner.$e = Some(val.into());
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
#[derive(Debug)]
pub struct NoOuter;

impl Serialize for NoOuter {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        // No-op
        Ok(())
    }
}


/// Many QueryDSL objects are structured as {field_name: [map of options]}, with optional
/// options at the outer layer
#[derive(Debug)]
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

impl<I, O> Serialize for FieldBasedQuery<I, O>
    where I: Serialize,
          O: Serialize {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {

        serializer.serialize_struct("FieldBasedQuery", FieldBasedQueryMapVisitor {
            fbq: self,
            state: 0
        })
    }
}

struct FieldBasedQueryMapVisitor<'a, I: 'a, O: 'a> {
    fbq: &'a FieldBasedQuery<I, O>,
    state: u8
}

impl<'a, I, O> MapVisitor for FieldBasedQueryMapVisitor<'a, I, O>
    where I: Serialize,
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
