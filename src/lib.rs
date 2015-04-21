#![crate_type = "lib"]
#![crate_name = "rs_es"]

#![feature(convert)]

#[macro_use] extern crate log;
extern crate hyper;
extern crate rustc_serialize;

use std::error::Error;
use std::io;
use std::fmt;

use rustc_serialize::json;
use rustc_serialize::json::Json;

// Error handling

#[derive(Debug)]
pub enum EsError {
    EsError(String),
    HttpError(hyper::error::HttpError),
    IoError(io::Error),
    JsonError(json::BuilderError)
}

impl From<io::Error> for EsError {
    fn from(err: io::Error) -> EsError {
        EsError::IoError(err)
    }
}

impl From<hyper::error::HttpError> for EsError {
    fn from(err: hyper::error::HttpError) -> EsError {
        EsError::HttpError(err)
    }
}

impl From<json::BuilderError> for EsError {
    fn from(err: json::BuilderError) -> EsError {
        EsError::JsonError(err)
    }
}

impl Error for EsError {
    fn description(&self) -> &str {
        match *self {
            EsError::EsError(ref err) => err.as_str(),
            EsError::HttpError(ref err) => err.description(),
            EsError::IoError(ref err) => err.description(),
            EsError::JsonError(ref err) => err.description()
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            EsError::EsError(_)         => None,
            EsError::HttpError(ref err) => Some(err as &Error),
            EsError::IoError(ref err)   => Some(err as &Error),
            EsError::JsonError(ref err) => Some(err as &Error)
        }
    }
}

impl fmt::Display for EsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EsError::EsError(ref s) => fmt::Display::fmt(s, f),
            EsError::HttpError(ref err) => fmt::Display::fmt(err, f),
            EsError::IoError(ref err) => fmt::Display::fmt(err, f),
            EsError::JsonError(ref err) => fmt::Display::fmt(err, f)
        }
    }
}

// The client

pub struct Client {
    host:        String,
    port:        u32,
    http_client: hyper::Client
}

impl Client {
    pub fn new(host: String, port: u32) -> Client {
        Client {
            host:        host,
            port:        port,
            http_client: hyper::Client::new()
        }
    }

    fn get_base_url(&self) -> String {
        format!("http://{}:{}/", self.host, self.port)
    }

    fn get(&mut self, url: &str, body: Option<&Json>) -> Result<Json, EsError> {
        let rb = self.http_client.get(url);
        let mut result = try!(match body {
            Some(json) => rb.body(json.as_string().unwrap()),
            None       => rb
        }.send());
        Ok(try!(Json::from_reader(&mut result)))
    }

    pub fn version(&mut self) -> Result<String, EsError> {
        let url = self.get_base_url();
        let json = try!(self.get(url.as_str(), None));
        match json.find_path(&["version", "number"]) {
            Some(version) => match version.as_string() {
                Some(string) => Ok(string.to_string()),
                None         => Err(EsError::EsError(format!("Cannot find version number in: {:?}",
                                                             json)))
            },
            None          => Err(EsError::EsError(format!("Cannot find version number in {:?}",
                                                          json)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Client;

    #[test]
    fn it_works() {
        let mut client = Client::new("localhost".to_string(), 9200);
        assert_eq!(client.version().unwrap(), "1.3.2");
    }
}
