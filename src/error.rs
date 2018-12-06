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

//! Errors and error conversion code for the `rs_es` crate

use std::error::Error;
use std::fmt;
use std::io;

use reqwest;

use serde_json;

// Error handling

/// Error that can occur include IO and parsing errors, as well as specific
/// errors from the ElasticSearch server and logic errors from this library
#[derive(Debug)]
pub enum EsError {
    /// An internal error from this library
    EsError(String),

    /// An error reported in a JSON response from the ElasticSearch server
    EsServerError(String),

    /// Miscellaneous error from the HTTP library
    HttpError(reqwest::Error),

    /// Miscellaneous error from the URL library
    UrlError(reqwest::UrlError),

    /// Miscellaneous IO error
    IoError(io::Error),

    /// JSON error
    JsonError(serde_json::Error),
}

impl From<io::Error> for EsError {
    fn from(err: io::Error) -> EsError {
        EsError::IoError(err)
    }
}

impl From<reqwest::Error> for EsError {
    fn from(err: reqwest::Error) -> EsError {
        EsError::HttpError(err)
    }
}

impl<'a> From<&'a mut reqwest::Response> for EsError {
    fn from(err: &'a mut reqwest::Response) -> EsError {
        let body = match err.text() {
            Ok(text) => text,
            Err(_) => {
                return EsError::EsServerError(format!(
                    "{} - cannot read response - {:?}",
                    err.status(),
                    err
                ));
            }
        };
        EsError::EsServerError(format!("{} - {}", err.status(), body))
    }
}

impl From<reqwest::UrlError> for EsError {
    fn from(err: reqwest::UrlError) -> EsError {
        EsError::UrlError(err)
    }
}

impl From<serde_json::Error> for EsError {
    fn from(err: serde_json::Error) -> EsError {
        EsError::JsonError(err)
    }
}

impl Error for EsError {
    fn description(&self) -> &str {
        match *self {
            EsError::EsError(ref err) => err,
            EsError::EsServerError(ref err) => err,
            EsError::HttpError(ref err) => err.description(),
            EsError::IoError(ref err) => err.description(),
            EsError::JsonError(ref err) => err.description(),
            EsError::UrlError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            EsError::EsError(_) => None,
            EsError::EsServerError(_) => None,
            EsError::HttpError(ref err) => Some(err as &Error),
            EsError::IoError(ref err) => Some(err as &Error),
            EsError::JsonError(ref err) => Some(err as &Error),
            EsError::UrlError(ref err) => Some(err as &Error),
        }
    }
}

impl fmt::Display for EsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EsError::EsError(ref s) => fmt::Display::fmt(s, f),
            EsError::EsServerError(ref s) => fmt::Display::fmt(s, f),
            EsError::HttpError(ref err) => fmt::Display::fmt(err, f),
            EsError::IoError(ref err) => fmt::Display::fmt(err, f),
            EsError::JsonError(ref err) => fmt::Display::fmt(err, f),
            EsError::UrlError(ref err) => fmt::Display::fmt(err, f),
        }
    }
}
