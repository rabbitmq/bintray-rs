extern crate reqwest;
extern crate sha2;
extern crate tempfile;

extern crate bintray;

use bintray::{Client, BintrayError, RepositoryType, Version};
use reqwest::StatusCode;
use sha2::{Sha256, Digest};
use std::fs::{self, File};
use std::io::{Read, Result, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::{tempdir, TempDir};

#[allow(dead_code)]
mod util;

pub static MYAPP_PACKAGE: &'static str = "myapp";
pub static MYAPP_GENERIC_VERSION: &'static str = "1.0";
pub static MYAPP_DEBIAN_VERSION: &'static str = "1.0-1";
pub static MYAPP_RPM_VERSION: &'static str = "1.0-1";
pub static MYAPP_MAVEN_VERSION: &'static str = "1.0";

#[test]
fn upload_and_download_using_file() {
    util::init_env_logger();

    match util::get_credentials() {
        Some((username, api_key)) => {
            let client = Client::new().unwrap()
                .user(&username, &api_key);

            let temp_dir = tempdir().unwrap();
            let source_filename = source_text_filename();
            let dest_filename = dest_text_filename(&temp_dir);
            let remote_filename = PathBuf::from("preexisting")
                .join(dest_filename.file_name().unwrap());

            let temp_version = create_temporary_package_and_version(
                &client, &RepositoryType::Generic, None);

            client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, None)
                .unwrap()
                .checksum_from_file(&source_filename).unwrap()
                .upload_from_file(&source_filename).unwrap()
                .wait_for_availability(Duration::from_secs(30)).unwrap();

            client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, None)
                .unwrap()
                .download_to_file(&dest_filename).unwrap();

            assert_files_eq(&source_filename, &dest_filename);
            temp_dir.close().unwrap();

            client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, None)
                .unwrap()
                .delete().unwrap();

            delete_temporary_package_and_version(&client, &temp_version);
        }
        None => {
            // Skipped.
        }
    }
}

#[test]
fn upload_and_download_using_reader_and_writer() {
    util::init_env_logger();

    match util::get_credentials() {
        Some((username, api_key)) => {
            let client = Client::new().unwrap()
                .user(&username, &api_key);

            let temp_dir = tempdir().unwrap();
            let source_filename = source_text_filename();
            let dest_filename = dest_text_filename(&temp_dir);
            let remote_filename = PathBuf::from("preexisting")
                .join(dest_filename.file_name().unwrap());

            let temp_version = create_temporary_package_and_version(
                &client, &RepositoryType::Generic, None);

            let reader = TrackingProgress::open(&source_filename).unwrap();
            let mut writer = TrackingProgress::create(&dest_filename).unwrap();

            let mut upload_content = client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, None)
                .unwrap();
            let download_content = upload_content.clone();

            upload_content
                .set_checksum_from_file(&source_filename).unwrap()
                .upload_from_reader(reader).unwrap()
                .wait_for_availability(Duration::from_secs(30)).unwrap();

            download_content
                .download_to_writer(&mut writer).unwrap();

            assert_files_eq(&source_filename, &dest_filename);
            temp_dir.close().unwrap();

            upload_content
                .delete().unwrap();

            delete_temporary_package_and_version(&client, &temp_version);
        }
        None => {
            // Skipped.
        }
    }
}

#[test]
fn upload_and_download_using_reqwest_response() {
    util::init_env_logger();

    match util::get_credentials() {
        Some((username, api_key)) => {
            let client = Client::new().unwrap()
                .user(&username, &api_key);

            let temp_dir = tempdir().unwrap();
            let source_filename = source_text_filename();
            let dest_filename = dest_text_filename(&temp_dir);
            let remote_filename = PathBuf::from("preexisting")
                .join(dest_filename.file_name().unwrap());

            let temp_version = create_temporary_package_and_version(
                &client, &RepositoryType::Generic, None);

            let reader = TrackingProgress::open(&source_filename).unwrap();
            let mut writer = TrackingProgress::create(&dest_filename).unwrap();

            let mut upload_content = client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, None)
                .unwrap();
            let download_content = upload_content.clone();

            upload_content
                .set_checksum_from_file(&source_filename).unwrap()
                .upload_from_reader(reader).unwrap()
                .wait_for_availability(Duration::from_secs(30)).unwrap();

            let mut response = download_content
                .download().unwrap();

            let checksum = bintray::checksum_from_response(&response)
                .unwrap();
            let size = bintray::content_size_from_response(&response)
                .unwrap();
            writer.set_size(size);
            response.copy_to(&mut writer).unwrap();

            assert_eq!(checksum, writer.get_checksum());
            assert_files_eq(&source_filename, &dest_filename);
            temp_dir.close().unwrap();

            download_content
                .delete().unwrap();

            delete_temporary_package_and_version(&client, &temp_version);
        }
        None => {
            // Skipped.
        }
    }
}

