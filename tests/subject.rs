extern crate bintray;
extern crate env_logger;

use bintray::Client;

#[allow(dead_code)]
mod util;

#[test]
fn list_repositories_anonymously() {
    util::init_env_logger();

    let client = Client::new().unwrap();

    let repository_names = client
        .subject(util::SUBJECT)
        .repository_names().unwrap();

    assert!(!repository_names.is_empty());
    assert!(repository_names.contains(&String::from(util::PREEXISTING_REPO)));
}
