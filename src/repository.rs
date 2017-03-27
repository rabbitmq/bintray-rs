//! Bintray repository
//!
//! The following operations are currently supported on a Bintray
//! repository:
//! * querying a repository;
//! * creating a repository;
//! * updating a repository.

use hyper::status::StatusCode;
use serde_json;
use std::fmt;
use std::io::{self, Read};

use client::{BintrayClient, BintrayError};
use utils;

/// Representation of a repository's attributes.
///
/// They are all the attributes reported by Bintray's API. Some of them
/// can be updated.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repository {
    /// The user or organization owning the repository.
    pub owner: String,
    /// the name of the repository.
    #[serde(rename = "name")]
    pub repository: String,
    /// The type of the repository.
    #[serde(rename = "type")]
    pub type_: RepositoryType,
    /// Flag to indicate if the account is premium.
    #[serde(skip_serializing_if="Option::is_none")]
    pub premium: bool,
    /// The date and time when the repository was created.
    #[serde(skip_serializing_if="Option::is_none")]
    pub created: Option<String>,
    /// The number of packages provided by this repository.
    #[serde(skip_serializing_if="Option::is_none")]
    pub package_count: u64,

    /// Flag to indicate if a repository is private.
    #[serde(skip_serializing_if="Option::is_none")]
    pub private: bool,
    /// Name of the business unit associated with the repository.
    ///
    /// This allows you to monitor overall usage pers business unit.
    #[serde(skip_serializing_if="Option::is_none")]
    pub business_unit: Option<String>,
    /// Description of this repository.
    #[serde(skip_serializing_if="Option::is_none")]
    pub desc: Option<String>,
    /// List of labels attached to this repository.
    #[serde(skip_serializing_if="Option::is_none")]
    pub labels: Option<Vec<String>>,
    /// Flag to indicate if repository metadata are automatically signed
    /// with Bintray GPG key.
    #[serde(skip_serializing_if="Option::is_none")]
    pub gpg_sign_metadata: bool,
    /// Flag to indeicate if repository files are automatically signed
    /// with Bintray GPG key.
    #[serde(skip_serializing_if="Option::is_none")]
    pub gpg_sign_files: bool,
    /// Flag to indicate if repository metadata and files are
    /// automatically signed with the owner's GPG key.
    #[serde(skip_serializing_if="Option::is_none")]
    pub gpg_use_owner_key: bool,

    /// Default Debian architecture to set on uploaded files.
    #[serde(skip_serializing_if="Option::is_none")]
    pub default_debian_architecture: Option<String>,
    /// Default Debian distribution to set on uploaded files.
    #[serde(skip_serializing_if="Option::is_none")]
    pub default_debian_distribution: Option<String>,
    /// Default Debian component to set on uploaded files.
    #[serde(skip_serializing_if="Option::is_none")]
    pub default_debian_component: Option<String>,

    /// Depth, relative to the repository root, at which YUM metadata is
    /// created.
    #[serde(skip_serializing_if="Option::is_none")]
    pub yum_metadata_depth: Option<u64>,
    /// Name of the file holding YUM groups data.
    #[serde(skip_serializing_if="Option::is_none")]
    pub yum_groups_file: Option<String>,
}

/// Repository types supported by Bintray.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all="snake_case")]
pub enum RepositoryType {
    /// Debian repository.
    Debian,
    /// Docker images repository.
    Docker,
    /// Generic file storage.
    Generic,
    /// Maven repository.
    Maven,
    /// Node.js/npm repository.
    Npm,
    /// NuGet repository.
    Nuget,
    /// OpenWRT repository.
    Opkg,
    /// RPM repository.
    Rpm,
    /// Vagrant boxes repository.
    Vagrant,

    /// Bintray API sometimes report Debian repository type as `deb` instead of `debian`.
    ///
    /// The module tries to convert `Deb` to `Debian`.
    // Looks like Bintray returns inconsistent types for Debian
    // repository.
    Deb,
}

impl Repository {
    /// Instanciates a new `Repository` structure.
    ///
    /// The `Repository` type defaults to `Repository::Generic`.
    pub fn new(owner: &str, repository: &str) -> Repository {
        Repository {
            repository: String::from(repository),
            owner: String::from(owner),
            type_: RepositoryType::Generic,
            premium: false,
            created: None,
            package_count: 0,

            private: false,
            business_unit: None,
            desc: None,
            labels: None,
            gpg_sign_metadata: false,
            gpg_sign_files: false,
            gpg_use_owner_key: false,

            default_debian_architecture: None,
            default_debian_distribution: None,
            default_debian_component: None,

            yum_metadata_depth: None,
            yum_groups_file: None,
        }
    }

