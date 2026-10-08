use serde::de::DeserializeOwned;
use url::Url;

use crate::{Client, Place};

/// A reverse geocoding request under construction, started by
/// [`Client::reverse_request`].
///
/// The query string is `format=json`, the coordinates, then one parameter per
/// method called, in the order of the calls:
///
/// ```rust
/// # use nominatim::{Client, IdentificationMethod};
/// let client = Client::new(IdentificationMethod::from_user_agent("Example Application Name"));
///
/// let request = client
///     .reverse_request("40.689249", "-74.044500")
///     .zoom(16)
///     .address_details(true);
///
/// assert_eq!(
///     request.url().as_str(),
///     "https://nominatim.openstreetmap.org/reverse?format=json&lat=40.689249&lon=-74.044500&zoom=16&addressdetails=1"
/// );
/// ```
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it is sent"]
pub struct ReverseRequest<'a> {
    client: &'a Client,
    params: Vec<(String, String)>,
}

impl<'a> ReverseRequest<'a> {
    pub(crate) fn new(client: &'a Client, latitude: &str, longitude: &str) -> Self {
        Self {
            client,
            params: vec![
                ("format".to_string(), "json".to_string()),
                ("lat".to_string(), latitude.to_string()),
                ("lon".to_string(), longitude.to_string()),
            ],
        }
    }

    /// Level of detail required for the address, from 0 (country) to 18
    /// (building).
    pub fn zoom(self, zoom: u8) -> Self {
        self.param("zoom", zoom.to_string())
    }

    /// Include a breakdown of the address into elements ([`Place::address`]).
    pub fn address_details(self, enabled: bool) -> Self {
        self.flag("addressdetails", enabled)
    }

    /// Include additional information such as wikipedia link or opening
    /// hours ([`Place::extratags`]).
    pub fn extra_tags(self, enabled: bool) -> Self {
        self.flag("extratags", enabled)
    }

    /// Include the alternative names of the place ([`Place::namedetails`]).
    pub fn name_details(self, enabled: bool) -> Self {
        self.flag("namedetails", enabled)
    }

    /// Preferred language order of the answer, as in the `Accept-Language`
    /// HTTP header.
    pub fn accept_language(self, languages: impl AsRef<str>) -> Self {
        self.param("accept-language", languages.as_ref())
    }

    /// Add any other parameter of the API.
    pub fn param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.push((name.into(), value.into()));
        self
    }

    fn flag(self, name: &str, enabled: bool) -> Self {
        self.param(name, if enabled { "1" } else { "0" })
    }

    /// The url the request is sent to.
    pub fn url(&self) -> Url {
        let mut url = self.client.endpoint("reverse");
        url.query_pairs_mut().extend_pairs(&self.params);

        url
    }

    /// Send the request and read the answer as a [`Place`].
    pub async fn send(self) -> Result<Place, reqwest::Error> {
        self.send_as().await
    }

    /// Send the request and return the answer as it came, without losing
    /// anything [`Place`] does not model.
    pub async fn send_json(self) -> Result<serde_json::Value, reqwest::Error> {
        self.send_as().await
    }

    /// Send the request and read the answer as any deserializable type.
    pub async fn send_as<T: DeserializeOwned>(self) -> Result<T, reqwest::Error> {
        self.client.get(self.url()).await
    }
}
