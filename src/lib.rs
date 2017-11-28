//! Library to access Bintray's API.
//!
//! [Bintray](https://bintray.com/) is a service which provides software
//! package repositories. It supports several kinds of repositories such
//! as Debian, RPM or generic file storage.

extern crate core;
extern crate env_logger;
#[macro_use] extern crate hyper;
extern crate hyper_rustls;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate version_compare;

#[macro_use] pub mod utils;

pub mod client;
pub mod repository;
pub mod package;
pub mod version;
pub mod content;
