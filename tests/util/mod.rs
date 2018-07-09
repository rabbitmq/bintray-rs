extern crate env_logger;
extern crate nanoid;

use std::env;

pub static SUBJECT: &'static str = "bintray-rust-crate-testing";
pub static PREEXISTING_REPO: &'static str = "preexisting-generic-repository";
pub static PREEXISTING_PACKAGE: &'static str = "preexisting-package";
pub static PREEXISTING_VERSION_1: &'static str = "preexisting-version-1";
pub static PREEXISTING_VERSION_2: &'static str = "preexisting-version-2";

pub static LICENSES: &'static [&'static str] = &["BSD 2-Clause"];
pub static VCS_URL: &'static str = "https://github.com/rabbitmq/bintray-rs";

pub fn init_env_logger()
{
    let _ = env_logger::try_init();
}

pub fn get_credentials() -> Option<(String, String)>
{
    match env::var("BINTRAY_USERNAME") {
        Ok(username) => {
            match env::var("BINTRAY_API_KEY") {
                Ok(api_key) => Some((username, api_key)),
                Err(_)      => None
            }
        }
        Err(_) => {
            None
        }
    }
}

pub fn random_name(length: usize) -> String
{
    let alphabet: [char; 36] = [
        '1', '2', '3', '4', '5', '6', '7', '8', '9', '0',
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j',
        'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't',
        'u', 'v', 'w', 'x', 'y', 'z'
    ];

    nanoid::custom(length, &alphabet)
}
