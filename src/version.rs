use chrono::{DateTime, Utc};
use failure::Error;
use reqwest::StatusCode;
use std::fmt;
use std::path::Path;
use ::{BintrayError, Client, Content, RepositoryType};

#[derive(Clone, Debug)]
pub struct Version {
    subject: String,
    repository: String,
    package: String,
    version: String,

    desc: String,
    labels: Vec<String>,
    released: Option<DateTime<Utc>>,
    vcs_tag: Option<String>,
    github_use_tag_release_notes: bool,
    github_release_notes_file: Option<String>,
    published: bool,
    created: Option<DateTime<Utc>>,
    updated: Option<DateTime<Utc>>,

    client: Client,
}

impl Version {
    pub fn new(client: &Client,
               subject: &str,
               repository: &str,
               package: &str,
               version: &str)
        -> Self
    {
        Version {
            subject: String::from(subject),
            repository: String::from(repository),
            package: String::from(package),
            version: String::from(version),

            desc: String::new(),
            labels: vec![],
            released: None,
            vcs_tag: None,
            github_use_tag_release_notes: false,
            github_release_notes_file: None,
            published: false,
            created: None,
            updated: None,

            client: client.clone(),
        }
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

    pub fn released(mut self, released: &DateTime<Utc>) -> Self
    {
        self.set_released(released);
        self
    }

    pub fn set_released(&mut self, released: &DateTime<Utc>) -> &mut Self
    {
        self.released = Some(released.clone());
        self
    }

    pub fn vcs_tag(mut self, vcs_tag: &str) -> Self
    {
        self.set_vcs_tag(vcs_tag);
        self
    }

    pub fn set_vcs_tag(&mut self, vcs_tag: &str) -> &mut Self
    {
        self.vcs_tag = Some(String::from(vcs_tag));
        self
    }

    pub fn github_use_tag_release_notes(mut self,
                                        github_use_tag_release_notes: bool)
        -> Self
    {
        self.set_github_use_tag_release_notes(github_use_tag_release_notes);
        self
    }

    pub fn set_github_use_tag_release_notes(&mut self,
                                            github_use_tag_release_notes: bool)
        -> &mut Self
    {
        self.github_use_tag_release_notes = github_use_tag_release_notes;
        self
    }

    pub fn github_release_notes_file(mut self,
                                     github_release_notes_file: &str)
        -> Self
    {
        self.set_github_release_notes_file(github_release_notes_file);
        self
    }

    pub fn set_github_release_notes_file(&mut self,
                                         github_release_notes_file: &str)
        -> &mut Self
    {
        self.github_release_notes_file =
            Some(String::from(github_release_notes_file));
        self
    }

    pub fn create(mut self) -> Result<Self, Error>
    {
        let url = self.client.api_url(
            &format!("/packages/{}/{}/{}/versions",
                     self.subject,
                     self.repository,
                     self.package))?;

        #[derive(Serialize)]
        struct CreateVersionReq {
            name: String,

            desc: String,
            labels: Vec<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            released: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            vcs_tag: Option<String>,
            github_use_tag_release_notes: bool,
            #[serde(skip_serializing_if="Option::is_none")]
            github_release_notes_file: Option<String>,
        }

        let req = CreateVersionReq {
            name: self.version.clone(),

            desc: self.desc.clone(),
            labels: self.labels.clone(),
            released: self.released.as_ref().map(|d| d.to_rfc3339()),
            vcs_tag: self.vcs_tag.clone(),
            github_use_tag_release_notes: self.github_use_tag_release_notes,
            github_release_notes_file: self.github_release_notes_file.clone(),
        };

        let mut response = self.client
            .post(url)
            .json(&req)
            .send()?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct CreateVersionResp {
                owner: String,
                repo: String,
                package: String,
                name: String,

                desc: String,
                labels: Vec<String>,
                released: String,
                vcs_tag: Option<String>,
                github_use_tag_release_notes: bool,
                github_release_notes_file: Option<String>,
                published: bool,
                created: String,
                updated: String,
            }

            let mut resp: CreateVersionResp = response.json()?;
            resp.labels.sort();

            debug_assert_eq!(self.subject, resp.owner);
            debug_assert_eq!(self.repository, resp.repo);
            debug_assert_eq!(self.package, resp.package);
            debug_assert_eq!(self.version, resp.name);
            debug_assert_eq!(self.desc, resp.desc);
            debug_assert_eq!(self.labels, resp.labels);
            debug_assert_eq!(self.vcs_tag, resp.vcs_tag);
            debug_assert_eq!(self.github_use_tag_release_notes,
                             resp.github_use_tag_release_notes);
            debug_assert_eq!(self.github_release_notes_file,
                             resp.github_release_notes_file);

            if let Some(ref released) = self.released {
                debug_assert_eq!(released.to_rfc3339(), resp.released);
            } else {
                self.released = resp.released.parse::<DateTime<Utc>>().ok();
            }

            self.published = resp.published;
            self.created = resp.created.parse::<DateTime<Utc>>().ok();
            self.updated = resp.updated.parse::<DateTime<Utc>>().ok();

            Ok(self)
        } else {
            #[derive(Deserialize)]
            struct CreateVersionError {
                message: String,
            }

            let resp: CreateVersionError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn exists(&self) -> Result<bool, Error>
    {
        let url = self.client.api_url(
            &format!("/packages/{}/{}/{}/versions/{}",
                     self.subject,
                     self.repository,
                     self.package,
                     self.version))?;

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
            &format!("/packages/{}/{}/{}/versions/{}",
                     self.subject,
                     self.repository,
                     self.package,
                     self.version))?;

        let mut response = self.client
            .get(url)
            .send()?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct GetVersionResp {
                owner: String,
                repo: String,
                package: String,
                name: String,

                desc: String,
                labels: Vec<String>,
                released: String,
                vcs_tag: Option<String>,
                github_use_tag_release_notes: bool,
                github_release_notes_file: Option<String>,
                published: bool,
                created: String,
                updated: String,
            }

            let mut resp: GetVersionResp = response.json()?;
            resp.labels.sort();

            debug_assert_eq!(self.subject, resp.owner);
            debug_assert_eq!(self.repository, resp.repo);
            debug_assert_eq!(self.package, resp.package);
            debug_assert_eq!(self.version, resp.name);

            self.desc = resp.desc;
            self.labels = resp.labels;
            self.released = resp.released.parse::<DateTime<Utc>>().ok();
            self.vcs_tag = resp.vcs_tag;
            self.github_use_tag_release_notes =
                resp.github_use_tag_release_notes;
            self.github_release_notes_file =
                resp.github_release_notes_file;
            self.published = resp.published;
            self.created = resp.created.parse::<DateTime<Utc>>().ok();
            self.updated = resp.updated.parse::<DateTime<Utc>>().ok();

            Ok(self)
        } else {
            #[derive(Deserialize)]
            struct GetVersionError {
                message: String,
            }

            let resp: GetVersionError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn update(&self) -> Result<&Self, Error>
    {
        let url = self.client.api_url(
            &format!("/packages/{}/{}/{}/versions/{}",
                     self.subject,
                     self.repository,
                     self.package,
                     self.version))?;

        #[derive(Serialize)]
        struct UpdateVersionReq {
            desc: String,
            labels: Vec<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            released: Option<String>,
            #[serde(skip_serializing_if="Option::is_none")]
            vcs_tag: Option<String>,
            github_use_tag_release_notes: bool,
            #[serde(skip_serializing_if="Option::is_none")]
            github_release_notes_file: Option<String>,
        }

        let req = UpdateVersionReq {
            desc: self.desc.clone(),
            labels: self.labels.clone(),
            released: self.released.as_ref().map(|d| d.to_rfc3339()),
            vcs_tag: self.vcs_tag.clone(),
            github_use_tag_release_notes: self.github_use_tag_release_notes,
            github_release_notes_file: self.github_release_notes_file.clone(),
        };

        let mut response = self.client
            .patch(url)
            .json(&req)
            .send()?;

        if response.status().is_success() {
            Ok(self)
        } else {
            #[derive(Deserialize)]
            struct UpdateVersionError {
                message: String,
            }

            let resp: UpdateVersionError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn delete(&self) -> Result<(), Error>
    {
        let url = self.client.api_url(
            &format!("/packages/{}/{}/{}/versions/{}",
                     self.subject,
                     self.repository,
                     self.package,
                     self.version))?;

        let mut response = self.client
            .delete(url)
            .send()?;

        if response.status().is_success() {
            Ok(())
        } else {
            #[derive(Deserialize)]
            struct DeleteVersionError {
                message: String,
            }

            let resp: DeleteVersionError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn get_version(&self) -> &str            { &self.version }
    pub fn get_package(&self) -> &str            { &self.package }
    pub fn get_repository(&self) -> &str         { &self.repository }
    pub fn get_subject(&self) -> &str            { &self.subject }
    pub fn get_desc(&self) -> &str               { &self.desc }
    pub fn get_labels(&self) -> &Vec<String>     { &self.labels }
    pub fn get_released(&self) -> &Option<DateTime<Utc>> { &self.released }
    pub fn get_vcs_tag(&self) -> &Option<String> { &self.vcs_tag }
    pub fn get_github_release_notes_file(&self) -> &Option<String>
    {
        &self.github_release_notes_file
    }
    pub fn get_github_use_tag_release_notes(&self) -> bool
    {
        self.github_use_tag_release_notes
    }
    pub fn is_published(&self) -> bool                  { self.published }
    pub fn get_created(&self) -> &Option<DateTime<Utc>> { &self.created }
    pub fn get_updated(&self) -> &Option<DateTime<Utc>> { &self.updated }

    pub fn file<T: AsRef<Path>>(&self,
                                path: T,
                                repo_type: Option<&RepositoryType>)
        -> Result<Content, Error>
    {
        Content::new(&self.client,
                     &self.subject,
                     &self.repository,
                     &self.package,
                     &self.version,
                     path,
                     repo_type)
    }
}

impl fmt::Display for Version {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "bintray::Version({}:{}:{}:{})",
            self.subject,
            self.repository,
            self.package,
            self.version)
    }
}
