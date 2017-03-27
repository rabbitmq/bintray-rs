extern crate bintray;
extern crate env_logger;

use bintray::client::BintrayClient;
use bintray::repository::Repository;
use std::env;

#[test]
fn create_update_and_delete_repository() {
    let username = env::var("BINTRAY_USERNAME").ok();
    let api_key = env::var("BINTRAY_API_KEY").ok();

    let owner = env::var("BINTRAY_OWNER")
        .unwrap_or(String::from("my-company"));
    let repository_name =
        "t-bintray-crate-integration-1";

    let description = "Created from Bintray crate testsuite";
    let labels = vec![String::from("label-1"), String::from("label-2")];

    let client = BintrayClient::new(username, api_key);

    {
        let mut repository = Repository::new(&owner, &repository_name);
        assert!(repository.delete(&client).is_ok());
        assert!(repository.created.is_none());

        assert!(!repository.exists(&client).unwrap());
        assert!(repository.created.is_none());
        assert!(repository.desc.is_none());
        assert!(repository.labels.is_none());

        repository.desc = Some(String::from(description));
        assert!(repository.create(&client).is_ok());
        assert!(repository.created.is_some());
        assert!(repository.desc.is_some());
    }

    {
        let mut repository = Repository::new(&owner, &repository_name);
        assert!(repository.get(&client).is_ok());
        assert!(repository.created.is_some());
        assert!(repository.desc.is_some());
        assert_eq!(
            repository.desc.as_ref().map(String::as_str).unwrap(),
            description);
        assert!(
            repository.labels.is_none() ||
            repository.labels.as_ref().unwrap().is_empty());

        repository.labels = Some(labels.clone());
        assert!(repository.update(&client).is_ok());
    }

    {
        let mut repository = Repository::new(&owner, &repository_name);
        assert!(repository.get(&client).is_ok());
        assert!(repository.created.is_some());
        assert!(repository.desc.is_some());
        assert_eq!(
            repository.desc.as_ref().map(String::as_str).unwrap(),
            description);
        assert!(repository.labels.is_some());
        assert_eq!(
            repository.labels.as_ref().unwrap(),
            &labels);

        assert!(repository.delete(&client).is_ok());
    }

    {
        let mut repository = Repository::new(&owner, &repository_name);
        assert!(repository.created.is_none());

        assert!(!repository.exists(&client).unwrap());
        assert!(repository.created.is_none());
        assert!(repository.desc.is_none());
        assert!(repository.labels.is_none());
    }
}
