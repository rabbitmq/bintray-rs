use failure::Error;
use itertools::Itertools;
use libflate::gzip;
use reqwest::{Body, Method, Response, StatusCode, Url};
use reqwest::header::ContentLength;
use serde_xml_rs;
use sha1::Sha1;
use sha2::{Sha256, Digest};
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf, Component};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use ::{Client, BintrayError, Repository, RepositoryType};

#[derive(Clone, Debug)]
pub struct ContentChecksum {
    sha1: Option<Vec<u8>>,
    sha256: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct Content {
    subject: String,
    repository: String,
    package: String,
    version: String,
    path: PathBuf,

    publish: Option<bool>,
    override_: Option<bool>,
    explode: Option<bool>,
    checksum: ContentChecksum,

    repository_type: RepositoryType,
    debian_distribution: Vec<String>,
    debian_component: Vec<String>,
    debian_architecture: Vec<String>,

    client: Client,
}

enum WaitCheckResult<T> {
    WaitOver(Result<T, Error>),
    TryAgain,
}

header! { (XPublish,  "X-Bintray-Publish")  => [u8] }
header! { (XOverride, "X-Bintray-Override") => [u8] }
header! { (XExplode,  "X-Bintray-Explode")  => [u8] }
header! { (XChecksum, "X-Checksum-Sha2")    => [String] }

header! { (XDebianDistribution, "X-Bintray-Debian-Distribution") => [String] }
header! { (XDebianComponent,    "X-Bintray-Debian-Component")    => [String] }
header! { (XDebianArchitecture, "X-Bintray-Debian-Architecture") => [String] }

impl Content {
    pub fn new<T: AsRef<Path>>(client: &Client,
                               subject: &str,
                               repository: &str,
                               package: &str,
                               version: &str,
                               path: T,
                               repo_type: Option<&RepositoryType>)
        -> Result<Self, Error>
    {
        let cleaned_path = clean_path(path);

        let actual_repo_type = match repo_type {
            Some(ref value) => (*value).clone(),
            None => {
                let repository = Repository::new(client,
                                                 subject,
                                                 repository)
                    .get()?;

                repository.get_type().clone()
            }
        };

        let content = Content {
            subject: String::from(subject),
            repository: String::from(repository),
            package: String::from(package),
            version: String::from(version),
            path: cleaned_path,

            publish: None,
            override_: None,
            explode: None,
            checksum: ContentChecksum {
                sha1: None,
                sha256: None,
            },

            repository_type: actual_repo_type,
            debian_distribution: vec![],
            debian_component: vec![],
            debian_architecture: vec![],

            client: client.clone(),
        };

        Ok(content)
    }

    pub fn publish_flag(mut self, publish: bool) -> Self
    {
        self.set_publish_flag(publish);
        self
    }

    pub fn set_publish_flag(&mut self, publish: bool) -> &mut Self
    {
        self.publish = Some(publish);
        self
    }

    pub fn override_flag(mut self, override_: bool) -> Self
    {
        self.set_override_flag(override_);
        self
    }

    pub fn set_override_flag(&mut self, override_: bool) -> &mut Self
    {
        self.override_ = Some(override_);
        self
    }

    pub fn explode_flag(mut self, explode: bool) -> Self
    {
        self.set_explode_flag(explode);
        self
    }

    pub fn set_explode_flag(&mut self, explode: bool) -> &mut Self
    {
        self.explode = Some(explode);
        self
    }

    pub fn checksum_sha1(mut self, checksum: &[u8]) -> Self
    {
        self.set_checksum_sha1(checksum);
        self
    }

    pub fn set_checksum_sha1(&mut self, checksum: &[u8]) -> &mut Self
    {
        self.checksum.sha1 = Some(Vec::from(checksum));
        self
    }

    pub fn checksum_sha256(mut self, checksum: &[u8]) -> Self
    {
        self.set_checksum_sha256(checksum);
        self
    }

    pub fn set_checksum_sha256(&mut self, checksum: &[u8]) -> &mut Self
    {
        self.checksum.sha256 = Some(Vec::from(checksum));
        self
    }

    pub fn checksum_from_file<P: AsRef<Path>>(mut self, filename: P)
        -> Result<Self, Error>
        {
            self.set_checksum_from_file(filename)?;
            Ok(self)
        }

