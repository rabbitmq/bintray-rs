use chrono::{DateTime, Utc};
use failure::Error;
use reqwest::StatusCode;
use std::cmp::Ordering;
use std::fmt;
use version_compare::{CompOp, VersionCompare};
use ::{Client, BintrayError, Version};

#[derive(Clone, Debug)]
pub struct Package {
    subject: String,
    repository: String,
    package: String,

    desc: String,
    labels: Vec<String>,
    licenses: Vec<String>,
    website_url: String,
    vcs_url: String,
    issue_tracker_url: String,
    github_repo: String,
    github_release_notes_file: String,
    maturity: PackageMaturity,
    created: Option<DateTime<Utc>>,
    updated: Option<DateTime<Utc>>,
    versions: Option<Vec<String>>,

    client: Client,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageMaturity {
    Official,
    Stable,
    Development,
    Experimental,
    #[serde(rename = "")]
    Unset
}

impl Package {
    pub fn new(client: &Client,
               subject: &str,
               repository: &str,
               package: &str)
        -> Self
    {
        Package {
            subject: String::from(subject),
            repository: String::from(repository),
            package: String::from(package),

            desc: String::new(),
            labels: vec![],
            licenses: vec![],
            website_url: String::new(),
            vcs_url: String::new(),
            issue_tracker_url: String::new(),
            github_repo: String::new(),
            github_release_notes_file: String::new(),
            maturity: PackageMaturity::Unset,
            created: None,
            updated: None,
            versions: None,

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

    pub fn licenses<T: AsRef<str>>(mut self, licenses: &[T]) -> Self
    {
        self.set_licenses(licenses);
        self
    }

    pub fn set_licenses<T: AsRef<str>>(&mut self, licenses: &[T]) -> &mut Self
    {
        let mut vec: Vec<String> = licenses
            .iter()
            .map(|s| s.as_ref().to_owned())
            .collect();
        vec.sort();

        self.licenses = vec;
        self
    }

    pub fn website_url(mut self, website_url: &str) -> Self
    {
        self.set_website_url(website_url);
        self
    }

    pub fn set_website_url(&mut self, website_url: &str) -> &mut Self
    {
        self.website_url = String::from(website_url);
        self
    }

    pub fn vcs_url(mut self, vcs_url: &str) -> Self
    {
        self.set_vcs_url(vcs_url);
        self
    }

    pub fn set_vcs_url(&mut self, vcs_url: &str) -> &mut Self
    {
        self.vcs_url = String::from(vcs_url);
        self
    }

    pub fn issue_tracker_url(mut self, issue_tracker_url: &str) -> Self
    {
        self.set_issue_tracker_url(issue_tracker_url);
        self
    }

    pub fn set_issue_tracker_url(&mut self, issue_tracker_url: &str)
        -> &mut Self
    {
        self.issue_tracker_url = String::from(issue_tracker_url);
        self
    }

    pub fn github_repo(mut self, github_repo: &str) -> Self
    {
        self.set_github_repo(github_repo);
        self
    }

    pub fn set_github_repo(&mut self, github_repo: &str) -> &mut Self
    {
        self.github_repo = String::from(github_repo);
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
            String::from(github_release_notes_file);
        self
    }

    pub fn maturity(mut self, maturity: &PackageMaturity) -> Self
    {
        self.set_maturity(maturity);
        self
    }

    pub fn set_maturity(&mut self, maturity: &PackageMaturity) -> &mut Self
    {
        self.maturity = maturity.clone();
        self
    }

    pub fn create(mut self) -> Result<Self, Error>
    {
        let url = self.client.api_url(
            &format!("/packages/{}/{}",
                     self.subject,
                     self.repository))?;

        #[derive(Serialize)]
        struct CreatePackageReq {
            name: String,

            desc: String,
            labels: Vec<String>,
            licenses: Vec<String>,
            website_url: String,
            vcs_url: String,
            issue_tracker_url: String,
            github_repo: String,
            github_release_notes_file: String,
            maturity: PackageMaturity
        }

        let req = CreatePackageReq {
            name: self.package.clone(),

            desc: self.desc.clone(),
            labels: self.labels.clone(),
            licenses: self.licenses.clone(),
            website_url: self.website_url.clone(),
            vcs_url: self.vcs_url.clone(),
            issue_tracker_url: self.issue_tracker_url.clone(),
            github_repo: self.github_repo.clone(),
            github_release_notes_file: self.github_release_notes_file.clone(),
            maturity: self.maturity.clone(),
        };

        let mut response = self.client
            .post(url)
            .json(&req)
            .send()?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct CreatePackageResp {
                owner: String,
                repo: String,
                name: String,

                desc: String,
                labels: Vec<String>,
                licenses: Vec<String>,
                website_url: String,
                vcs_url: String,
                issue_tracker_url: String,
                github_repo: String,
                github_release_notes_file: String,
                maturity: PackageMaturity,
                created: String,
                updated: String,
            }

            let mut resp: CreatePackageResp = response.json()?;
            resp.labels.sort();
            resp.licenses.sort();

            debug_assert_eq!(self.subject, resp.owner);
            debug_assert_eq!(self.repository, resp.repo);
            debug_assert_eq!(self.package, resp.name);
            debug_assert_eq!(self.desc, resp.desc);
            debug_assert_eq!(self.labels, resp.labels);
            debug_assert_eq!(self.licenses, resp.licenses);
            debug_assert_eq!(self.website_url, resp.website_url);
            debug_assert_eq!(self.vcs_url, resp.vcs_url);
            debug_assert_eq!(self.issue_tracker_url, resp.issue_tracker_url);
            debug_assert_eq!(self.github_repo, resp.github_repo);
            debug_assert_eq!(self.github_release_notes_file,
                             resp.github_release_notes_file);
            debug_assert_eq!(self.maturity, resp.maturity);

            self.created = resp.created.parse::<DateTime<Utc>>().ok();
            self.updated = resp.updated.parse::<DateTime<Utc>>().ok();

            Ok(self)
        } else {
            #[derive(Deserialize)]
            struct CreatePackageError {
                message: String,
            }

            let resp: CreatePackageError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn exists(&self) -> Result<bool, Error>
    {
        let url = self.client.api_url(
            &format!("/packages/{}/{}/{}",
                     self.subject,
                     self.repository,
                     self.package))?;

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
            &format!("/packages/{}/{}/{}",
                     self.subject,
                     self.repository,
                     self.package))?;

        let mut response = self.client
            .get(url)
            .send()?;

        if response.status().is_success() {
            #[derive(Deserialize)]
            struct GetPackageResp {
                owner: String,
                repo: String,
                name: String,

                desc: String,
                labels: Vec<String>,
                licenses: Vec<String>,
                website_url: String,
                vcs_url: String,
                issue_tracker_url: String,
                github_repo: Option<String>,
                github_release_notes_file: Option<String>,
                maturity: PackageMaturity,
                created: String,
                updated: String,
                versions: Vec<String>,
            }

            let mut resp: GetPackageResp = response.json()?;
            resp.labels.sort();
            resp.licenses.sort();
            resp.versions.sort_by(|ref a, ref b| {
                match VersionCompare::compare(a, b) {
                    Ok(CompOp::Lt) => Ordering::Less,
                    Ok(CompOp::Eq) => Ordering::Equal,
                    Ok(CompOp::Gt) => Ordering::Greater,
                    _              => Ordering::Less,
                }
            });

            debug_assert_eq!(self.subject, resp.owner);
            debug_assert_eq!(self.repository, resp.repo);
            debug_assert_eq!(self.package, resp.name);

            self.desc = resp.desc;
            self.labels = resp.labels;
            self.licenses = resp.licenses;
            self.website_url = resp.website_url;
            self.vcs_url = resp.vcs_url;
            self.issue_tracker_url = resp.issue_tracker_url;
            self.github_repo = resp.github_repo
                .unwrap_or(String::new());
            self.github_release_notes_file = resp.github_release_notes_file
                .unwrap_or(String::new());
            self.maturity = resp.maturity;
            self.created = resp.created.parse::<DateTime<Utc>>().ok();
            self.updated = resp.updated.parse::<DateTime<Utc>>().ok();
            self.versions = Some(resp.versions);

            trace!("{}:\n\
                   - desc: \"{}\"\n\
                   - labels: {:?}\n\
                   - website_url: \"{}\"\n\
                   - vcs_url: \"{}\"\n\
                   - issue_tracker_url: \"{}\"\n\
                   - github_repo: \"{}\"\n\
                   - github_release_notes_file: \"{}\"\n\
                   - maturity: {:?}\n\
                   - created: {}\n\
                   - updated: {}\n\
                   ",
                   self,
                   self.desc,
                   self.labels,
                   self.website_url,
                   self.vcs_url,
                   self.issue_tracker_url,
                   self.github_repo,
                   self.github_release_notes_file,
                   self.maturity,
                   self.created
                   .map_or(String::from("(unknown)"), |d| d.to_string()),
                   self.updated
                   .map_or(String::from("(unknown)"), |d| d.to_string()));

            Ok(self)
        } else {
            #[derive(Deserialize)]
            struct GetPackageError {
                message: String,
            }

            let resp: GetPackageError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn update(&mut self) -> Result<&Self, Error>
    {
        let url = self.client.api_url(
            &format!("/packages/{}/{}/{}",
                     self.subject,
                     self.repository,
                     self.package))?;

        #[derive(Serialize)]
        struct UpdatePackageReq {
            desc: String,
            labels: Vec<String>,
            licenses: Vec<String>,
            website_url: String,
            vcs_url: String,
            issue_tracker_url: String,
            github_repo: String,
            github_release_notes_file: String,
            maturity: PackageMaturity,
        }

        let req = UpdatePackageReq {
            desc: self.desc.clone(),
            labels: self.labels.clone(),
            licenses: self.licenses.clone(),
            website_url: self.website_url.clone(),
            vcs_url: self.vcs_url.clone(),
            issue_tracker_url: self.issue_tracker_url.clone(),
            github_repo: self.github_repo.clone(),
            github_release_notes_file: self.github_release_notes_file.clone(),
            maturity: self.maturity.clone(),
        };

        let mut response = self.client
            .patch(url)
            .json(&req)
            .send()?;

        if response.status().is_success() {
            /* Bintray doesn't return the new `updated` value. So clear
             * it to be sure the caller doesn't assume the value is
             * up-to-date. */
            self.updated = None;

            Ok(self)
        } else {
            #[derive(Deserialize)]
            struct UpdatePackageError {
                message: String,
            }

            let resp: UpdatePackageError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn delete(&self) -> Result<(), Error>
    {
        let url = self.client.api_url(
            &format!("/packages/{}/{}/{}",
                     self.subject,
                     self.repository,
                     self.package))?;

        let mut response = self.client
            .delete(url)
            .send()?;

        if response.status().is_success() {
            Ok(())
        } else {
            #[derive(Deserialize)]
            struct DeletePackageError {
                message: String,
            }

            let resp: DeletePackageError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn get_name(&self) -> &str               { &self.package }
    pub fn get_repository(&self) -> &str         { &self.repository }
    pub fn get_subject(&self) -> &str            { &self.subject }
    pub fn get_desc(&self) -> &str               { &self.desc }
    pub fn get_labels(&self) -> &Vec<String>     { &self.labels }
    pub fn get_licenses(&self) -> &Vec<String>   { &self.licenses }
    pub fn get_website_url(&self) -> &str        { &self.website_url }
    pub fn get_vcs_url(&self) -> &str            { &self.vcs_url }
    pub fn get_issue_tracker_url(&self) -> &str  { &self.issue_tracker_url }
    pub fn get_github_repo(&self) -> &str        { &self.github_repo }
    pub fn get_github_release_notes_file(&self) -> &str
    {
        &self.github_release_notes_file
    }
    pub fn get_maturity(&self) -> &PackageMaturity { &self.maturity }
    pub fn get_created(&self) -> &Option<DateTime<Utc>> { &self.created }
    pub fn get_updated(&self) -> &Option<DateTime<Utc>> { &self.updated }

    pub fn versions(&self) -> Result<Vec<String>, Error>
    {
        match self.versions {
            Some(ref versions) => Ok(versions.clone()),
            None               => throw!(BintrayError::CallGetFirst),
        }
    }

    pub fn version(&self, version_string: &str) -> Version
    {
        Version::new(&self.client,
                     &self.subject,
                     &self.repository,
                     &self.package,
                     version_string)
    }
}

impl fmt::Display for Package {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "bintray::Package({}:{}:{})",
            self.subject,
            self.repository,
            self.package)
    }
}
