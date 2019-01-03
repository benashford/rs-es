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

use crate::json::FieldBased;

// Helper macros

/// Build the `build` function for each builder struct
macro_rules! build {
    ($t:ident) => (
        pub fn build(self) -> Query {
            Query::$t(Box::new(self))
        }
    )
}

pub type FieldBasedQuery<I, O> = FieldBased<String, I, O>;
