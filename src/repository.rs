use chrono::{DateTime, Utc};
use failure::Error;
use reqwest::StatusCode;
use std::fmt;
use std::path::Path;
use ::{Client, BintrayError, Package};

use std::iter::Map;
use std::vec::IntoIter;

#[derive(Clone, Debug)]
pub struct Repository {
    subject: String,
    repository: String,
    type_: RepositoryType,

    /* Generic properties. */
    private: bool,
    premium: bool,
    desc: String,
    labels: Vec<String>,
    created: Option<DateTime<Utc>>,
    package_count: Option<u64>,
    gpg_use_owner_key: bool,
    gpg_sign_files: bool,
    gpg_sign_metadata: bool,

    /* Debian-specific properties. */
    default_debian_architecture: Option<String>,
    default_debian_distribution: Option<String>,
    default_debian_component: Option<String>,

    /* RPM-specific properties. */
    yum_metadata_depth: Option<usize>,
    yum_groups_file: Option<String>,

    client: Client,
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

    /// Bintray API sometimes reports Debian repository type as `deb`
    /// instead of `debian`.
    ///
    /// This module enforces then use of `Debian`, converting `Deb`
    /// internally when Bintray API responds with it.
    Deb,
}

#[derive(Deserialize)]
struct PackageNamesListEntry {
    name: String,
}

impl RepositoryType {
    pub fn is_indexed(&self) -> bool
    {
        match self {
            &RepositoryType::Debian => true,
            &RepositoryType::Rpm    => true,
            _                       => false,
        }
    }

    pub fn fixup(self) -> RepositoryType
    {
        match self {
            RepositoryType::Deb => RepositoryType::Debian,
            _                   => self,
        }
    }
}

impl Repository {
    pub fn new(client: &Client,
               subject: &str,
               repository: &str)
        -> Self
    {
        Repository {
            subject: String::from(subject),
            repository: String::from(repository),
            type_: RepositoryType::Generic,

            /* Generic properties. */
            private: false,
            premium: false,
            desc: String::new(),
            labels: vec![],
            created: None,
            package_count: None,
            gpg_use_owner_key: false,
            gpg_sign_files: false,
            gpg_sign_metadata: false,

            /* Debian-specific properties. */
            default_debian_architecture: None,
            default_debian_distribution: None,
            default_debian_component: None,

            /* RPM-specific properties. */
            yum_metadata_depth: None,
            yum_groups_file: None,

            client: client.clone(),
        }
    }

    pub fn repo_type(mut self, type_: &RepositoryType) -> Self
    {
        self.type_ = (*type_).clone().fixup();
        self
    }

    pub fn private(mut self, private: bool) -> Self
    {
        self.set_private(private);
        self
    }

    pub fn set_private(&mut self, private: bool) -> &mut Self
    {
        self.private = private;
        self
    }

    pub fn desc(mut self, desc: &str) -> Self
    {
        self.set_desc(desc);
        self
    }

    pub fn set_desc(&mut self, desc: &str) -> &mut Self
    {
        self.desc = String::from(desc);
        self
    }

    pub fn labels<T: AsRef<str>>(mut self, labels: &[T]) -> Self
    {
        self.set_labels(labels);
        self
    }

    pub fn set_labels<T: AsRef<str>>(&mut self, labels: &[T]) -> &mut Self
    {
        let mut vec: Vec<String> = labels
            .iter()
            .map(|s| s.as_ref().to_owned())
            .collect();
        vec.sort();

        self.labels = vec;
        self
    }

    pub fn gpg_use_owner_key(mut self, gpg_use_owner_key: bool) -> Self
    {
        self.set_gpg_use_owner_key(gpg_use_owner_key);
        self
    }

    pub fn set_gpg_use_owner_key(&mut self, gpg_use_owner_key: bool)
        -> &mut Self
    {
        self.gpg_use_owner_key = gpg_use_owner_key;
        self
    }

    pub fn gpg_sign_files(mut self, gpg_sign_files: bool) -> Self
    {
        self.set_gpg_sign_files(gpg_sign_files);
        self
    }

    pub fn set_gpg_sign_files(&mut self, gpg_sign_files: bool) -> &mut Self
    {
        self.gpg_sign_files = gpg_sign_files;
        self
    }

    pub fn gpg_sign_metadata(mut self, gpg_sign_metadata: bool) -> Self
    {
        self.set_gpg_sign_metadata(gpg_sign_metadata);
        self
    }

    pub fn set_gpg_sign_metadata(&mut self, gpg_sign_metadata: bool)
        -> &mut Self
    {
        self.gpg_sign_metadata = gpg_sign_metadata;
        self
    }

