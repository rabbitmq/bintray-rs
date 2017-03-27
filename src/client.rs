use hyper::{self, Client, Url};
use hyper::client::{IntoUrl, Body};
use hyper::client::response::Response;
use hyper::error::Result;
use hyper::header::{Authorization, Basic, Header, HeaderFormat, Headers};
use hyper::method::Method;
use hyper::net::HttpsConnector;
use hyper_rustls::TlsClient;
use serde_json;
use std::borrow::Borrow;
use std::{fmt, io, error};

pub struct BintrayClient {
    inner: Client,
    api_base_url: Url,
    dl_base_url: Url,
    username: Option<String>,
    api_key: Option<String>,
}

static BINTRAY_API_BASEURL: &'static str = "https://api.bintray.com/";
static BINTRAY_DL_BASEURL: &'static str = "https://dl.bintray.com/";

pub enum BintrayError {
    Io(io::Error),
    Http(hyper::Error),
    Json(serde_json::error::Error),
}

impl BintrayClient {
    pub fn new(username: Option<String>,
               api_key: Option<String>) -> BintrayClient
    {
        let api_base_url = Url::parse(BINTRAY_API_BASEURL).unwrap();
        let dl_base_url = Url::parse(BINTRAY_DL_BASEURL).unwrap();
        assert_eq!(api_base_url.scheme(), dl_base_url.scheme());

        /* We need to setup a TLS client because we'll use HTTPS. */
        let client = match api_base_url.scheme() {
            "https" => {
                let ssl = TlsClient::new();
                let connector = HttpsConnector::new(ssl);
                Client::with_connector(connector)
            }
            _ => {
                Client::new()
            }
        };

        BintrayClient {
            inner: client,
            api_base_url: api_base_url,
            dl_base_url: dl_base_url,
            username: username,
            api_key: api_key,
        }
    }

    pub fn get_base_url(&self) -> Url {
        self.api_base_url.clone()
    }

    pub fn get_dl_base_url(&self) -> Url {
        self.dl_base_url.clone()
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::Get, url)
    }

    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::Post, url)
    }

    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::Put, url)
    }

    pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::Patch, url)
    }

    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::Delete, url)
    }

    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        let final_url = url.into_url().unwrap();

        let request = self.inner.request(method.clone(), final_url.clone());
        RequestBuilder {
            inner: request,
            username: self.username.clone(),
            password: self.api_key.clone(),

            method: method,
            url: final_url,
        }
    }
}

impl From<io::Error> for BintrayError {
    fn from(error: io::Error) -> BintrayError {
        BintrayError::Io(error)
    }
}

impl From<hyper::Error> for BintrayError {
    fn from(error: hyper::Error) -> BintrayError {
        BintrayError::Http(error)
    }
}

impl From<serde_json::error::Error> for BintrayError {
    fn from(error: serde_json::error::Error) -> BintrayError {
        BintrayError::Json(error)
    }
}

impl fmt::Debug for BintrayError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BintrayError::Io(ref e) => write!(f, "{:?}", e),
            BintrayError::Http(ref e) => write!(f, "{:?}", e),
            BintrayError::Json(ref e) => write!(f, "{:?}", e),
        }
    }
}

impl fmt::Display for BintrayError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BintrayError::Io(ref e) => write!(f, "I/O error: {}", e),
            BintrayError::Http(ref e) => write!(f, "HTTP error: {}", e),
            BintrayError::Json(ref e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl error::Error for BintrayError {
    fn description(&self) -> &str {
        match *self {
            BintrayError::Io(ref e) => e.description(),
            BintrayError::Http(ref e) => e.description(),
            BintrayError::Json(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            BintrayError::Io(ref e) => Some(e),
            BintrayError::Http(ref e) => Some(e),
            BintrayError::Json(ref e) => Some(e),
        }
    }
}

pub struct RequestBuilder<'a> {
    inner: hyper::client::RequestBuilder<'a>,
    username: Option<String>,
    password: Option<String>,

    method: Method,
    url: Url,
}

impl<'a> RequestBuilder<'a> {
    pub fn add_auth_header(mut self) -> RequestBuilder<'a> {
        match self.username.clone() {
            Some(username) => {
                let credentials = Basic {
                    username: username,
                    password: self.password.clone(),
                };
                self.inner = self.inner.header(Authorization(credentials));
            }
            None => { }
        }
        self
    }

    pub fn add_gpg_passphrase(mut self, gpg_passphrase: Option<&str>)
        -> RequestBuilder<'a>
    {
        header! { (XGpgPassphrase, "X-GPG-PASSPHRASE") => [String] }

        match gpg_passphrase {
            Some(gpg_passphrase) => {
                let header = XGpgPassphrase(String::from(gpg_passphrase));
                self.inner = self.inner.header(header);
            }
            None => { }
        }
        self
    }

    pub fn add_debian_architecture<T: Borrow<str>>(
        mut self, debian_architecture: &[T])
        -> RequestBuilder<'a>
    {
        header! { (XDebianArchitecture, "X-Bintray-Debian-Architecture") => [String] }

        if debian_architecture.len() > 0 {
            let header = XDebianArchitecture(debian_architecture.join(","));
            self.inner = self.inner.header(header);
        }

        self
    }

    pub fn add_debian_distribution<T: Borrow<str>>(
        mut self, debian_distribution: &[T])
        -> RequestBuilder<'a>
    {
        header! { (XDebianDistribution, "X-Bintray-Debian-Distribution") => [String] }

        if debian_distribution.len() > 0 {
            let header = XDebianDistribution(debian_distribution.join(","));
            self.inner = self.inner.header(header);
        }

        self
    }

    pub fn add_debian_component<T: Borrow<str>>(
        mut self, debian_component: &[T])
        -> RequestBuilder<'a>
    {
        header! { (XDebianComponent, "X-Bintray-Debian-Component") => [String] }

        if debian_component.len() > 0 {
            let header = XDebianComponent(debian_component.join(","));
            self.inner = self.inner.header(header);
        }

        self
    }

    pub fn headers(mut self, headers: Headers) -> RequestBuilder<'a> {
        self.inner = self.inner.headers(headers);
        self
    }

    pub fn header<H: Header + HeaderFormat>(mut self, header: H) -> RequestBuilder<'a> {
        self.inner = self.inner.header(header);
        self
    }

    pub fn body<B: Into<Body<'a>>>(mut self, body: B) -> RequestBuilder<'a> {
        self.inner = self.inner.body(body);
        self
    }

    pub fn send(self) -> Result<Response> {
        info!("{:?}", self);

        self.add_auth_header().inner.send()
    }
}

impl<'a> fmt::Debug for RequestBuilder<'a> {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {:?}", self.method, self.url)
    }
}
