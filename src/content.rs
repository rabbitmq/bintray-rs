use hyper::client::Body;
use hyper::status::StatusCode;
use serde_json;
use std::borrow::Borrow;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, BufReader, Write};
use std::path::{Path, PathBuf, Component};

use client::{BintrayClient, BintrayError};
use utils;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Content {
    pub owner: String,
    #[serde(rename = "repo")]
    pub repository: String,
    pub package: String,
    pub version: String,
    pub path: PathBuf,

    #[serde(skip_serializing_if="Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub size: Option<usize>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub sha1: Option<String>,
}

impl Content {
    pub fn new<T: AsRef<Path>>(owner: &str,
                               repository: &str,
                               package: &str,
                               version: &str,
                               path: T) -> Content
    {
        let cleaned_path = clean_path(path);

        Content {
            path: cleaned_path,
            version: String::from(version),
            package: String::from(package),
            repository: String::from(repository),
            owner: String::from(owner),

            created: None,
            size: None,
            sha1: None,
        }
    }

    pub fn upload<T: Borrow<str>>(&self,
                                 local_filename: &PathBuf,
                                 publish: bool,
                                 override_: bool,
                                 explode: bool,
                                 gpg_passphrase: Option<&str>,
                                 debian_architecture: &[T],
                                 debian_distribution: &[T],
                                 debian_component: &[T],
                                 client: &BintrayClient)
        -> Result<Option<String>, BintrayError>
    {
        // TODO: Suport Maven upload which use a different URL.

        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.extend(&["content",
                        &self.owner,
                        &self.repository,
                        &self.package,
                        &self.version]);
            path.extend(self.path.iter().map(|v| v.to_string_lossy()));
        }

        if publish {
            url.query_pairs_mut().append_pair("publish", "1");
        }
        if override_ {
            url.query_pairs_mut().append_pair("override", "1");
        }
        if explode {
            url.query_pairs_mut().append_pair("explode", "1");
        }

        let local_file = File::open(local_filename)?;
        let local_file_size = local_file.metadata()?.len();
        let mut local_file_reader = BufReader::new(local_file);

        let mut resp = client.put(url)
            .add_gpg_passphrase(gpg_passphrase)
            .add_debian_architecture(debian_architecture)
            .add_debian_distribution(debian_distribution)
            .add_debian_component(debian_component)
            .body(Body::SizedBody(&mut local_file_reader, local_file_size))
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Created => {
                info!("UploadContent({}): {}", self, body);

                report_bintray_warning!(
                    self, resp, body, "UploadContent")
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "UploadContent",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "UploadContent",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ if resp.status == StatusCode::Conflict => {
                report_bintray_error!(
                    self, resp, body, "UploadContent",
                    io::ErrorKind::PermissionDenied,
                    "Conflict with existing file")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "UploadContent",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    pub fn download(&self,
                    local_filename: &PathBuf,
                    client: &BintrayClient)
        -> Result<(), BintrayError>
    {
        let mut url = client.get_dl_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.extend(&[&self.owner,
                        &self.repository]);
            path.extend(self.path.iter().map(|v| v.to_string_lossy()));
        }

        let mut resp = client.get(url)
            .send()?;

        let body = String::from("(body not logged)");

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("DownloadContent({}): Ok {}", self, body);

                let mut local_file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(local_filename)?;

                let buffer = &mut vec![0; 65536];
                loop {
                    let len_read = resp.read(buffer)?;
                    if len_read == 0 {
                        break;
                    }

                    local_file.write_all(&buffer[..len_read])?;
                }

                Ok(())
            }
            _ if resp.status == StatusCode::NotFound => {
                report_bintray_error!(
                    self, resp, body, "DownloadContent",
                    io::ErrorKind::NotFound,
                    "File not found")
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "DownloadContent",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "DownloadContent",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "DownloadContent",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    pub fn show_in_download_list(&self, show: bool, client: &BintrayClient)
        -> Result<Option<String>, BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.extend(&["file_metadata",
                        &self.owner,
                        &self.repository]);
            path.extend(self.path.iter().map(|v| v.to_string_lossy()));
        }

        #[derive(Serialize)]
        struct ShowInDownloadListReq {
            list_in_downloads: bool
        };

        let args = ShowInDownloadListReq {
            list_in_downloads: show,
        };

        let json = serde_json::to_string_pretty(&args)?;
        info!(
            "ShowInDownloadList({}): Submitting the following properties:\n{}",
            self, json);

        let mut resp = client.put(url)
            .body(&json)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("ShowInDownloadList({}): {}", self, body);

                report_bintray_warning!(
                    self, resp, body, "ShowInDownloadList")
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "ShowInDownloadList",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "ShowInDownloadList",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ if resp.status == StatusCode::BadRequest &&
            body.contains("unpublished file") => {
                report_bintray_error!(
                    self, resp, body, "ShowInDownloadList",
                    io::ErrorKind::NotFound,
                    "File is not yet published")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "ShowInDownloadList",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }

    pub fn remove(&self, client: &BintrayClient)
        -> Result<Option<String>, BintrayError>
    {
        let mut url = client.get_base_url();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.extend(&["content",
                        &self.owner,
                        &self.repository]);
            path.extend(self.path.iter().map(|v| v.to_string_lossy()));
        }

        let mut resp = client.delete(url)
            .send()?;

        let mut body = String::new();
        resp.read_to_string(&mut body)?;

        match resp {
            _ if resp.status == StatusCode::Ok => {
                info!("DeleteContent({}): {}", self, body);

                report_bintray_warning!(
                    self, resp, body, "DeleteContent")
            }
            _ if resp.status == StatusCode::Unauthorized => {
                report_bintray_error!(
                    self, resp, body, "DeleteContent",
                    io::ErrorKind::PermissionDenied,
                    "Missing or refused authentication")
            }
            _ if resp.status == StatusCode::Forbidden => {
                report_bintray_error!(
                    self, resp, body, "DeleteContent",
                    io::ErrorKind::PermissionDenied,
                    "Requires admin privileges")
            }
            _ => {
                report_bintray_error!(
                    self, resp, body, "DeleteContent",
                    io::ErrorKind::Other,
                    "Unrecognized error")
            }
        }
    }
}

pub fn clean_path<T: AsRef<Path>>(path: T) -> PathBuf {
    let initial_path = Path::new(path.as_ref());
    let cleaned_components = initial_path.components()
        .skip_while(|&c| match c {
            Component::Prefix(_) => true,
            Component::RootDir   => true,
            Component::CurDir    => true,
            Component::ParentDir => true,
            Component::Normal(_) => false,
        })
        .map(Component::as_os_str);
    let mut cleaned_path = PathBuf::new();
    cleaned_path.extend(cleaned_components);
    cleaned_path
}

impl fmt::Display for Content {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.path.is_absolute() {
            write!(f, "{}/{}/{}/{}{}", self.owner, self.repository, self.package,
                   self.version, self.path.display())
        } else {
            write!(f, "{}/{}/{}/{}/{}", self.owner, self.repository, self.package,
                   self.version, self.path.display())
        }
    }
}
