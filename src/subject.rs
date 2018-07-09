use failure::Error;
use ::{BintrayError, Client, Repository};

use std::iter::Map;
use std::vec::IntoIter;

#[derive(Clone, Debug)]
pub struct Subject {
    subject: String,

    client: Client,
}

#[derive(Deserialize)]
struct RepositoryNamesListEntry {
    name: String,
}

impl Subject {
    pub fn new(client: &Client, subject: &str) -> Subject
    {
        Subject {
            subject: String::from(subject),

            client: client.clone(),
        }
    }

    pub fn get_name(&self) -> &str { &self.subject }

    fn repository_names_iter(&self)
        -> Result<Map<IntoIter<RepositoryNamesListEntry>,
        fn(RepositoryNamesListEntry) -> String>, Error>
    {
        let url = self.client.api_url(&format!("/repos/{}", self.subject))?;

        let mut response = self.client
            .get(url)
            .send()?;

        if response.status().is_success() {
            let repository_entries: Vec<RepositoryNamesListEntry> = response.json()?;

            fn extract_repository_name(e: RepositoryNamesListEntry) -> String {
                e.name
            }
            let extract_repository_name: fn(RepositoryNamesListEntry) -> String =
                extract_repository_name;

            let repository_names_iter = repository_entries
                .into_iter()
                .map(extract_repository_name);
            Ok(repository_names_iter)
        } else {
            #[derive(Deserialize)]
            struct ListRepositoryNamesError {
                message: String,
            }

            let resp: ListRepositoryNamesError = response.json()?;

            throw!(BintrayError::BintrayApiError { message: resp.message })
        }
    }

    pub fn repository_names(&self) -> Result<Vec<String>, Error>
    {
        let mut repository_names: Vec<String> = self
            .repository_names_iter()?
            .collect();
        repository_names.sort();

        Ok(repository_names)
    }

    pub fn repository(&self, repository_name: &str) -> Repository
    {
        Repository::new(&self.client,
                        &self.subject,
                        repository_name)
    }
}
