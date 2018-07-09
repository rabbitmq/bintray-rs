extern crate bintray;
extern crate chrono;
extern crate env_logger;

use chrono::Utc;
use bintray::Client;

#[allow(dead_code)]
mod util;

#[test]
#[should_panic(expected = "This resource requires authentication")]
fn get_version_anonymously() {
    util::init_env_logger();

    let client = Client::new().unwrap();

    client
        .subject(util::SUBJECT)
        .repository(util::PREEXISTING_REPO)
        .package(util::PREEXISTING_PACKAGE)
        .version(util::PREEXISTING_VERSION_1)
        .get().unwrap();
}

#[test]
#[should_panic(expected = "This resource requires authentication")]
fn create_version_anonymously() {
    util::init_env_logger();

    let client = Client::new().unwrap();

    let version_string = util::random_name(8);

    client
        .subject(util::SUBJECT)
        .repository(util::PREEXISTING_REPO)
        .package(util::PREEXISTING_PACKAGE)
        .version(&version_string)
        .create().unwrap();
}

#[test]
fn create_and_delete_version_as_authenticated_user() {
    util::init_env_logger();

    match util::get_credentials() {
        Some((username, api_key)) => {
            let client = Client::new().unwrap()
                .user(&username, &api_key);

            let version_string = util::random_name(16);

            assert!(!client
                    .subject(util::SUBJECT)
                    .repository(util::PREEXISTING_REPO)
                    .package(util::PREEXISTING_PACKAGE)
                    .version(&version_string)
                    .exists().unwrap());

            let version = client
                .subject(util::SUBJECT)
                .repository(util::PREEXISTING_REPO)
                .package(util::PREEXISTING_PACKAGE)
                .version(&version_string)
                .desc("")
                .create().unwrap();

            assert_eq!(version.get_subject(), util::SUBJECT);
            assert_eq!(version.get_repository(), util::PREEXISTING_REPO);
            assert_eq!(version.get_package(), util::PREEXISTING_PACKAGE);
            assert_eq!(version.get_version(), &version_string);
            assert_eq!(version.get_desc(), "");
            assert_eq!(version.get_created().unwrap().date(), Utc::now().date());

            assert!(client
                    .subject(util::SUBJECT)
                    .repository(util::PREEXISTING_REPO)
                    .package(util::PREEXISTING_PACKAGE)
                    .version(&version_string)
                    .exists().unwrap());

            let mut version = client
                .subject(util::SUBJECT)
                .repository(util::PREEXISTING_REPO)
                .package(util::PREEXISTING_PACKAGE)
                .version(&version_string)
                .get().unwrap();

            assert_eq!(version.get_subject(), util::SUBJECT);
            assert_eq!(version.get_repository(), util::PREEXISTING_REPO);
            assert_eq!(version.get_package(), util::PREEXISTING_PACKAGE);
            assert_eq!(version.get_version(), &version_string);
            assert_eq!(version.get_desc(), "");
            assert_eq!(version.get_created().unwrap().date(), Utc::now().date());

            let desc = "Temporary version created from a testsuite";

            version
                .set_desc(desc)
                .update().unwrap();

            assert_eq!(version.get_subject(), util::SUBJECT);
            assert_eq!(version.get_repository(), util::PREEXISTING_REPO);
            assert_eq!(version.get_package(), util::PREEXISTING_PACKAGE);
            assert_eq!(version.get_version(), &version_string);
            assert_eq!(version.get_desc(), desc);
            assert_eq!(version.get_created().unwrap().date(), Utc::now().date());

            let version = client
                .subject(util::SUBJECT)
                .repository(util::PREEXISTING_REPO)
                .package(util::PREEXISTING_PACKAGE)
                .version(&version_string)
                .get().unwrap();

            assert_eq!(version.get_subject(), util::SUBJECT);
            assert_eq!(version.get_repository(), util::PREEXISTING_REPO);
            assert_eq!(version.get_package(), util::PREEXISTING_PACKAGE);
            assert_eq!(version.get_version(), &version_string);
            assert_eq!(version.get_desc(), desc);
            assert_eq!(version.get_created().unwrap().date(), Utc::now().date());

            version.delete().unwrap();

            client
                .subject(util::SUBJECT)
                .repository(util::PREEXISTING_REPO)
                .package(util::PREEXISTING_PACKAGE)
                .version(&version_string)
                .get()
                // TODO: Replace this with a test that it's the correct
                // exception.
                .expect_err("Version should have been removed");
        }
        None => {
            // Skipped.
        }
    }
}
