use hyper::status::StatusCode;
use serde_json;
use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read};

use client::{BintrayClient, BintrayError};
use package::Package;
use content::Content;
use utils;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    pub owner: String,
    #[serde(rename = "repo")]
    pub repository: String,
    pub package: String,
    #[serde(rename = "name")]
    pub version: String,

    #[serde(skip_serializing_if="Option::is_none")]
    pub desc: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub published: Option<bool>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub updated: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub released: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub ordinal: Option<u64>,

    #[serde(skip_serializing_if="Option::is_none")]
    pub vcs_tag: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub github_release_notes_file: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub github_use_tag_release_notes: Option<bool>,

    #[serde(skip_serializing_if="Option::is_none")]
    pub attribute_names: Option<Vec<String>>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub attributes: Option<HashMap<String, String>>,
}

impl Version {
    pub fn new(owner: &str, repository: &str, package: &str,
               version: &str) -> Version
    {
        Version {
            version: String::from(version),
            package: String::from(package),
            repository: String::from(repository),
            owner: String::from(owner),

            desc: None,
            labels: None,
            published: None,
            created: None,
            updated: None,
            released: None,
            ordinal: None,

            vcs_tag: None,
            github_release_notes_file: None,
            github_use_tag_release_notes: None,
            attribute_names: None,
            attributes: None
        }
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
            path.push("versions");
            path.push(&self.version);
        }

        if get_attribute_values {
            url.query_pairs_mut().append_pair("attribute_values", "1");
        }

        let mut resp = client.get(url).send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("GetVersion({}): {}", self, body);

                let queried: Version = serde_json::from_str(&body)?;

                self.desc = queried.desc;
                self.labels = queried.labels
                    .map(|mut v| { v.sort(); v });
                self.created = queried.created;
                self.updated = queried.updated;
                self.released = queried.released
                    .and_then(|s| {
                        if s.is_empty() { None } else {  Some(s) }
                    });
                self.ordinal = queried.ordinal;

                self.vcs_tag = queried.vcs_tag;
                self.github_release_notes_file =
                    queried.github_release_notes_file;
                self.github_use_tag_release_notes =
                    queried.github_use_tag_release_notes;

                self.attribute_names = queried.attribute_names
                    .map(|mut v| { v.sort(); v });
                self.attributes = queried.attributes;

                Ok(())
            }
            _ if resp.status == StatusCode::NotFound => {
                report_bintray_error!(
                    self, resp, body, "GetVersion",
                    io::ErrorKind::NotFound,
                    "Package version not found", true)
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "GetVersion",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "GetVersion",
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
            path.push(&self.package);
            path.push("versions");
        }

        #[derive(Serialize)]
        struct CreateVersionReq {
            name: String,
            #[serde(skip_serializing_if="Option::is_none")]
            desc: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            released: Option<String>,

            #[serde(skip_serializing_if="Option::is_none")]
            vcs_tag: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            github_release_notes_file: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            github_use_tag_release_notes: Option<bool>,
        };

        let args = CreateVersionReq {
            name: self.version.clone(),
            desc: self.desc.clone(),
            released: self.released.clone(),

            vcs_tag: self.vcs_tag.clone(),
            github_release_notes_file: self.github_release_notes_file.clone(),
            github_use_tag_release_notes: self.github_use_tag_release_notes,
        };

        let json = serde_json::to_string_pretty(&args)?;
        info!(
            "CreateVersion({}): Submitting the following properties:\n{}",
            self, json);

        let mut resp = client.post(url)
            .body(&json)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Created => {
                info!("CreateVersion({}): {}", self, body);

                let created: Version = serde_json::from_str(&body)?;
                self.created = created.created;
                self.updated = created.updated;
                // TODO: Assert that created == package. */

                Ok(())
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "CreateVersion",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "CreateVersion",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "CreateVersion",
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
            path.push("versions");
            path.push(&self.version);
        }

        #[derive(Serialize)]
        struct UpdateVersionReq {
            #[serde(skip_serializing_if="Option::is_none")]
            desc: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            released: Option<String>,

            #[serde(skip_serializing_if="Option::is_none")]
            vcs_tag: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            github_release_notes_file: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            github_use_tag_release_notes: Option<bool>,
        };

        let args = UpdateVersionReq {
            desc: self.desc.clone(),
            released: self.released.clone(),

            vcs_tag: self.vcs_tag.clone(),
            github_release_notes_file: self.github_release_notes_file.clone(),
            github_use_tag_release_notes: self.github_use_tag_release_notes,
        };

        let json = serde_json::to_string_pretty(&args)?;
        info!(
            "UpdateVersion({}): Submitting the following properties:\n{}",
            self, json);

        let mut resp = client.patch(url)
            .body(&json)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("UpdateVersion({}): {}", self, body);
                Ok(())
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "UpdateVersion",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "UpdateVersion",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "UpdateVersion",
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
                        &self.package,
                        "versions",
                        &self.version]);
        }

        let mut resp = client.delete(url)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("DeleteVersion({}): {}", self, body);

                report_bintray_warning!(
                    self, resp, body, "DeleteVersion")
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "DeleteVersion",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "DeleteVersion",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "DeleteVersion",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    pub fn publish_content(&self,
                           wait_for_publish: Option<i32>,
                           discard_unpublished: bool,
                           client: &BintrayClient)
        -> Result<usize, BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.push("content");
            path.push(&self.owner);
            path.push(&self.repository);
            path.push(&self.package);
            path.push(&self.version);
            path.push("publish");
        }

        #[derive(Serialize)]
        struct PublishContentReq {
            #[serde(skip_serializing_if="Option::is_none")]
            publish_wait_for_secs: Option<i32>,
            discard: bool,
        };

        #[derive(Deserialize)]
        struct PublishContentResp {
            files: usize
        };

        let args = PublishContentReq {
            publish_wait_for_secs: wait_for_publish,
            discard: discard_unpublished,
        };

        let json = serde_json::to_string_pretty(&args)?;
        info!(
            "PublishContent({}): Submitting the following properties:\n{}",
            self, json);

        let mut resp = client.post(url)
            .body(&json)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("PublishContent({}): {}", self, body);

                let result: PublishContentResp =
                    serde_json::from_str(&body)?;

                Ok(result.files)
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "PublishContent",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "PublishContent",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "PublishContent",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    pub fn list_files(&self,
                      include_unpublished: bool,
                      client: &BintrayClient)
        -> Result<Vec<Content>, BintrayError>
    {
        let package = Package::new(&self.owner,
                                   &self.repository,
                                   &self.package);
        package.list_files(Some(&self.version), include_unpublished, client)
    }
}

impl fmt::Display for Version {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}/{}/{}", self.owner, self.repository, self.package,
               self.version)
    }
}
