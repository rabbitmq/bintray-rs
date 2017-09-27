use hyper::status::StatusCode;
use serde_json;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read};

use client::{BintrayClient, BintrayError};
use version::Version;
use content::Content;
use utils;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub owner: String,
    #[serde(rename = "repo")]
    pub repository: String,
    #[serde(rename = "name")]
    pub package: String,

    #[serde(skip_serializing_if="Option::is_none")]
    pub desc: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub updated: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub linked_to_repos: Option<Vec<String>>,
    pub public_download_numbers: bool,
    pub public_stats: bool,
    #[serde(skip_serializing_if="Option::is_none")]
    pub permissions: Option<Vec<String>>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub maturity: Option<PackageMaturity>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub versions: Vec<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub system_ids: Option<Vec<String>>,

    #[serde(skip_serializing_if="Option::is_none")]
    pub licenses: Option<Vec<String>>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub custom_licenses: Option<Vec<String>>,

    #[serde(skip_serializing_if="Option::is_none")]
    pub rating: Option<u32>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub rating_count: Option<u64>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub followers_count: Option<u64>,

    #[serde(skip_serializing_if="Option::is_none")]
    pub website_url: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub issue_tracker_url: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub vcs_url: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub github_repo: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub github_release_notes_file: Option<String>,

    #[serde(skip_serializing_if="Option::is_none")]
    pub attribute_names: Option<Vec<String>>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub attributes: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum PackageMaturity {
    Official,
    Stable,
    Development,
    Experimental,
    #[serde(rename = "")]
    Unset
}

impl Package {
    pub fn new(owner: &str, repository: &str, package: &str) -> Package {
        Package {
            package: String::from(package),
            repository: String::from(repository),
            owner: String::from(owner),

            desc: None,
            labels: None,
            created: None,
            updated: None,
            linked_to_repos: None,
            public_download_numbers: false,
            public_stats: false,
            permissions: None,
            maturity: None,
            versions: vec![],
            latest_version: None,
            system_ids: None,

            licenses: None,
            custom_licenses: None,

            rating: None,
            rating_count: None,
            followers_count: None,

            website_url: None,
            issue_tracker_url: None,
            vcs_url: None,
            github_repo: None,
            github_release_notes_file: None,

            attribute_names: None,
            attributes: None,
        }
    }

    pub fn set_licenses<T: Borrow<str>>(mut self, licenses: &[T]) -> Package {
        let mut licenses = licenses.iter()
            .map(|s| String::from(s.borrow()))
            .collect::<Vec<_>>();
        licenses.sort();

        self.licenses = Some(licenses);
        self
    }

    pub fn set_vcs_url(mut self, vcs_url: &str) -> Package {
        self.vcs_url = Some(String::from(vcs_url));
        self
    }

