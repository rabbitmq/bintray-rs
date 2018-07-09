extern crate bintray;
extern crate chrono;
extern crate env_logger;

use chrono::Utc;
use bintray::{Client, PackageMaturity};

#[allow(dead_code)]
mod util;

#[test]
fn get_package_anonymously() {
    util::init_env_logger();

    let client = Client::new().unwrap();

    let package = client
        .subject(util::SUBJECT)
        .repository(util::PREEXISTING_REPO)
        .package(util::PREEXISTING_PACKAGE)
        .get().unwrap();

    assert_eq!(package.get_subject(), util::SUBJECT);
    assert_eq!(package.get_repository(), util::PREEXISTING_REPO);
    assert_eq!(package.get_name(), util::PREEXISTING_PACKAGE);
    assert!(package.get_desc().contains("expected by the testsuite"));
}

#[test]
#[should_panic(expected = "This resource requires authentication")]
fn create_package_anonymously() {
    util::init_env_logger();

    let client = Client::new().unwrap();

    let package_name = util::random_name(16);

    client
        .subject(util::SUBJECT)
        .repository(util::PREEXISTING_REPO)
        .package(&package_name)
        .licenses(util::LICENSES)
        .vcs_url(util::VCS_URL)
        .create().unwrap();
}

#[test]
fn create_and_delete_package_as_authenticated_user() {
    util::init_env_logger();

    match util::get_credentials() {
        Some((username, api_key)) => {
            let client = Client::new().unwrap()
                .user(&username, &api_key);

            let package_name = util::random_name(16);

            assert!(!client
                    .subject(util::SUBJECT)
                    .repository(util::PREEXISTING_REPO)
                    .package(&package_name)
                    .exists().unwrap());

            let package = client
                .subject(util::SUBJECT)
                .repository(util::PREEXISTING_REPO)
                .package(&package_name)
                .desc("")
                .labels(&vec![] as &Vec<String>)
                .licenses(util::LICENSES)
                .vcs_url(util::VCS_URL)
                .create().unwrap();

            assert_eq!(package.get_subject(), util::SUBJECT);
            assert_eq!(package.get_repository(), util::PREEXISTING_REPO);
            assert_eq!(package.get_name(), &package_name);
            assert_eq!(package.get_desc(), "");
            assert_eq!(package.get_labels(), &vec![] as &Vec<String>);
            assert_eq!(package.get_licenses(), &util::LICENSES);
            assert_eq!(package.get_vcs_url(), util::VCS_URL);
            assert_eq!(package.get_created().unwrap().date(), Utc::now().date());

            assert!(client
                    .subject(util::SUBJECT)
                    .repository(util::PREEXISTING_REPO)
                    .package(&package_name)
                    .exists().unwrap());

            let mut package = client
                .subject(util::SUBJECT)
                .repository(util::PREEXISTING_REPO)
                .package(&package_name)
                .get().unwrap();

            assert_eq!(package.get_subject(), util::SUBJECT);
            assert_eq!(package.get_repository(), util::PREEXISTING_REPO);
            assert_eq!(package.get_name(), &package_name);
            assert_eq!(package.get_desc(), "");
            assert_eq!(package.get_labels(), &vec![] as &Vec<String>);
            assert_eq!(package.get_licenses(), &util::LICENSES);
            assert_eq!(package.get_vcs_url(), util::VCS_URL);
            assert_eq!(package.get_created().unwrap().date(), Utc::now().date());

            let desc = "Temporary package created from a testsuite";
            let mut labels = ["testing", "rust"];
            labels.sort();

            package
                .set_desc(desc)
                .set_labels(&labels)
                .set_maturity(&PackageMaturity::Development)
                .update().unwrap();

            assert_eq!(package.get_subject(), util::SUBJECT);
            assert_eq!(package.get_repository(), util::PREEXISTING_REPO);
            assert_eq!(package.get_name(), &package_name);
            assert_eq!(package.get_desc(), desc);
            assert_eq!(package.get_labels(), &labels);
            assert_eq!(package.get_licenses(), &util::LICENSES);
            assert_eq!(package.get_vcs_url(), util::VCS_URL);
            assert_eq!(package.get_maturity(), &PackageMaturity::Development);
            assert_eq!(package.get_created().unwrap().date(), Utc::now().date());

            let package = client
                .subject(util::SUBJECT)
                .repository(util::PREEXISTING_REPO)
                .package(&package_name)
                .get().unwrap();

            assert_eq!(package.get_subject(), util::SUBJECT);
            assert_eq!(package.get_repository(), util::PREEXISTING_REPO);
            assert_eq!(package.get_name(), &package_name);
            assert_eq!(package.get_desc(), desc);
            assert_eq!(package.get_labels(), &labels);
            assert_eq!(package.get_licenses(), &util::LICENSES);
            assert_eq!(package.get_vcs_url(), util::VCS_URL);
            assert_eq!(package.get_maturity(), &PackageMaturity::Development);
            assert_eq!(package.get_created().unwrap().date(), Utc::now().date());

            package.delete().unwrap();

            client
                .subject(util::SUBJECT)
                .repository(util::PREEXISTING_REPO)
                .package(&package_name)
                .get()
                // TODO: Replace this with a test that it's the correct
                // exception.
                .expect_err("Package should have been removed");
        }
        None => {
            // Skipped.
        }
    }
}

#[test]
fn list_versions_anonymously() {
    util::init_env_logger();

    let client = Client::new().unwrap();

    let versions = client
        .subject(util::SUBJECT)
        .repository(util::PREEXISTING_REPO)
        .package(util::PREEXISTING_PACKAGE)
        .get().unwrap()
        .versions().unwrap();

    let v1 = String::from(util::PREEXISTING_VERSION_1);
    let v2 = String::from(util::PREEXISTING_VERSION_2);

    assert!(!versions.is_empty());
    assert!(versions.contains(&v1));
    assert!(versions.contains(&v2));

    let i1 = versions.binary_search(&v1).unwrap();
    let i2 = versions.binary_search(&v2).unwrap();
    assert!(i1 < i2);
}
