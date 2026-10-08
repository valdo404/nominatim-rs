#![doc = include_str!("../README.md")]

use std::{str::FromStr, time::Duration};

use reqwest::header::{HeaderName, HeaderValue};
use serde::de::DeserializeOwned;
use url::Url;

mod ident;
mod reverse;
mod types;

pub use ident::IdentificationMethod;
pub use reverse::ReverseRequest;
pub use types::{Address, ExtraTags, Place, Status};

/// The public Nominatim server, used unless another base URL is set.
pub const DEFAULT_BASE_URL: &str = "https://nominatim.openstreetmap.org/";
/// The request timeout used unless another one is set.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// The interface for accessing a Nominatim API server.
#[derive(Debug, Clone)]
pub struct Client {
    ident: IdentificationMethod, // how to access the server
    base_url: Url,               // defaults to https://nominatim.openstreetmap.org
    client: reqwest::Client,
    /// HTTP Request Timeout [`Duration`]
    pub timeout: Duration,
}

/// Configures a [`Client`]: base URL, request timeout and the underlying
/// [`reqwest::Client`].
///
/// ```rust
/// use std::time::Duration;
/// use nominatim::{Client, IdentificationMethod};
///
/// let http = reqwest::Client::builder()
///     .connect_timeout(Duration::from_secs(5))
///     .build()
///     .unwrap();
///
/// let client = Client::builder(IdentificationMethod::from_user_agent("Example Application Name"))
///     .base_url("http://localhost:8080/")
///     .unwrap()
///     .timeout(Duration::from_secs(30))
///     .http_client(http)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    ident: IdentificationMethod,
    base_url: Url,
    client: Option<reqwest::Client>,
    timeout: Duration,
}

impl ClientBuilder {
    /// Set the base url for all requests (default: [`DEFAULT_BASE_URL`]).
    ///
    /// Endpoints are resolved relative to it, so a base url with a path
    /// should end with a `/`.
    pub fn base_url<U: TryInto<Url>>(mut self, url: U) -> Result<Self, U::Error> {
        self.base_url = url.try_into()?;

        Ok(self)
    }

    /// Set the timeout of each request (default: [`DEFAULT_TIMEOUT`]).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Send the requests with an existing [`reqwest::Client`] instead of
    /// building a new one, to share its connection pool and its settings
    /// (TLS, proxy, connect timeout, default headers…).
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Build the [`Client`].
    pub fn build(self) -> Client {
        Client {
            ident: self.ident,
            base_url: self.base_url,
            client: self.client.unwrap_or_default(),
            timeout: self.timeout,
        }
    }
}

impl Client {
    /// Create a new [`Client`] from an [`IdentificationMethod`].
    pub fn new(ident: IdentificationMethod) -> Self {
        Self::builder(ident).build()
    }

    /// Start configuring a [`Client`] from an [`IdentificationMethod`].
    pub fn builder(ident: IdentificationMethod) -> ClientBuilder {
        ClientBuilder {
            ident,
            base_url: Url::parse(DEFAULT_BASE_URL).unwrap(),
            client: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Set the client's internal base url for all requests.
    pub fn set_base_url<U: TryInto<Url>>(&mut self, url: U) -> Result<(), U::Error> {
        self.base_url = url.try_into()?;

        Ok(())
    }

    /// The base url of all requests.
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// The url of an endpoint (`reverse`, `lookup`…), without a query.
    pub(crate) fn endpoint(&self, path: &str) -> Url {
        self.base_url.join(path).unwrap()
    }

    /// Send a GET request, identified as the client is, and read the answer
    /// as JSON. The HTTP status is not looked at: Nominatim also describes
    /// its errors in JSON.
    pub(crate) async fn get<T: DeserializeOwned>(&self, url: Url) -> Result<T, reqwest::Error> {
        self.client
            .get(url)
            .header(
                HeaderName::from_str(&self.ident.header())
                    .expect("invalid nominatim auth header name"),
                HeaderValue::from_str(&self.ident.value())
                    .expect("invalid nominatim auth header value"),
            )
            .timeout(self.timeout)
            .send()
            .await?
            .json()
            .await
    }

    /// Check the status of the nominatim server.
    ///
    /// [Documentation](https://nominatim.org/release-docs/develop/api/Status/)
    pub async fn status(&self) -> Result<Status, reqwest::Error> {
        let mut url = self.endpoint("status.php");
        url.set_query(Some("format=json"));

        self.get(url).await
    }

    /// Get [`Place`]s from a search query.
    ///
    /// [Documentation](https://nominatim.org/release-docs/develop/api/Search/)
    pub async fn search(&self, query: impl AsRef<str>) -> Result<Vec<Place>, reqwest::Error> {
        let mut url = self.base_url.clone();
        url.set_query(Some(&format!(
            "addressdetails=1&extratags=1&q={}&format=json",
            query.as_ref().replace(' ', "+")
        )));

        self.get(url).await
    }

    /// Generate a [`Place`] from latitude and longitude.
    ///
    /// Address details and extra tags are always asked for. See
    /// [`Client::reverse_request`] to choose the parameters of the request
    /// or to read the answer as raw JSON.
    ///
    /// [Documentation](https://nominatim.org/release-docs/develop/api/Reverse/)
    pub async fn reverse(
        &self,
        latitude: impl AsRef<str>,
        longitude: impl AsRef<str>,
        zoom: Option<u8>,
    ) -> Result<Place, reqwest::Error> {
        let mut url = self.endpoint("reverse");

        match zoom {
            Some(zoom) => {
                url.set_query(Some(&format!(
                    "addressdetails=1&extratags=1&format=json&lat={}&lon={}&zoom={}",
                    latitude.as_ref().replace(' ', ""),
                    longitude.as_ref().replace(' ', ""),
                    zoom
                )));
            }
            None => {
                url.set_query(Some(&format!(
                    "addressdetails=1&extratags=1&format=json&lat={}&lon={}",
                    latitude.as_ref().replace(' ', ""),
                    longitude.as_ref().replace(' ', ""),
                )));
            }
        }

        self.get(url).await
    }

    /// Start a reverse geocoding request whose parameters are chosen one by
    /// one. Nothing is asked for beyond `format=json` and the coordinates
    /// until the [`ReverseRequest`] says so.
    ///
    /// [Documentation](https://nominatim.org/release-docs/develop/api/Reverse/)
    pub fn reverse_request(
        &self,
        latitude: impl AsRef<str>,
        longitude: impl AsRef<str>,
    ) -> ReverseRequest<'_> {
        ReverseRequest::new(self, latitude.as_ref(), longitude.as_ref())
    }

    /// Return [`Place`]s from a list of OSM Node, Way, or Relations.
    ///
    /// [Documentation](https://nominatim.org/release-docs/develop/api/Lookup/)
    pub async fn lookup(&self, queries: Vec<&str>) -> Result<Vec<Place>, reqwest::Error> {
        let queries = queries.join(",");

        let mut url = self.endpoint("lookup");
        url.set_query(Some(&format!(
            "osm_ids={}&addressdetails=1&extratags=1&format=json",
            queries
        )));

        self.get(url).await
    }
}