#[test]
fn wait_for_file_to_be_published() {
    util::init_env_logger();

    match util::get_credentials() {
        Some((username, api_key)) => {
            let client = Client::new().unwrap()
                .user(&username, &api_key);
            let anon_client = Client::new().unwrap();

            let temp_dir = tempdir().unwrap();
            let source_filename = source_text_filename();
            let dest_filename = dest_text_filename(&temp_dir);
            let remote_filename = PathBuf::from("preexisting")
                .join(dest_filename.file_name().unwrap());

            let temp_version = create_temporary_package_and_version(
                &client, &RepositoryType::Generic, None);

            let mut upload_content = client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, None)
                .unwrap()
                .checksum_from_file(&source_filename).unwrap();

            upload_content
                .upload_from_file(&source_filename).unwrap();

            let mut download_content = anon_client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, None)
                .unwrap();

            assert!(!download_content.exists().unwrap());

            let error = download_content
                .wait_for_availability(Duration::from_secs(10))
                .unwrap_err()
                .downcast::<BintrayError>()
                .unwrap();
            match error {
                BintrayError::ContentNotAvailable { reqwest_error: e } => {
                    assert!(e.is_none());
                }
                _ => { panic!("Not the error we expect: {:?}", error); }
            }

            let status = download_content
                .download_to_file(&dest_filename)
                .unwrap_err()
                .downcast::<reqwest::Error>()
                .unwrap()
                .status()
                .unwrap();
            assert!(status == StatusCode::NotFound ||
                    status == StatusCode::Unauthorized);

            upload_content
                .set_publish_flag(true)
                .upload_from_file(&source_filename).unwrap();

            download_content
                .wait_for_availability(Duration::from_secs(30))
                .unwrap();

            assert!(download_content.exists().unwrap());

            download_content
                .download_to_file(&dest_filename).unwrap();

            assert_files_eq(&source_filename, &dest_filename);
            temp_dir.close().unwrap();

            upload_content
                .delete().unwrap();

            delete_temporary_package_and_version(&client, &temp_version);
        }
        None => {
            // Skipped.
        }
    }
}

#[test]
fn upload_and_download_debian_package() {
    util::init_env_logger();

    match util::get_credentials() {
        Some((username, api_key)) => {
            let client = Client::new().unwrap()
                .user(&username, &api_key);

            let repo_type = RepositoryType::Debian;
            let temp_version = create_temporary_package_and_version(
                &client, &repo_type, None);

            let source_filename = source_debian_package_filename();
            let remote_filename = PathBuf::from("pool")
                .join(source_filename.file_name().unwrap());

            client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, Some(&repo_type))
                .unwrap()
                .publish_flag(true)
                .debian_distributions(&["stretch"])
                .debian_components(&["main"])
                .debian_architectures(&["amd64"])
                .checksum_from_file(&source_filename).unwrap()
                .upload_from_file(&source_filename).unwrap()
                .wait_for_availability(Duration::from_secs(30)).unwrap()
                .wait_for_indexation(Duration::from_secs(20 * 60)).unwrap();

            client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, Some(&repo_type))
                .unwrap()
                .delete().unwrap();

            delete_temporary_package_and_version(&client, &temp_version);
        }
        None => {
            // Skipped.
        }
    }
}

#[test]
fn upload_and_download_rpm_package() {
    util::init_env_logger();

    match util::get_credentials() {
        Some((username, api_key)) => {
            let client = Client::new().unwrap()
                .user(&username, &api_key);

            let repo_type = RepositoryType::Rpm;
            let yum_depth = 3;
            let temp_version = create_temporary_package_and_version(
                &client, &repo_type, Some(yum_depth));

            let dist_name = "el";
            let dist_version = "7";
            let dist_arch = "noarch";

            let source_filename = source_rpm_package_filename();
            let remote_filename =
                PathBuf::from(format!("{}/{}/{}/{}",
                                      temp_version.get_package(),
                                      dist_name,
                                      dist_version,
                                      dist_arch))
                .join(source_filename.file_name().unwrap());

            client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, Some(&repo_type))
                .unwrap()
                .publish_flag(true)
                .checksum_from_file(&source_filename).unwrap()
                .upload_from_file(&source_filename).unwrap()
                .wait_for_availability(Duration::from_secs(30)).unwrap()
                .wait_for_indexation(Duration::from_secs(20 * 60)).unwrap();

            client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, Some(&repo_type))
                .unwrap()
                .delete().unwrap();

            delete_temporary_package_and_version(&client, &temp_version);
        }
        None => {
            // Skipped.
        }
    }
}

