use std::error::Error;
use std::fmt;
use std::io;

use hyper;
use hyper::client::response;
use rustc_serialize::json;

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
    HttpError(hyper::error::Error),

    /// Miscellaneous IO error
    IoError(io::Error),

    /// Miscellaneous JSON decoding error
    JsonError(json::DecoderError),

    /// Miscllenaeous JSON building error
    JsonBuilderError(json::BuilderError)
}

impl From<io::Error> for EsError {
    fn from(err: io::Error) -> EsError {
        EsError::IoError(err)
    }
}

impl From<hyper::error::Error> for EsError {
    fn from(err: hyper::error::Error) -> EsError {
        EsError::HttpError(err)
    }
}

impl From<json::DecoderError> for EsError {
    fn from(err: json::DecoderError) -> EsError {
        EsError::JsonError(err)
    }
}

impl From<json::BuilderError> for EsError {
    fn from(err: json::BuilderError) -> EsError {
        EsError::JsonBuilderError(err)
    }
}

impl<'a> From<&'a mut response::Response> for EsError {
    fn from(err: &'a mut response::Response) -> EsError {
        EsError::EsServerError(format!("{} - {:?}", err.status, err))
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
            EsError::JsonBuilderError(ref err) => err.description()
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            EsError::EsError(_)                => None,
            EsError::EsServerError(_)          => None,
            EsError::HttpError(ref err)        => Some(err as &Error),
            EsError::IoError(ref err)          => Some(err as &Error),
            EsError::JsonError(ref err)        => Some(err as &Error),
            EsError::JsonBuilderError(ref err) => Some(err as &Error)
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
            EsError::JsonBuilderError(ref err) => fmt::Display::fmt(err, f)
        }
    }
}
