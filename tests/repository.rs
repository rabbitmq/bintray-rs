extern crate bintray;
extern crate chrono;
extern crate env_logger;

use chrono::Utc;
use bintray::{Client, RepositoryType};

#[allow(dead_code)]
mod util;

#[test]
fn get_repository_anonymously() {
    util::init_env_logger();

    let client = Client::new().unwrap();

    let repository = client
        .subject(util::SUBJECT)
        .repository(util::PREEXISTING_REPO)
        .get().unwrap();

    assert_eq!(repository.get_subject(), util::SUBJECT);
    assert_eq!(repository.get_name(), util::PREEXISTING_REPO);
    assert_eq!(repository.get_type(), &RepositoryType::Generic);
    assert_eq!(repository.is_private(), false);
    assert!(repository.get_desc().contains("expected by the testsuite"));
}

#[test]
#[should_panic(expected = "This resource requires authentication")]
fn create_repository_anonymously() {
    util::init_env_logger();

    let client = Client::new().unwrap();

    let repository_name = util::random_name(16);

    client
        .subject(util::SUBJECT)
        .repository(&repository_name)
        .create().unwrap();
}

#[test]
fn create_and_delete_repository_as_authenticated_user() {
    util::init_env_logger();

    match util::get_credentials() {
        Some((username, api_key)) => {
            let client = Client::new().unwrap()
                .user(&username, &api_key);

            let repository_name = util::random_name(16);

            assert!(!client
                    .subject(util::SUBJECT)
                    .repository(&repository_name)
                    .exists().unwrap());

            let repository = client
                .subject(util::SUBJECT)
                .repository(&repository_name)
                .repo_type(&RepositoryType::Debian)
                .create().unwrap();

            assert_eq!(repository.get_subject(), util::SUBJECT);
            assert_eq!(repository.get_name(), &repository_name);
            assert_eq!(repository.get_type(), &RepositoryType::Debian);
            assert_eq!(repository.is_private(), false);
            assert_eq!(repository.is_premium(), false);
            assert_eq!(repository.get_desc(), "");
            assert_eq!(repository.get_labels(), &vec![] as &Vec<String>);
            assert_eq!(repository.get_created().unwrap().date(), Utc::now().date());

            assert!(client
                    .subject(util::SUBJECT)
                    .repository(&repository_name)
                    .exists().unwrap());

            let mut repository = client
                .subject(util::SUBJECT)
                .repository(&repository_name)
                .get().unwrap();

            assert_eq!(repository.get_subject(), util::SUBJECT);
            assert_eq!(repository.get_name(), &repository_name);
            assert_eq!(repository.get_type(), &RepositoryType::Debian);
            assert_eq!(repository.is_private(), false);
            assert_eq!(repository.is_premium(), false);
            assert_eq!(repository.get_desc(), "");
            assert_eq!(repository.get_labels(), &vec![] as &Vec<String>);
            assert_eq!(repository.get_created().unwrap().date(), Utc::now().date());

            let desc = "Temporary repository created from a testsuite";
            let mut labels = ["testing", "rust"];
            labels.sort();

            repository
                .set_desc(desc)
                .set_labels(&labels)
                .update().unwrap();

            assert_eq!(repository.get_subject(), util::SUBJECT);
            assert_eq!(repository.get_name(), &repository_name);
            assert_eq!(repository.get_type(), &RepositoryType::Debian);
            assert_eq!(repository.is_private(), false);
            assert_eq!(repository.is_premium(), false);
            assert_eq!(repository.get_desc(), desc);
            assert_eq!(repository.get_labels(), &labels);
            assert_eq!(repository.get_created().unwrap().date(), Utc::now().date());

            let repository = client
                .subject(util::SUBJECT)
                .repository(&repository_name)
                .get().unwrap();

            assert_eq!(repository.get_subject(), util::SUBJECT);
            assert_eq!(repository.get_name(), &repository_name);
            assert_eq!(repository.get_type(), &RepositoryType::Debian);
            assert_eq!(repository.is_private(), false);
            assert_eq!(repository.is_premium(), false);
            assert_eq!(repository.get_desc(), desc);
            assert_eq!(repository.get_labels(), &labels);
            assert_eq!(repository.get_created().unwrap().date(), Utc::now().date());

            repository.delete().unwrap();

            client
                .subject(util::SUBJECT)
                .repository(&repository_name)
                .get()
                // TODO: Replace this with a test that it's the correct
                // exception.
                .expect_err("Repository should have been removed");
        }
        None => {
            // Skipped.
        }
    }
}

#[test]
fn list_packages_anonymously() {
    util::init_env_logger();

    let client = Client::new().unwrap();

    let package_names = client
        .subject(util::SUBJECT)
        .repository(util::PREEXISTING_REPO)
        .package_names().unwrap();

    assert!(!package_names.is_empty());
    assert!(package_names.contains(&String::from(util::PREEXISTING_PACKAGE)));
}