    pub fn default_debian_architecture(mut self,
                                       default_debian_architecture: &str)
        -> Self
    {
        self.set_default_debian_architecture(default_debian_architecture);
        self
    }

    pub fn set_default_debian_architecture(&mut self,
                                           default_debian_architecture: &str)
        -> &mut Self
    {
        self.default_debian_architecture =
            Some(String::from(default_debian_architecture));
        self
    }

    pub fn default_debian_distribution(mut self,
                                       default_debian_distribution: &str)
        -> Self
    {
        self.set_default_debian_distribution(default_debian_distribution);
        self
    }

    pub fn set_default_debian_distribution(&mut self,
                                           default_debian_distribution: &str)
        -> &mut Self
    {
        self.default_debian_distribution =
            Some(String::from(default_debian_distribution));
        self
    }

    pub fn default_debian_component(mut self,
                                       default_debian_component: &str)
        -> Self
    {
        self.set_default_debian_component(default_debian_component);
        self
    }

    pub fn set_default_debian_component(&mut self,
                                           default_debian_component: &str)
        -> &mut Self
    {
        self.default_debian_component =
            Some(String::from(default_debian_component));
        self
    }

    pub fn yum_metadata_depth(mut self, yum_metadata_depth: usize) -> Self
    {
        self.set_yum_metadata_depth(yum_metadata_depth);
        self
    }

    pub fn set_yum_metadata_depth(&mut self, yum_metadata_depth: usize)
        -> &mut Self
    {
        self.yum_metadata_depth = Some(yum_metadata_depth);
        self
    }

    pub fn yum_groups_file<P: AsRef<Path>>(mut self, yum_groups_file: P)
        -> Self
    {
        self.set_yum_groups_file(yum_groups_file);
        self
    }

    pub fn set_yum_groups_file<P: AsRef<Path>>(&mut self, yum_groups_file: P)
        -> &mut Self
    {
        let path = Path::new(yum_groups_file.as_ref());
        self.yum_groups_file = Some(path.to_string_lossy().into_owned());
        self
    }