#[test]
fn upload_and_download_maven_package() {
    util::init_env_logger();

    match util::get_credentials() {
        Some((username, api_key)) => {
            let client = Client::new().unwrap()
                .user(&username, &api_key);

            let repo_type = RepositoryType::Maven;
            let temp_version = create_temporary_package_and_version(
                &client, &repo_type, None);

            let source_filename = source_maven_package_filename();
            let remote_filename =
                PathBuf::from(format!("io/pivotal/bintray_rs/{}/{}",
                                      temp_version.get_package(),
                                      temp_version.get_version()))
                .join(source_filename.file_name().unwrap());

            client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, Some(&repo_type))
                .unwrap()
                .publish_flag(true)
                .checksum_from_file(&source_filename).unwrap()
                .upload_from_file(&source_filename).unwrap()
                .wait_for_availability(Duration::from_secs(30)).unwrap();
                // FIXME .wait_for_indexation(Duration::from_secs(20 * 60)).unwrap();

            client
                .subject(util::SUBJECT)
                .repository(temp_version.get_repository())
                .package(temp_version.get_package())
                .version(temp_version.get_version())
                .file(&remote_filename, Some(&repo_type))
                .unwrap()
                .delete().unwrap();

            delete_temporary_package_and_version(&client, &temp_version);
        }
        None => {
            // Skipped.
        }
    }
}

fn create_temporary_package_and_version(client: &Client,
                                        repo_type: &RepositoryType,
                                        yum_depth: Option<usize>)
    -> Version
{
    let name = util::random_name(16);

    let mut repo = client
        .subject(util::SUBJECT)
        .repository(&name)
        .repo_type(repo_type);

    if repo_type == &RepositoryType::Rpm {
        if let Some(yum_depth) = yum_depth {
            repo.set_yum_metadata_depth(yum_depth);
        }
    }

    repo = repo
        .create()
        .unwrap();

    let package = repo
        .package(MYAPP_PACKAGE)
        .licenses(util::LICENSES)
        .vcs_url(util::VCS_URL)
        .create()
        .unwrap();

    let version_number = match repo_type {
        RepositoryType::Debian => MYAPP_DEBIAN_VERSION,
        RepositoryType::Rpm    => MYAPP_RPM_VERSION,
        RepositoryType::Maven  => MYAPP_MAVEN_VERSION,
        _                      => MYAPP_GENERIC_VERSION,
    };

    let mut version = package
        .version(version_number);

    if repo_type != &RepositoryType::Maven {
        version = version
            .create()
            .unwrap();
    }

    version
}

fn delete_temporary_package_and_version(client: &Client, version: &Version)
{
    client
        .subject(util::SUBJECT)
        .repository(version.get_repository())
        .delete()
        .unwrap();
}

fn source_text_filename() -> PathBuf
{
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.extend(&["tests", "data", "README.md"]);
    filename
}

fn source_debian_package_filename() -> PathBuf
{
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.extend(&["tests", "data", "myapp_1.0-1_all.deb"]);
    filename
}

fn source_rpm_package_filename() -> PathBuf
{
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.extend(&["tests", "data", "myapp-1.0-1.noarch.rpm"]);
    filename
}

fn source_maven_package_filename() -> PathBuf
{
    let mut filename = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    filename.extend(&["tests", "data", "myapp-1.0.jar"]);
    filename
}

fn dest_text_filename(dir: &TempDir) -> PathBuf
{
    let mut filename = dir.path().join(util::random_name(8));
    filename.set_extension("txt");
    filename
}

fn assert_files_eq<P: AsRef<Path>>(left: P, right: P)
{
    let left_buf = fs::read(left).unwrap();
    let right_buf = fs::read(right).unwrap();

    assert!(!left_buf.is_empty());
    assert!(!right_buf.is_empty());
    assert_eq!(left_buf, right_buf);
}

struct TrackingProgress {
    file: File,
    size: u64,
    read: usize,
    written: usize,
    digest: Sha256,
}

impl TrackingProgress {
    fn open<P: AsRef<Path>>(path: P) -> Result<TrackingProgress>
    {
        let file = File::open(path)?;
        let size = file.metadata()?.len();

        Ok(TrackingProgress {
            file: file,
            size: size,
            read: 0,
            written: 0,
            digest: Sha256::new(),
        })
    }

    fn create<P: AsRef<Path>>(path: P) -> Result<TrackingProgress>
    {
        let file = File::create(path)?;

        Ok(TrackingProgress {
            file: file,
            size: 0,
            read: 0,
            written: 0,
            digest: Sha256::new(),
        })
    }

    fn get_checksum(self) -> Vec<u8>
    {
        let mut ret = self.digest.result();
        Vec::from(ret.as_mut_slice())
    }

    fn set_size(&mut self, size: u64)
    {
        self.size = size;
    }
}

impl Read for TrackingProgress {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>
    {
        let read = self.file.read(buf)?;
        self.read += read;
        self.digest.input(&buf);

        println!("Upload: {}/{} bytes", self.read, self.size);

        Ok(read)
    }
}

impl Write for TrackingProgress {
    fn write(&mut self, buf: &[u8]) -> Result<usize>
    {
        let written = self.file.write(buf)?;
        self.written += written;
        self.digest.input(&buf);

        match self.size {
            0 => println!("Download: {} bytes", self.written),
            _ => println!("Download: {}/{} bytes", self.written, self.size),
        }

        Ok(written)
    }

    fn flush(&mut self) -> Result<()>
    {
        self.file.flush()
    }
}
