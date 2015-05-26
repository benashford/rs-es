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

/// Every ES operation has a set of options
pub type Options<'a> = Vec<(&'a str, String)>;

/// Adds a function to an operation to add specific options to that operations
/// builder interface.
macro_rules! add_option {
    ($n:ident, $e:expr) => (
        pub fn $n<T: ToString>(&'a mut self, val: &T) -> &'a mut Self {
            self.options.push(($e, val.to_string()));
            self
        }
    )
}
