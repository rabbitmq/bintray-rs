use reqwest;

#[derive(Debug, Fail)]
pub enum BintrayError {
    #[fail(display = "Bintray API error: {}", message)]
    BintrayApiError {
        message: String
    },

    #[fail(display = "get() must be called first before using this function")]
    CallGetFirst,

    #[fail(display = "Bintray content unavailable")]
    ContentNotAvailable {
        reqwest_error: Option<reqwest::Error>
    },

    #[fail(display = "Content checksum was not returned by Bintray")]
    ContentChecksumNotReturned,

    #[fail(display = "Content checksum must be set")]
    ContentChecksumRequired,

    #[fail(display = "Only for Debian and RPM repositories and packages")]
    OnlyForIndexedPackages,

    #[fail(display = "Only SHA-1 is supported in RPM indexation check")]
    RpmRepoChecksumUnsupported,
}

macro_rules! into_err {
    ($e:expr) => {
        Err(::std::convert::Into::into($e));
    }
}

macro_rules! throw {
    ($e:expr) => {
        return into_err!($e);
    }
}
