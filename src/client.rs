use reqwest::{self, IntoUrl, Method, RequestBuilder, Url, UrlError};
use failure::Error;

use ::Subject;

#[derive(Clone, Debug)]
pub struct Client {
    username: Option<String>,
    api_key: Option<String>,

    reqwest_client: reqwest::Client,
    api_base_url: Url,
    dl_base_url: Url,
}

static BINTRAY_API_BASEURL: &'static str = "https://api.bintray.com/";
static BINTRAY_DL_BASEURL: &'static str = "https://dl.bintray.com/";

impl Client {
    pub fn new() -> Result<Client, Error>
    {
        let reqwest_client = reqwest::Client::new();

        let api_base_url = Url::parse(BINTRAY_API_BASEURL)?;
        let dl_base_url = Url::parse(BINTRAY_DL_BASEURL)?;
        assert_eq!(api_base_url.scheme(), dl_base_url.scheme());

        Ok(Client {
            username: None,
            api_key: None,

            reqwest_client: reqwest_client,
            api_base_url: api_base_url,
            dl_base_url: dl_base_url,
        })
    }

    pub fn user(mut self, username: &str, api_key: &str) -> Self
    {
        self.username = Some(String::from(username));
        self.api_key = Some(String::from(api_key));
        self
    }

    pub fn subject(&self, subject: &str) -> Subject
    {
        Subject::new(self, subject)
    }

    pub fn api_url(&self, path: &str) -> Result<Url, UrlError>
    {
        self.api_base_url.join(path)
    }

    pub fn dl_url(&self, path: &str) -> Result<Url, UrlError>
    {
        self.dl_base_url.join(path)
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder
    {
        let builder = self.reqwest_client.get(url);
        self.add_basic_auth(builder)
    }

    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder
    {
        let builder = self.reqwest_client.put(url);
        self.add_basic_auth(builder)
    }

    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder
    {
        let builder = self.reqwest_client.post(url);
        self.add_basic_auth(builder)
    }

    pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder
    {
        let builder = self.reqwest_client.patch(url);
        self.add_basic_auth(builder)
    }

    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder
    {
        let builder = self.reqwest_client.delete(url);
        self.add_basic_auth(builder)
    }

    pub fn request<U: IntoUrl>(&self, method: Method, url: U)
        -> RequestBuilder
    {
        let builder = self.reqwest_client.request(method, url);
        self.add_basic_auth(builder)
    }

    pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder
    {
        let builder = self.reqwest_client.head(url);
        self.add_basic_auth(builder)
    }

    fn add_basic_auth(&self, mut builder: RequestBuilder) -> RequestBuilder
    {
        match self.username {
            Some(ref username) => {
                builder.basic_auth(username.clone(), self.api_key.clone());
                builder
            }
            None => {
                builder
            }
        }
    }
}