    pub fn set_checksum_from_file<P: AsRef<Path>>(&mut self, filename: P)
        -> Result<&mut Self, Error>
    {
        let mut file = File::open(filename)?;

        let mut sha1 = Sha1::default();
        let mut sha256 = Sha256::default();

        let mut buffer = [0u8; 1024];
        loop {
            let bytes_read = file.read(&mut buffer)?;
            sha1.input(&buffer[..bytes_read]);
            sha256.input(&buffer[..bytes_read]);
            if bytes_read == 0 {
                break;
            }
        }

        self.set_checksum_sha1(&sha1.result());
        self.set_checksum_sha256(&sha256.result());

        Ok(self)
    }

    pub fn debian_distributions<T: AsRef<str>>(mut self, distributions: &[T])
        -> Self
    {
        self.set_debian_distributions(distributions);
        self
    }

    pub fn set_debian_distributions<T: AsRef<str>>(&mut self,
                                                   distributions: &[T])
        -> &mut Self
    {
        let mut vec: Vec<String> = distributions
            .iter()
            .map(|s| s.as_ref().to_owned())
            .collect();
        vec.sort();

        self.debian_distribution = vec;
        self
    }

    pub fn debian_components<T: AsRef<str>>(mut self, components: &[T])
        -> Self
    {
        self.set_debian_components(components);
        self
    }

    pub fn set_debian_components<T: AsRef<str>>(&mut self,
                                                   components: &[T])
        -> &mut Self
    {
        let mut vec: Vec<String> = components
            .iter()
            .map(|s| s.as_ref().to_owned())
            .collect();
        vec.sort();

        self.debian_component = vec;
        self
    }

    pub fn debian_architectures<T: AsRef<str>>(mut self, architectures: &[T])
        -> Self
    {
        self.set_debian_architectures(architectures);
        self
    }

    pub fn set_debian_architectures<T: AsRef<str>>(&mut self,
                                                   architectures: &[T])
        -> &mut Self
    {
        let mut vec: Vec<String> = architectures
            .iter()
            .map(|s| s.as_ref().to_owned())
            .collect();
        vec.sort();

        self.debian_architecture = vec;
        self
    }

    pub fn upload_from_file<P: AsRef<Path>>(&mut self, filename: P)
        -> Result<&mut Self, Error>
    {
        let file = File::open(filename)?;
        let size = file.metadata()?.len();
        let body = Body::sized(file, size);

        self.upload_from_body(body)
    }

    pub fn upload_from_reader<R: Read + Send + 'static>(&mut self, reader: R)
        -> Result<&mut Self, Error>
    {
        let body = Body::new(reader);

        self.upload_from_body(body)
    }