    pub fn create(mut self) -> Result<Self, Error>
    {
        let url = self.client.api_url(
            &format!("/repos/{}/{}",
                     self.subject,
                     self.repository))?;

        #[derive(Serialize)]
        struct CreateRepositoryReq {
            name: String,
            #[serde(rename = "type")]
            type_: RepositoryType,

            /* Generic properties. */
            private: bool,
            desc: String,
            labels: Vec<String>,
            gpg_use_owner_key: bool,
            gpg_sign_metadata: bool,
            gpg_sign_files: bool,

            /* Debian-specific properties. */
            #[serde(skip_serializing_if="Option::is_none")]
            default_debian_architecture: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            default_debian_distribution: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            default_debian_component: Option<String>,

            /* RPM-specific properties. */
            #[serde(skip_serializing_if="Option::is_none")]
            yum_metadata_depth: Option<usize>,
            #[serde(skip_serializing_if="Option::is_none")]
            yum_groups_file: Option<String>,
        }

        let req = CreateRepositoryReq {
            name: self.repository.clone(),
            type_: self.type_.clone(),

            private: self.private,
            desc: self.desc.clone(),
            labels: self.labels.clone(),
            gpg_use_owner_key: self.gpg_use_owner_key,
            gpg_sign_metadata: self.gpg_sign_files,
            gpg_sign_files: self.gpg_sign_metadata,

            default_debian_architecture: self.default_debian_architecture.clone(),
            default_debian_distribution: self.default_debian_distribution.clone(),
            default_debian_component: self.default_debian_component.clone(),

            yum_metadata_depth: self.yum_metadata_depth.clone(),
            yum_groups_file: self.yum_groups_file.clone(),
        };

        let mut response = self.client
            .post(url)
            .json(&req)
            .send()?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct CreateRepositoryResp {
                owner: String,
                name: String,
                #[serde(rename = "type")]
                type_: RepositoryType,

                /* Generic properties. */
                private: bool,
                premium: bool,
                desc: String,
                labels: Vec<String>,
                created: String,
                package_count: u64,
                gpg_use_owner_key: bool,
                gpg_sign_files: bool,
                gpg_sign_metadata: bool,

                /* Debian-specific properties. */
                default_debian_architecture: Option<String>,
                default_debian_distribution: Option<String>,
                default_debian_component: Option<String>,

                /* RPM-specific properties. */
                yum_metadata_depth: Option<usize>,
                yum_groups_file: Option<String>,
            }

            let mut resp: CreateRepositoryResp = response.json()?;
            resp.type_ = resp.type_.fixup();
            resp.labels.sort();

            debug_assert_eq!(self.subject, resp.owner);
            debug_assert_eq!(self.repository, resp.name);
            debug_assert_eq!(self.type_, resp.type_);
            debug_assert_eq!(self.private, resp.private);
            debug_assert_eq!(self.desc, resp.desc);
            debug_assert_eq!(self.labels, resp.labels);
            debug_assert_eq!(self.gpg_use_owner_key,
                             resp.gpg_use_owner_key);
            debug_assert_eq!(self.gpg_sign_files,
                             resp.gpg_sign_files);
            debug_assert_eq!(self.gpg_sign_metadata,
                             resp.gpg_sign_metadata);
            debug_assert_eq!(self.default_debian_architecture,
                             resp.default_debian_architecture);
            debug_assert_eq!(self.default_debian_distribution,
                             resp.default_debian_distribution);
            debug_assert_eq!(self.default_debian_component,
                             resp.default_debian_component);
            debug_assert_eq!(self.yum_metadata_depth, resp.yum_metadata_depth);
            debug_assert_eq!(self.yum_groups_file, resp.yum_groups_file);

            self.premium = resp.premium;
            self.package_count = Some(resp.package_count);
            self.created = resp.created.parse::<DateTime<Utc>>().ok();

            Ok(self)
        } else {
            #[derive(Deserialize)]
            struct CreateRepositoryError {
                message: String,
            }

            let resp: CreateRepositoryError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn exists(&self) -> Result<bool, Error>
    {
        let url = self.client.api_url(
            &format!("/repos/{}/{}",
                     self.subject,
                     self.repository))?;

        let response = self.client
            .head(url)
            .send()?;

        if response.status().is_success() {
            Ok(true)
        } else {
            match response.status() {
                StatusCode::Unauthorized |
                StatusCode::NotFound => {
                    Ok(false)
                }
                status => {
                    throw!(BintrayError::BintrayApiError {
                        message: format!("Unexpected status from Bintray: {}",
                                         status)
                    })
                }
            }
        }
    }

    pub fn get(mut self) -> Result<Self, Error>
    {
        let url = self.client.api_url(
            &format!("/repos/{}/{}",
                     self.subject,
                     self.repository))?;

        let mut response = self.client
            .get(url)
            .send()?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct GetRepositoryResp {
                owner: String,
                name: String,
                #[serde(rename = "type")]
                type_: RepositoryType,

                /* Generic properties. */
                private: bool,
                premium: bool,
                desc: String,
                labels: Vec<String>,
                created: String,
                package_count: u64,
                gpg_use_owner_key: Option<bool>,
                gpg_sign_files: Option<bool>,
                gpg_sign_metadata: Option<bool>,

                /* Debian-specific properties. */
                default_debian_architecture: Option<String>,
                default_debian_distribution: Option<String>,
                default_debian_component: Option<String>,

                /* RPM-specific properties. */
                yum_metadata_depth: Option<usize>,
                yum_groups_file: Option<String>,
            }

            let mut resp: GetRepositoryResp = response.json()?;
            resp.type_ = resp.type_.fixup();
            resp.labels.sort();

            debug_assert_eq!(self.subject, resp.owner);
            debug_assert_eq!(self.repository, resp.name);

            self.type_ = resp.type_;
            self.private = resp.private;
            self.premium = resp.premium;
            self.desc = resp.desc;
            self.labels = resp.labels;
            self.created = resp.created.parse::<DateTime<Utc>>().ok();
            self.package_count = Some(resp.package_count);
            self.gpg_use_owner_key = resp.gpg_use_owner_key.unwrap_or(false);
            self.gpg_sign_files = resp.gpg_sign_files.unwrap_or(false);
            self.gpg_sign_metadata = resp.gpg_sign_metadata.unwrap_or(false);

            self.default_debian_architecture = resp.default_debian_architecture;
            self.default_debian_distribution = resp.default_debian_distribution;
            self.default_debian_component = resp.default_debian_component;

            self.yum_metadata_depth = resp.yum_metadata_depth;
            self.yum_groups_file = resp.yum_groups_file;

            trace!("{}:\n\
                   - private: {}\n\
                   - premium: {}\n\
                   - desc: \"{}\"\n\
                   - labels: {:?}\n\
                   - created: {}\n\
                   - package_count: {}\n\
                   ",
                   self,
                   self.private,
                   self.premium,
                   self.desc,
                   self.labels,
                   self.created
                   .map_or(String::from("(unknown)"), |d| d.to_string()),
                   self.package_count.unwrap_or(0));

            Ok(self)
        } else {
            #[derive(Deserialize)]
            struct GetRepositoryError {
                message: String,
            }

            let resp: GetRepositoryError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn update(&self) -> Result<&Self, Error>
    {
        let url = self.client.api_url(
            &format!("/repos/{}/{}",
                     self.subject,
                     self.repository))?;

        #[derive(Serialize)]
        struct UpdateRepositoryReq {
            desc: String,
            labels: Vec<String>,
            gpg_sign_metadata: bool,
            gpg_sign_files: bool,
            gpg_use_owner_key: bool,
        }

        let req = UpdateRepositoryReq {
            desc: self.desc.clone(),
            labels: self.labels.clone(),
            gpg_use_owner_key: self.gpg_use_owner_key,
            gpg_sign_files: self.gpg_sign_files,
            gpg_sign_metadata: self.gpg_sign_metadata,
        };

        let mut response = self.client
            .patch(url)
            .json(&req)
            .send()?;

        if response.status().is_success() {
            Ok(self)
        } else {
            #[derive(Deserialize)]
            struct UpdateRepositoryError {
                message: String,
            }

            let resp: UpdateRepositoryError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn delete(&self) -> Result<(), Error>
    {
        let url = self.client.api_url(
            &format!("/repos/{}/{}",
                     self.subject,
                     self.repository))?;

        let mut response = self.client
            .delete(url)
            .send()?;

        if response.status().is_success() {
            Ok(())
        } else {
            #[derive(Deserialize)]
            struct DeleteRepositoryError {
                message: String,
            }

            let resp: DeleteRepositoryError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn get_name(&self) -> &str            { &self.repository }
    pub fn get_subject(&self) -> &str         { &self.subject }
    pub fn get_type(&self) -> &RepositoryType { &self.type_ }
    pub fn is_private(&self) -> bool          { self.private }
    pub fn is_premium(&self) -> bool          { self.premium }
    pub fn get_desc(&self) -> &str            { &self.desc }
    pub fn get_labels(&self) -> &Vec<String>  { &self.labels }
    pub fn get_gpg_use_owner_key(&self) -> bool { self.gpg_use_owner_key }
    pub fn get_gpg_sign_files(&self) -> bool    { self.gpg_sign_files }
    pub fn get_gpg_sign_metadata(&self) -> bool { self.gpg_sign_metadata }
    pub fn get_created(&self) -> &Option<DateTime<Utc>> { &self.created }

    pub fn get_default_debian_architecture(&self) -> &Option<String>
    {
        &self.default_debian_architecture
    }

    pub fn get_default_debian_distribution(&self) -> &Option<String>
    {
        &self.default_debian_distribution
    }

    pub fn get_default_debian_component(&self) -> &Option<String>
    {
        &self.default_debian_component
    }

    pub fn get_yum_metadata_depth(&self) -> &Option<usize>
    {
        &self.yum_metadata_depth
    }

    pub fn get_yum_groups_file(&self) -> &Option<String>
    {
        &self.yum_groups_file
    }

    fn package_names_iter(&self)
        -> Result<Map<IntoIter<PackageNamesListEntry>,
        fn(PackageNamesListEntry) -> String>, Error>
    {
        let url = self.client.api_url(
            &format!("/repos/{}/{}/packages",
                     self.subject,
                     self.repository))?;

        let mut response = self.client
            .get(url)
            .send()?;

        if response.status().is_success() {
            let package_entries: Vec<PackageNamesListEntry> = response.json()?;

            fn extract_package_name(e: PackageNamesListEntry) -> String {
                e.name
            }
            let extract_package_name: fn(PackageNamesListEntry) -> String =
                extract_package_name;

            let package_names_iter = package_entries
                .into_iter()
                .map(extract_package_name);
            Ok(package_names_iter)
        } else {
            #[derive(Deserialize)]
            struct ListPackageNamesError {
                message: String,
            }

            let resp: ListPackageNamesError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn package_names(&self) -> Result<Vec<String>, Error>
    {
        let mut package_names: Vec<String> = self
            .package_names_iter()?
            .collect();
        package_names.sort();

        Ok(package_names)
    }

    pub fn package(&self, package_name: &str) -> Package
    {
        Package::new(&self.client,
                     &self.subject,
                     &self.repository,
                     package_name)
    }
}

impl fmt::Display for Repository {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "bintray::Repository({}:{} {:?})",
            self.subject,
            self.repository,
            self.type_)
    }
}