    pub fn get(&mut self, get_attribute_values: bool, client: &BintrayClient)
        -> Result<(), BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.push("packages");
            path.push(&self.owner);
            path.push(&self.repository);
            path.push(&self.package);
        }

        if get_attribute_values {
            url.query_pairs_mut().append_pair("attribute_values", "1");
        }

        let mut resp = client.get(url).send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("GetPackage({}): {}", self, body);

                let queried: Package = serde_json::from_str(&body)?;

                self.desc = queried.desc;
                self.labels = queried.labels
                    .map(|mut v| { v.sort(); v });
                self.created = queried.created;
                self.updated = queried.updated;
                self.linked_to_repos = queried.linked_to_repos
                    .map(|mut v| { v.sort(); v });
                self.public_download_numbers =
                    queried.public_download_numbers;
                self.public_stats = queried.public_stats;
                self.permissions = queried.permissions
                    .map(|mut v| { v.sort(); v });
                self.maturity = queried.maturity;
                self.versions = queried.versions;
                self.latest_version = queried.latest_version;
                self.system_ids = queried.system_ids
                    .map(|mut v| { v.sort(); v });

                self.licenses = queried.licenses
                    .map(|mut v| { v.sort(); v });
                self.custom_licenses = queried.custom_licenses
                    .map(|mut v| { v.sort(); v });

                self.rating = queried.rating;
                self.rating_count = queried.rating_count;
                self.followers_count = queried.followers_count;

                self.website_url = queried.website_url;
                self.issue_tracker_url = queried.issue_tracker_url;
                self.vcs_url = queried.vcs_url;
                self.github_repo = queried.github_repo
                    .and_then(|s| {
                        if s.is_empty() { None } else {  Some(s) }
                    });
                self.github_release_notes_file =
                    queried.github_release_notes_file
                    .and_then(|s| {
                        if s.is_empty() { None } else {  Some(s) }
                    });

                self.attribute_names = queried.attribute_names
                    .map(|mut v| { v.sort(); v });
                self.attributes = queried.attributes;

                self.versions.reverse();

                Ok(())
            }
            _ if resp.status == StatusCode::NotFound => {
                report_bintray_error!(
                    self, resp, body, "GetPackage",
                    io::ErrorKind::NotFound,
                    "Package not found", true)
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "GetPackage",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "GetPackage",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    pub fn exists(&mut self, client: &BintrayClient)
        -> Result<bool, BintrayError>
    {
        match self.get(false, client) {
            Ok(()) => Ok(true),
            Err(BintrayError::Io(ref e))
                if e.kind() == io::ErrorKind::NotFound => {
                    Ok(false)
                }
            Err(e) => Err(e),
        }
    }

    pub fn create(&mut self, client: &BintrayClient)
        -> Result<(), BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.push("packages");
            path.push(&self.owner);
            path.push(&self.repository);
        }

        #[derive(Serialize)]
        struct CreatePackageReq {
            name: String,
            #[serde(skip_serializing_if="Option::is_none")]
            desc: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            labels: Option<Vec<String>>,
            public_download_numbers: bool,
            public_stats: bool,
            #[serde(skip_serializing_if="Option::is_none")]
            maturity: Option<PackageMaturity>,

            #[serde(skip_serializing_if="Option::is_none")]
            licenses: Option<Vec<String>>,
            #[serde(skip_serializing_if="Option::is_none")]
            custom_licenses: Option<Vec<String>>,

            #[serde(skip_serializing_if="Option::is_none")]
            website_url: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            issue_tracker_url: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            vcs_url: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            github_repo: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            github_release_notes_file: Option<String>,
        };

        let args = CreatePackageReq {
            name: self.package.clone(),
            desc: self.desc.clone(),
            labels: self.labels.clone(),
            public_download_numbers: self.public_download_numbers,
            public_stats: self.public_stats,
            maturity: self.maturity.clone(),

            licenses: self.licenses.clone(),
            custom_licenses: self.custom_licenses.clone(),

            website_url: self.website_url.clone(),
            issue_tracker_url: self.issue_tracker_url.clone(),
            vcs_url: self.vcs_url.clone(),
            github_repo: self.github_repo.clone(),
            github_release_notes_file: self.github_release_notes_file.clone(),
        };

        let json = serde_json::to_string_pretty(&args)?;
        info!(
            "CreatePackage({}): Submitting the following properties:\n{}",
            self, json);

        let mut resp = client.post(url)
            .body(&json)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Created => {
                info!("CreatePackage({}): {}", self, body);

                let created: Package = serde_json::from_str(&body)?;
                self.created = created.created;
                self.updated = created.updated;
                // TODO: Assert that created == package. */

                Ok(())
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "CreatePackage",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "CreatePackage",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "CreatePackage",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    pub fn update(&mut self, client: &BintrayClient)
        -> Result<(), BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.push("packages");
            path.push(&self.owner);
            path.push(&self.repository);
            path.push(&self.package);
        }

        #[derive(Serialize)]
        struct UpdatePackageReq {
            #[serde(skip_serializing_if="Option::is_none")]
            desc: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            labels: Option<Vec<String>>,
            public_download_numbers: bool,
            public_stats: bool,
            #[serde(skip_serializing_if="Option::is_none")]
            maturity: Option<PackageMaturity>,

            #[serde(skip_serializing_if="Option::is_none")]
            licenses: Option<Vec<String>>,
            #[serde(skip_serializing_if="Option::is_none")]
            custom_licenses: Option<Vec<String>>,

            #[serde(skip_serializing_if="Option::is_none")]
            website_url: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            issue_tracker_url: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            vcs_url: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            github_repo: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            github_release_notes_file: Option<String>,
        };

        let args = UpdatePackageReq {
            desc: self.desc.clone(),
            labels: self.labels.clone(),
            public_download_numbers: self.public_download_numbers,
            public_stats: self.public_stats,
            maturity: self.maturity.clone(),

            licenses: self.licenses.clone(),
            custom_licenses: self.custom_licenses.clone(),

            website_url: self.website_url.clone(),
            issue_tracker_url: self.issue_tracker_url.clone(),
            vcs_url: self.vcs_url.clone(),
            github_repo: self.github_repo.clone(),
            github_release_notes_file: self.github_release_notes_file.clone(),
        };

        let json = serde_json::to_string_pretty(&args)?;
        info!(
            "UpdatePackage({}): Submitting the following properties:\n{}",
            self, json);

        let mut resp = client.patch(url)
            .body(&json)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("UpdatePackage({}): {}", self, body);
                Ok(())
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "UpdatePackage",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "UpdatePackage",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "UpdatePackage",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    pub fn delete(&self, client: &BintrayClient)
        -> Result<Option<String>, BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.extend(&["packages",
                        &self.owner,
                        &self.repository,
                        &self.package]);
        }

        let mut resp = client.delete(url)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("DeletePackage({}): {}", self, body);

                report_bintray_warning!(
                    self, resp, body, "DeletePackage")
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "DeletePackage",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "DeletePackage",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "DeletePackage",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    pub fn get_latest_version(&self, client: Option<&BintrayClient>)
        -> Option<Version>
    {
        self.get_versions_starting_at(&self.latest_version, client).pop()
    }

    pub fn get_versions(&self, client: Option<&BintrayClient>)
        -> Vec<Version>
    {
        self.get_versions_starting_at(&None, client)
    }

    pub fn get_versions_starting_at(&self,
                                    oldest: &Option<String>,
                                    client: Option<&BintrayClient>)
        -> Vec<Version>
    {
        let filter = |version: &String| -> Version {
            let mut version = Version::new(
                &self.owner,
                &self.repository,
                &self.package,
                &version);
            match client {
                Some(client) => {
                    match version.get(false, client) {
                        Ok(()) => { }
                        Err(e) => error!("Failed to query version: {}", e),
                    };
                }
                None => { }
            }
            version
        };

        match oldest {
            &None => {
                self.versions.iter()
                    .map(filter)
                    .collect::<Vec<Version>>()
            }
            &Some(ref oldest) => {
                self.versions.iter()
                    .skip_while(|&version| version != oldest)
                    .map(filter)
                    .collect::<Vec<Version>>()
            }
        }
    }

    pub fn list_files(&self,
                      version: Option<&str>,
                      include_unpublished: bool,
                      client: &BintrayClient)
        -> Result<Vec<Content>, BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.push("packages");
            path.push(&self.owner);
            path.push(&self.repository);
            path.push(&self.package);
            match version {
                Some(version) => { path.extend(&["versions", version]); },
                None          => { }
            }
            path.push("files");
        }

        if include_unpublished {
            url.query_pairs_mut().append_pair("include_unpublished", "1");
        }

        #[derive(Deserialize)]
        struct GetFilesResp {
            owner: String,
            #[serde(rename = "repo")]
            repository: String,
            package: String,
            version: String,
            //name: String,
            path: String,
            created: String,
            size: usize,
            sha1: String,
        };

        let mut resp = client.get(url).send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("GetFiles({}): {}", self, body);

                let result: Vec<GetFilesResp> =
                    serde_json::from_str(&body)?;

                let files = result.into_iter()
                    .map(|item| {
                        let mut file = Content::new(
                            &item.owner,
                            &item.repository,
                            &item.package,
                            &item.version,
                            &item.path);
                        file.created = Some(item.created.clone());
                        file.size = Some(item.size);
                        file.sha1 = Some(item.sha1.clone());
                        file
                    })
                    .collect();
                Ok(files)
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "GetFiles",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "GetFiles",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "GetFiles",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }
}

impl fmt::Display for Package {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}/{}", self.owner, self.repository, self.package)
    }
}

impl From<String> for PackageMaturity {
    fn from(from: String) -> PackageMaturity {
        match from.to_lowercase().trim() {
            "official"     => PackageMaturity::Official,
            "stable"       => PackageMaturity::Stable,
            "development"  => PackageMaturity::Development,
            "experimental" => PackageMaturity::Experimental,
            _              => PackageMaturity::Unset,
        }
    }
}

impl fmt::Display for PackageMaturity {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        let type_ = match self {
            &PackageMaturity::Official => "Official",
            &PackageMaturity::Stable => "Stable",
            &PackageMaturity::Development => "Generic",
            &PackageMaturity::Experimental => "Experimental",
            &PackageMaturity::Unset => "None",
        };
        write!(f, "{}", type_)
    }
}