    fn upload_from_body(&mut self, body: Body) -> Result<&mut Self, Error>
    {
        /*
         * The URL to use depends on the package type: for Maven
         * uploads, Bintray uses a different URLs than other
         * packages.
         */
        let url = match self.repository_type {
            RepositoryType::Maven => {
                self.client.api_url(
                    &format!("/maven/{}/{}/{}/{}",
                             self.subject,
                             self.repository,
                             self.package,
                             self.path.to_string_lossy()))?
            }
            _ => {
                self.client.api_url(
                    &format!("/content/{}/{}/{}/{}/{}",
                             self.subject,
                             self.repository,
                             self.package,
                             self.version,
                             self.path.to_string_lossy()))?
            }
        };

        trace!("{} upload: URL: {}", self, url);

        let mut builder = self.client.put(url);

        /*
         * Now the few flags accepted by Bintray. According to
         * the documentation, Maven packages only support the
         * `publish` flag.
         */
        match self.publish {
            Some(flag) => {
                trace!("{} upload: publish: {}", self, flag);
                let header = XPublish(bool_to_int(flag));
                builder.header(header);
            }
            None => {}
        }

        match self.repository_type {
            RepositoryType::Maven => {}
            _ => {
                match self.override_ {
                    Some(flag) => {
                        trace!("{} upload: override: {}", self, flag);
                        let header = XOverride(bool_to_int(flag));
                        builder.header(header);
                    }
                    None => {}
                }
                match self.explode {
                    Some(flag) => {
                        trace!("{} upload: explode: {}", self, flag);
                        let header = XExplode(bool_to_int(flag));
                        builder.header(header);
                    }
                    None => {}
                }
            }
        }

        match self.checksum.sha256 {
            Some(ref checksum) => {
                let value = checksum_to_string(checksum);
                trace!("{} upload: checksum: {}", self, value);
                let header = XChecksum(value);
                builder.header(header);
            }
            None => {}
        }

        /*
         * Finally, for Debian packages, we have to specify
         * several attributes so that Bintray can index the
         * package properly.
         */
        match self.repository_type {
            RepositoryType::Debian => {
                let value = self.debian_distribution.join(",");
                trace!("{} upload: debian distributions: {:?}",
                       self, value);
                let header = XDebianDistribution(value);
                builder.header(header);

                let value = self.debian_component.join(",");
                trace!("{} upload: debian components: {:?}",
                       self, value);
                let header = XDebianComponent(value);
                builder.header(header);

                let value = self.debian_architecture.join(",");
                trace!("{} upload: debian architectures: {:?}",
                       self, value);
                let header = XDebianArchitecture(value);
                builder.header(header);
            }
            _ => {}
        }

        /* Ready to upload! */
        let mut response = builder
            .body(body)
            .send()?;

        if response.status().is_success() {
            Ok(self)
        } else {
            #[derive(Deserialize)]
            struct UploadContentError {
                message: String,
            }

            let resp: UploadContentError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn download_to_file<P: AsRef<Path>>(&self, filename: P)
        -> Result<u64, Error>
    {
        let mut file = File::create(filename)?;
        self.download_to_writer(&mut file)
    }

    pub fn download_to_writer<W: ?Sized>(&self, writer: &mut W)
        -> Result<u64, Error>
        where W: Write
    {
        let mut response = self.download()?;

        let size = response.copy_to(writer)?;
        Ok(size)
    }

    pub fn download(&self) -> Result<Response, Error>
    {
        let url = self.client.dl_url(
            &format!("/{}/{}/{}",
                     self.subject,
                     self.repository,
                     self.path.to_string_lossy()))?;

        trace!("{} download: URL: {}", self, url);

        let response = self.client
            .get(url)
            .send()?
            .error_for_status()?;

        Ok(response)
    }

    pub fn exists(&mut self) -> Result<bool, Error>
    {
        let url = self.client.dl_url(
            &format!("/{}/{}/{}",
                     self.subject,
                     self.repository,
                     self.path.to_string_lossy()))?;

        let response = self.client
            .head(url)
            .send()?;

        if response.status().is_success() {
            let checksum = checksum_from_response(&response);

            if self.checksum.sha256.is_some() {
                Ok(checksum == self.checksum.sha256)
            } else {
                self.checksum.sha256 = checksum;
                Ok(true)
            }
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

    pub fn wait_for_availability(&mut self, timeout: Duration)
        -> Result<&Self, Error>
    {
        let displayed_content = format!("{}", self);

        let url = self.client.dl_url(
            &format!("/{}/{}/{}",
                     self.subject,
                     self.repository,
                     self.path.to_string_lossy()))?;
        trace!("{} availability: URL: {}", displayed_content, url);

        let known_checksum = self.checksum.sha256.clone();

        let check = move |response: Response| {
            trace!("{} availability: Response: {}",
                   displayed_content, response.status());

            if response.status().is_success() {
                /* Content available! */
                let checksum = checksum_from_response(&response);

                if known_checksum.is_some() {
                    if checksum == known_checksum {
                        return WaitCheckResult::WaitOver(Ok(checksum));
                    } else {
                        return WaitCheckResult::TryAgain;
                    }
                } else {
                    return WaitCheckResult::WaitOver(Ok(checksum));
                }
            }

            match response.status() {
                StatusCode::Unauthorized |
                StatusCode::NotFound => {
                    /* Not available yet, try again. */
                    WaitCheckResult::TryAgain
                }
                _ => {
                    /* Unexpected error => bail out. */
                    let error =
                        BintrayError::ContentNotAvailable {
                            reqwest_error:
                                Some(response.error_for_status()
                                     .unwrap_err())
                        };
                    WaitCheckResult::WaitOver(into_err!(error))
                }
            }
        };

        let ret = self.wait_for_condition(Method::Head,
                                          url,
                                          check,
                                          Duration::from_secs(1),
                                          timeout);
        match ret {
            Ok(Some(checksum)) => {
                self.checksum.sha256 = Some(checksum);
                Ok(self)
            }
            Ok(None) => {
                throw!(BintrayError::ContentChecksumNotReturned);
            }
            Err(error) => {
                Err(error)
            }
        }
    }

    pub fn wait_for_indexation(&self, timeout: Duration)
        -> Result<&Self, Error>
    {
        if !self.repository_type.is_indexed() {
            throw!(BintrayError::OnlyForIndexedPackages);
        }

        let mut remaining = timeout.clone();

        match self.repository_type {
            RepositoryType::Debian => {
                for distribution in &self.debian_distribution {
                    for component in &self.debian_component {
                        for architecture in &self.debian_architecture {
                            let t0 = Instant::now();

                            let ret = self.wait_for_debian_indexation_in(
                                &distribution,
                                &component,
                                &architecture,
                                remaining);

                            if ret.is_err() {
                                return ret;
                            }

                            remaining -= t0.elapsed();
                        }
                    }
                }
            }

            RepositoryType::Rpm => {
                let repository = self.client
                    .subject(&self.subject)
                    .repository(&self.repository)
                    .get()?;

                let yum_metadata_depth = repository
                    .get_yum_metadata_depth()
                    .unwrap_or(0);

                let ret = self.wait_for_rpm_indexation_in(
                    yum_metadata_depth,
                    remaining);

                if ret.is_err() {
                    return ret;
                }
            }

            _ => {
                panic!("This should have been caught by an earlier if()");
            }
        }

        Ok(self)
    }

    fn wait_for_debian_indexation_in(&self,
                                     distribution: &str,
                                     component: &str,
                                     architecture: &str,
                                     timeout: Duration)
        -> Result<&Self, Error>
    {
        if self.checksum.sha256.is_none() {
            throw!(BintrayError::ContentChecksumRequired);
        }

        let displayed_content = format!("{}", self);

        let url = self.client.dl_url(
            &format!("/{}/{}/dists/{}/{}/binary-{}/Packages",
                     self.subject,
                     self.repository,
                     distribution,
                     component,
                     architecture))?;
        trace!("{} indexation: URL: {}", displayed_content, url);

        let checksum = match self.checksum.sha256 {
            Some(ref checksum) => checksum_to_string(checksum),
            None => panic!("This function should have aborted earlier"),
        };
        let checksum_line = format!("SHA256: {}", checksum);
        trace!("{} indexation: Looking for \"{}\"", self, checksum_line);

        let check = move |mut response: Response| {
            trace!("{} indexation: Response: {}",
                   displayed_content, response.status());

            if response.status().is_success() {
                match response.text() {
                    Ok(packages_file) => {
                        let found = packages_file
                            .lines()
                            .any(|line| line == checksum_line);

                        if found {
                            return WaitCheckResult::WaitOver(Ok(()));
                        } else {
                            return WaitCheckResult::TryAgain;
                        }
                    }
                    Err(error) => {
                        return WaitCheckResult::WaitOver(into_err!(error));
                    }
                }
            }

            match response.status() {
                StatusCode::Unauthorized |
                StatusCode::NotFound => {
                    /* Not available yet, try again. */
                    WaitCheckResult::TryAgain
                }
                _ => {
                    /* Unexpected error => bail out. */
                    let error =
                        BintrayError::ContentNotAvailable {
                            reqwest_error:
                                Some(response.error_for_status()
                                     .unwrap_err())
                        };
                    WaitCheckResult::WaitOver(into_err!(error))
                }
            }
        };

        let ret = self.wait_for_condition(Method::Get,
                                          url,
                                          check,
                                          Duration::from_secs(30),
                                          timeout);
        match ret {
            Ok(()) => Ok(self),
            Err(error) => Err(error),
        }
    }

    fn wait_for_rpm_indexation_in(&self,
                                  yum_metadata_depth: usize,
                                  timeout: Duration)
        -> Result<&Self, Error>
    {
        if self.checksum.sha1.is_none() {
            throw!(BintrayError::ContentChecksumRequired);
        }

        let displayed_content = format!("{}", self);

        let repodata_url = if yum_metadata_depth > 0 {
            let yum_metadata_root = PathBuf::from(
                self.path.components()
                .take(yum_metadata_depth)
                .collect::<PathBuf>());

            self.client.dl_url(
                &format!("/{}/{}/{}/",
                         self.subject,
                         self.repository,
                         yum_metadata_root.to_string_lossy()))?
        } else {
            self.client.dl_url(
                &format!("/{}/{}/",
                         self.subject,
                         self.repository))?
        };

        let repomd_xml_url = repodata_url.join("repodata/repomd.xml")?;
        trace!("{} indexation: repomd.xml URL: {}",
               displayed_content, repomd_xml_url);

        let checksum = match self.checksum.sha1 {
            Some(ref checksum) => checksum_to_string(checksum),
            None => panic!("This function should have aborted earlier"),
        };

        /* Structure of repomd.xml. */
        #[derive(Deserialize)]
        struct RepomdDataLocation {
            href: String,
        }

        #[derive(Deserialize)]
        struct RepomdData {
            #[serde(rename = "type")]
            type_: String,
            location: RepomdDataLocation,
        }

        #[derive(Deserialize)]
        struct Repomd {
            data: Vec<RepomdData>,
        }

        /* Structure of *-primary.xml. */
        #[derive(Deserialize)]
        struct MetadataPackageVersion {
            ver: String,
            rel: String,
            epoch: String,
        }

        #[derive(Deserialize)]
        struct MetadataPackageChecksum {
            #[serde(rename = "type")]
            type_: String,
            #[serde(rename = "$value")]
            checksum: String,
        }

        #[derive(Deserialize)]
        struct MetadataPackage {
            name: String,
            arch: String,
            version: MetadataPackageVersion,
            checksum: MetadataPackageChecksum,
        }

        #[derive(Deserialize)]
        struct Metadata {
            package: Vec<MetadataPackage>,
        }

        fn package_filename(md: &MetadataPackage) -> String
        {
            let filename = if md.version.epoch == "0" {
                format!("{}-{}-{}.{}.rpm",
                        md.name,
                        md.version.ver,
                        md.version.rel,
                        md.arch)
            } else {
                format!("{}:{}-{}-{}.{}.rpm",
                        md.version.epoch,
                        md.name,
                        md.version.ver,
                        md.version.rel,
                        md.arch)
            };
            trace!("Indexed package filename: {}", filename);
            filename
        }

        let client = self.client.clone();
        let filename = format!("{}", self.path
                               .file_name()
                               .unwrap()
                               .to_string_lossy());

        let check = move |response: Response| {
            trace!("{} indexation: Response: {}",
                   displayed_content, response.status());

            if response.status().is_success() {
                let repomd: Repomd =
                    match serde_xml_rs::deserialize(response) {
                        Ok(value) =>
                            value,
                        Err(error) =>
                            return WaitCheckResult::WaitOver(
                                into_err!(error)),
                    };

                let primary_entry = repomd.data
                    .iter()
                    .find(|d| d.type_ == "primary");
                let primary_url = match primary_entry {
                    Some(value) =>
                        match repodata_url.join(&value.location.href) {
                            Ok(value) =>
                                value,
                            Err(error) =>
                                return WaitCheckResult::WaitOver(
                                    into_err!(error)),
                        }
                    None =>
                        return WaitCheckResult::TryAgain,
                };
                trace!("{} indexation: primary.xml URL: {}",
                       displayed_content, primary_url);

                let ret = client
                    .request(Method::Get, primary_url.clone())
                    .send();

                match ret {
                    Ok(response) => {
                        let gzip_reader = match gzip::Decoder::new(response) {
                            Ok(value) =>
                                value,
                            Err(error) =>
                                return WaitCheckResult::WaitOver(
                                    into_err!(error)),
                        };
                        let metadata: Metadata =
                            match serde_xml_rs::deserialize(gzip_reader) {
                                Ok(value) =>
                                    value,
                                Err(error) =>
                                    return WaitCheckResult::WaitOver(
                                        into_err!(error)),
                            };

                        let package = metadata.package
                            .iter()
                            .find(|p| package_filename(&p) == filename);

                        match package {
                            Some(package) => {
                                trace!("{} indexation: Package `{}` listed",
                                       displayed_content, filename);
                                trace!("{} indexation: Checksum: {}/{}",
                                       displayed_content,
                                       package.checksum.type_,
                                       package.checksum.checksum);
                                if package.checksum.type_ != "sha" {
                                    let error =
                                        BintrayError::RpmRepoChecksumUnsupported;
                                    return WaitCheckResult::WaitOver(into_err!(error))
                                }

                                if package.checksum.checksum == checksum {
                                    return WaitCheckResult::WaitOver(Ok(()));
                                } else {
                                    return WaitCheckResult::TryAgain;
                                }
                            }
                            None => {
                                return WaitCheckResult::TryAgain;
                            }
                        }
                    }
                    Err(_) => {
                        return WaitCheckResult::TryAgain;
                    }
                }
            }

            match response.status() {
                StatusCode::Unauthorized |
                StatusCode::NotFound => {
                    /* Not available yet, try again. */
                    WaitCheckResult::TryAgain
                }
                _ => {
                    /* Unexpected error => bail out. */
                    let error =
                        BintrayError::ContentNotAvailable {
                            reqwest_error:
                                Some(response.error_for_status()
                                     .unwrap_err())
                        };
                    WaitCheckResult::WaitOver(into_err!(error))
                }
            }
        };

        let ret = self.wait_for_condition(Method::Get,
                                          repomd_xml_url,
                                          check,
                                          Duration::from_secs(30),
                                          timeout);
        match ret {
            Ok(()) => Ok(self),
            Err(error) => Err(error),
        }
    }

    fn wait_for_condition<F, T>(&self,
                                method: Method,
                                url: Url,
                                check: F,
                                interval: Duration,
                                timeout: Duration)
        -> Result<T, Error>
        where F: Fn(Response) -> WaitCheckResult<T> + Send + Sync + 'static,
              T: Send + Sync + 'static
    {
        let client = self.client.clone();

        enum WorkerControl {
            Stop,
        };

        let (control_tx, control_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            loop {
                let ret = client
                    .request(method.clone(), url.clone())
                    .send();

                match ret {
                    Ok(response) => {
                        match check(response) {
                            WaitCheckResult::TryAgain => {}
                            WaitCheckResult::WaitOver(result) => {
                                result_tx.send(result).unwrap();
                                return;
                            }
                        }
                    }
                    Err(reqwest_error) => {
                        /* Unexpected error => bail out. */
                        let error = BintrayError::ContentNotAvailable {
                            reqwest_error: Some(reqwest_error)
                        };
                        result_tx.send(into_err!(error)).unwrap();
                        return;
                    }
                }

                match control_rx.recv_timeout(interval) {
                    Ok(WorkerControl::Stop) => {
                        /* Abort. */
                        return;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        /* Loop. */
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        panic!("Control channel disconnected");
                    }
                }
            }
        });

        let result = match result_rx.recv_timeout(timeout) {
            Ok(ret) => {
                ret
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                trace!("{} condition check: <timeout>", self);
                control_tx.send(WorkerControl::Stop).unwrap();
                let error = BintrayError::ContentNotAvailable {
                    reqwest_error: None
                };
                into_err!(error)
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("Result channel disconnected");
            }
        };

        handle.join().unwrap();

        result
    }

    pub fn delete(&self) -> Result<(), Error>
    {
        let url = self.client.api_url(
            &format!("/content/{}/{}/{}",
                     self.subject,
                     self.repository,
                     self.path.to_string_lossy()))?;

        let mut response = self.client
            .delete(url)
            .send()?;

        if response.status().is_success() {
            Ok(())
        } else {
            #[derive(Deserialize)]
            struct DeleteContentError {
                message: String,
            }

            let resp: DeleteContentError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }
}

fn checksum_to_string(checksum: &Vec<u8>) -> String
{
    checksum.iter()
        .format_with("", |item, f| f(&format_args!("{:02x}", item)))
        .to_string()
}

pub fn checksum_from_response(response: &Response) -> Option<Vec<u8>>
{
    match response.headers().get::<XChecksum>() {
        Some(checksum) => {
            let mut bytes = Vec::new();
            for i in 0..(checksum.len()/2) {
                let res = u8::from_str_radix(&checksum[2 * i .. 2 * i + 2], 16);
                match res {
                    Ok(v) => bytes.push(v),
                    Err(_) => return None,
                };
            };

            Some(bytes)
        }
        None => None,
    }
}

pub fn content_size_from_response(response: &Response) -> Option<u64>
{
    response.headers().get::<ContentLength>()
        .map(|ct_len| **ct_len)
}

fn clean_path<T: AsRef<Path>>(path: T) -> PathBuf {
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

fn bool_to_int(flag: bool) -> u8
{
    match flag {
        true  => 1,
        false => 0
    }
}

impl fmt::Display for Content {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "bintray::Content({}:{}:{}:{}:{})",
            self.subject,
            self.repository,
            self.package,
            self.version,
            self.path.display())
    }
}