    /// Queries Bintray API and update `Repository` structure with the
    /// returned attributes.
    ///
    /// # Examples
    ///
    /// Query an existing RPM repository:
    ///
    /// ```rust
    /// use bintray::client::BintrayClient;
    /// use bintray::repository::{Repository, RepositoryType};
    /// # use std::env;
    /// # let username = env::var("BINTRAY_USERNAME").ok();
    /// # let api_key = env::var("BINTRAY_API_KEY").ok();
    /// # let owner = env::var("BINTRAY_OWNER")
    /// #     .unwrap_or(String::from("my-company"));
    /// # let repository_name =
    /// #     "t-bintray-crate-repository-1";
    ///
    /// // Initialize a Bintray client with a username and an API key.
    /// // Those are `Option` because some operations can be performed
    /// // anonymously.
    /// let client = BintrayClient::new(username, api_key);
    ///
    /// # {
    /// #   let mut repository = Repository::new(&owner, &repository_name);
    /// #   repository.type_ = RepositoryType::Rpm;
    /// #   assert!(repository.delete(&client).is_ok());
    /// #   assert!(repository.create(&client).is_ok());
    /// # }
    /// // Create a `Repository` structure. At this time, the structure
    /// // is empty: nothing was requested from Bintray yet. That's why
    /// // the creation date is empty for instance.
    /// let mut repository = Repository::new(&owner, &repository_name);
    /// assert!(repository.created.is_none());
    ///
    /// // Query Bintray for informations about this repository. This
    /// // time, the creation date is available. If the repository
    /// // didn't exist, the call would have returned an error.
    /// assert!(repository.get(&client).is_ok());
    /// assert!(repository.created.is_some());
    /// assert_eq!(repository.type_, RepositoryType::Rpm);
    /// # assert!(repository.delete(&client).is_ok());
    /// ```
    pub fn get(&mut self, client: &BintrayClient)
        -> Result<(), BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.push("repos");
            path.push(&self.owner);
            path.push(&self.repository);
        }

        let mut resp = client.get(url).send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("GetRepository({}): {}", self, body);

                let queried: Repository = serde_json::from_str(&body)?;

                self.type_ = queried.type_;
                self.premium = queried.premium;
                self.created = queried.created;
                self.package_count = queried.package_count;

                self.private = queried.private;
                self.business_unit = queried.business_unit;
                self.desc = queried.desc;
                self.labels = queried.labels
                    .map(|mut v| { v.sort(); v });
                self.gpg_sign_metadata = queried.gpg_sign_metadata;
                self.gpg_sign_files = queried.gpg_sign_files;
                self.gpg_use_owner_key = queried.gpg_use_owner_key;

                self.default_debian_architecture =
                    queried.default_debian_architecture;
                self.default_debian_distribution =
                    queried.default_debian_distribution;
                self.default_debian_component =
                    queried.default_debian_component;

                self.yum_metadata_depth = queried.yum_metadata_depth;
                self.yum_groups_file = queried.yum_groups_file;

                if self.type_ == RepositoryType::Deb {
                    self.type_ = RepositoryType::Debian;
                }

                Ok(())
            }
            _ if resp.status == StatusCode::NotFound => {
                report_bintray_error!(
                    self, resp, body, "GetRepository",
                    io::ErrorKind::NotFound,
                    "Repository not found", true)
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "GetRepository",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "GetRepository",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    /// Same as `get()` but returns true if the repository exists, or
    /// false if it doesn't.
    ///
    /// Only the "404 Not Found" error is accepted and false is
    /// returned. Other errors are returned like `get()`.
    ///
    /// # Examples
    ///
    /// Create an RPM repository if it doesn't exist:
    ///
    /// ```rust
    /// use bintray::client::BintrayClient;
    /// use bintray::repository::{Repository, RepositoryType};
    /// # use std::env;
    /// # let username = env::var("BINTRAY_USERNAME").ok();
    /// # let api_key = env::var("BINTRAY_API_KEY").ok();
    /// # let owner = env::var("BINTRAY_OWNER")
    /// #     .unwrap_or(String::from("my-company"));
    /// # let repository_name =
    /// #     "t-bintray-crate-repository-2";
    ///
    /// // Initialize a Bintray client with a username and an API key.
    /// // Those are `Option` because some operations can be performed
    /// // anonymously.
    /// let client = BintrayClient::new(username, api_key);
    ///
    /// # {
    /// #   let mut repository = Repository::new(&owner, &repository_name);
    /// #   assert!(repository.delete(&client).is_ok());
    /// # }
    /// // Create a `Repository` structure. At this time, the structure
    /// // is empty: nothing was requested from Bintray yet. That's why
    /// // the creation date is empty for instance.
    /// let mut repository = Repository::new(&owner, &repository_name);
    ///
    /// // Test if the repository exists, and create if it doesn't.
    /// if !repository.exists(&client).unwrap() {
    ///     repository.desc = Some(String::from("My RPM repository"));
    ///     repository.type_ = RepositoryType::Rpm;
    ///     assert!(repository.create(&client).is_ok());
    /// }
    ///
    /// // We can query it again to double-check the type is correct.
    /// assert!(repository.get(&client).is_ok());
    /// assert!(repository.created.is_some());
    /// assert_eq!(repository.type_, RepositoryType::Rpm);
    /// # assert!(repository.delete(&client).is_ok());
    /// ```
    pub fn exists(&mut self, client: &BintrayClient)
        -> Result<bool, BintrayError>
    {
        match self.get(client) {
            Ok(()) => Ok(true),
            Err(BintrayError::Io(ref e))
                if e.kind() == io::ErrorKind::NotFound => {
                    Ok(false)
                }
            Err(e) => Err(e),
        }
    }

    /// Creates the repository.
    ///
    /// The `Repository` structure members should be filled first. They
    /// are then passed to the API as is. Fields set to `None` are not
    /// submitted to the API, the remote value will be the default.
    ///
    /// # Examples
    ///
    /// Create a Debian repository:
    ///
    /// ```rust
    /// use bintray::client::BintrayClient;
    /// use bintray::repository::{Repository, RepositoryType};
    /// # use std::env;
    /// # let username = env::var("BINTRAY_USERNAME").ok();
    /// # let api_key = env::var("BINTRAY_API_KEY").ok();
    /// # let owner = env::var("BINTRAY_OWNER")
    /// #     .unwrap_or(String::from("my-company"));
    /// # let repository_name =
    /// #     "t-bintray-crate-repository-3";
    ///
    /// // Initialize a Bintray client with a username and an API key.
    /// // Those are `Option` because some operations can be performed
    /// // anonymously.
    /// let client = BintrayClient::new(username, api_key);
    ///
    /// # {
    /// #   let mut repository = Repository::new(&owner, &repository_name);
    /// #   assert!(repository.delete(&client).is_ok());
    /// # }
    /// // Create a `Repository` structure and initialize several
    /// // fields. In particular the type is important, though it
    /// // defaults to `RepositoryType::Generic`.
    /// let mut repository = Repository::new(&owner, &repository_name);
    /// repository.desc = Some(String::from("My RPM repository"));
    /// repository.type_ = RepositoryType::Debian;
    /// assert!(repository.create(&client).is_ok());
    ///
    /// // We can query it again to double-check the type is correct.
    /// assert!(repository.get(&client).is_ok());
    /// assert!(repository.created.is_some());
    /// assert_eq!(repository.type_, RepositoryType::Debian);
    /// # assert!(repository.delete(&client).is_ok());
    /// ```
    pub fn create(&mut self, client: &BintrayClient)
        -> Result<(), BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.push("repos");
            path.push(&self.owner);
            path.push(&self.repository);
        }

        #[derive(Serialize)]
        struct CreateRepositoryReq {
            name: String,
            #[serde(rename = "type")]
            type_: RepositoryType,
            private: bool,
            #[serde(skip_serializing_if="Option::is_none")]
            business_unit: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            desc: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            labels: Option<Vec<String>>,
            gpg_sign_metadata: bool,
            gpg_sign_files: bool,
            gpg_use_owner_key: bool,

            #[serde(skip_serializing_if="Option::is_none")]
            default_debian_architecture: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            default_debian_distribution: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            default_debian_component: Option<String>,

            // RPM-specific properties.
            #[serde(skip_serializing_if="Option::is_none")]
            yum_metadata_depth: Option<u64>,
            #[serde(skip_serializing_if="Option::is_none")]
            yum_groups_file: Option<String>,
        };

        let args = CreateRepositoryReq {
            name: self.repository.clone(),
            type_: self.type_.clone(),

            private: self.private,
            business_unit: self.business_unit.clone(),
            desc: self.desc.clone(),
            labels: self.labels.clone(),
            gpg_sign_metadata: self.gpg_sign_metadata,
            gpg_sign_files: self.gpg_sign_files,
            gpg_use_owner_key: self.gpg_use_owner_key,

            default_debian_architecture:
                self.default_debian_architecture.clone(),
            default_debian_distribution:
                self.default_debian_distribution.clone(),
            default_debian_component:
                self.default_debian_component.clone(),

            yum_metadata_depth: self.yum_metadata_depth,
            yum_groups_file: self.yum_groups_file.clone(),
        };

        let json = serde_json::to_string_pretty(&args)?;
        info!(
            "CreateRepository({}): Submitting the following properties:\n{}",
            self, json);

        let mut resp = client.post(url)
            .body(&json)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Created => {
                info!("CreateRepository({}): {}", self, body);

                let created: Repository = serde_json::from_str(&body)?;
                self.created = created.created;
                // TODO: Assert that created == repo. */

                Ok(())
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "CreateRepository",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "CreateRepository",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "CreateRepository",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    /// Updates the repository attributes.
    ///
    /// The `Repository` structure members should be filled first. They
    /// are then passed to the API as is. Fields set to `None` are not
    /// submitted to the API, the remote value will thus remain the
    /// same.
    ///
    /// # Examples
    ///
    /// Set a description and the default distribution of a Debian
    /// repository:
    ///
    /// ```rust
    /// use bintray::client::BintrayClient;
    /// use bintray::repository::{Repository, RepositoryType};
    /// # use std::env;
    /// # let username = env::var("BINTRAY_USERNAME").ok();
    /// # let api_key = env::var("BINTRAY_API_KEY").ok();
    /// # let owner = env::var("BINTRAY_OWNER")
    /// #     .unwrap_or(String::from("my-company"));
    /// # let repository_name =
    /// #     "t-bintray-crate-repository-4";
    ///
    /// // Initialize a Bintray client with a username and an API key.
    /// // Those are `Option` because some operations can be performed
    /// // anonymously.
    /// let client = BintrayClient::new(username, api_key);
    ///
    /// # {
    /// #   let mut repository = Repository::new(&owner, &repository_name);
    /// #   repository.type_ = RepositoryType::Debian;
    /// #   assert!(repository.delete(&client).is_ok());
    /// #   assert!(repository.create(&client).is_ok());
    /// # }
    /// let mut repository = Repository::new(&owner, &repository_name);
    /// assert!(repository.get(&client).is_ok());
    ///
    /// // Set properties and update the repository.
    /// repository.desc = Some(String::from("My repository"));
    /// repository.default_debian_distribution = Some(String::from("jessie"));
    /// assert!(repository.update(&client).is_ok());
    /// # assert!(repository.delete(&client).is_ok());
    /// ```
    pub fn update(&self, client: &BintrayClient)
        -> Result<(), BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.push("repos");
            path.push(&self.owner);
            path.push(&self.repository);
        }

        #[derive(Serialize)]
        struct UpdateRepositoryReq {
            #[serde(skip_serializing_if="Option::is_none")]
            business_unit: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            desc: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            labels: Option<Vec<String>>,
            gpg_sign_metadata: bool,
            gpg_sign_files: bool,
            gpg_use_owner_key: bool,
        };

        let args = UpdateRepositoryReq {
            business_unit: self.business_unit.clone(),
            desc: self.desc.clone(),
            labels: self.labels.clone(),
            gpg_sign_metadata: self.gpg_sign_metadata,
            gpg_sign_files: self.gpg_sign_files,
            gpg_use_owner_key: self.gpg_use_owner_key,
        };

        let json = serde_json::to_string_pretty(&args)?;
        info!(
            "UpdateRepository({}): Submitting the following properties:\n{}",
            self, json);

        let mut resp = client.patch(url)
            .body(&json)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("UpdateRepository({}): {}", self, body);
                Ok(())
            }
            _ if resp.status == StatusCode::NotFound => {
                report_bintray_error!(
                    self, resp, body, "UpdateRepository",
                    io::ErrorKind::NotFound,
                    "Repository not found")
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "UpdateRepository",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "UpdateRepository",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "UpdateRepository",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    /// Deletes a repository.
    ///
    /// The `Repository` structure members may be left uninitialized.
    ///
    /// # Examples
    ///
    /// Delete a repository:
    ///
    /// ```rust
    /// use bintray::client::BintrayClient;
    /// use bintray::repository::Repository;
    /// # use std::env;
    /// # let username = env::var("BINTRAY_USERNAME").ok();
    /// # let api_key = env::var("BINTRAY_API_KEY").ok();
    /// # let owner = env::var("BINTRAY_OWNER")
    /// #     .unwrap_or(String::from("my-company"));
    /// # let repository_name =
    /// #     "t-bintray-crate-repository-5";
    ///
    /// // Initialize a Bintray client with a username and an API key.
    /// // Those are `Option` because some operations can be performed
    /// // anonymously.
    /// let client = BintrayClient::new(username, api_key);
    ///
    /// # {
    /// #   let mut repository = Repository::new(&owner, &repository_name);
    /// #   assert!(repository.delete(&client).is_ok());
    /// #   assert!(repository.create(&client).is_ok());
    /// # }
    /// // Initialize the `Repository` structure and delete the
    /// // repository named `repository_name`.
    /// let mut repository = Repository::new(&owner, &repository_name);
    /// assert!(repository.delete(&client).is_ok());
    ///
    /// // We can double-check that the repository is gone.
    /// assert!(!repository.exists(&client).unwrap());
    /// ```
    pub fn delete(&mut self, client: &BintrayClient)
        -> Result<(), BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.push("repos");
            path.push(&self.owner);
            path.push(&self.repository);
        }

        let mut resp = client.delete(url)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("DeleteRepository({}): {}", self, body);

                self.created = None;

                Ok(())
            }
            _ if resp.status == StatusCode::NotFound => {
                info!("DeleteRepository({}): {}", self, body);

                self.created = None;

                Ok(())
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "DeleteRepository",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "DeleteRepository",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "DeleteRepository",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    /// Return the list of packages provided by this repository.
    ///
    /// # Examples
    ///
    /// List packages provided by an RPM repository:
    ///
    /// ```rust
    /// use bintray::client::BintrayClient;
    /// use bintray::repository::{Repository, RepositoryType};
    /// use bintray::package::Package;
    /// # use std::env;
    /// # let username = env::var("BINTRAY_USERNAME").ok();
    /// # let api_key = env::var("BINTRAY_API_KEY").ok();
    /// # let owner = env::var("BINTRAY_OWNER")
    /// #     .unwrap_or(String::from("my-company"));
    /// # let repository_name =
    /// #     "t-bintray-crate-repository-6";
    ///
    /// // Initialize a Bintray client with a username and an API key.
    /// // Those are `Option` because some operations can be performed
    /// // anonymously.
    /// let client = BintrayClient::new(username, api_key);
    ///
    /// # {
    /// #   let mut repository = Repository::new(&owner, &repository_name);
    /// #   repository.type_ = RepositoryType::Rpm;
    /// #   assert!(repository.delete(&client).is_ok());
    /// #   assert!(repository.create(&client).is_ok());
    /// # }
    /// let mut repository = Repository::new(&owner, &repository_name);
    ///
    /// // Create a package in this repository.
    /// let mut package = Package::new(
    ///     &repository.owner,
    ///     &repository.repository,
    ///     "my-package")
    ///     .set_licenses(&["BSD 2-Clause"])
    ///     .set_vcs_url("https://github.com/my-company/my-project.git");
    /// assert!(package.create(&client).is_ok());
    ///
    /// // Get the list of packages for this repository.
    /// let packages = repository.list_packages(&client).unwrap();
    /// assert_eq!(packages.len(), 1);
    /// # assert!(repository.delete(&client).is_ok());
    /// ```
    pub fn list_packages(&self, client: &BintrayClient)
        -> Result<Vec<String>, BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.push("repos");
            path.push(&self.owner);
            path.push(&self.repository);
            path.push("packages");
        }

        #[derive(Deserialize)]
        struct GetPackagesResp {
            name: String,
            // linked: bool,
        };

        let mut resp = client.get(url).send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("GetPackages({}): {}", self, body);

                let result: Vec<GetPackagesResp> =
                    serde_json::from_str(&body)?;

                let packages: Vec<String> = result.into_iter()
                    .map(|item| item.name)
                    .collect();
                Ok(packages)
            }
            _ if resp.status == StatusCode::NotFound => {
                report_bintray_error!(
                    self, resp, body, "GetPackages",
                    io::ErrorKind::NotFound,
                    "Repository not found")
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "GetPackages",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "GetPackages",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }
}

impl fmt::Display for Repository {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}<{}>", self.owner, self.repository, self.type_)
    }
}

impl fmt::Display for RepositoryType {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        let type_ = match self {
            &RepositoryType::Debian => "Debian",
            &RepositoryType::Docker => "Docker",
            &RepositoryType::Generic => "Generic",
            &RepositoryType::Maven => "Maven",
            &RepositoryType::Npm => "npm",
            &RepositoryType::Nuget => "NuGet",
            &RepositoryType::Opkg => "opkg",
            &RepositoryType::Rpm => "RPM",
            &RepositoryType::Vagrant => "Vagrant",

            &RepositoryType::Deb => "Deb",
        };
        write!(f, "{}", type_)
    }
}
