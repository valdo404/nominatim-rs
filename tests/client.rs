mod common;

use std::time::Duration;

use nominatim::{Client, IdentificationMethod};

const PLACE: &str = r#"{
    "place_id": 1,
    "licence": "Data © OpenStreetMap contributors, ODbL 1.0.",
    "osm_type": "way",
    "osm_id": 32965412,
    "lat": "40.689253",
    "lon": "-74.044548",
    "display_name": "Statue of Liberty, New York, United States",
    "boundingbox": ["40.6888049", "40.6896741", "-74.0451069", "-74.0439637"]
}"#;

fn client(base_url: &str) -> Client {
    Client::builder(IdentificationMethod::from_user_agent("nominatim-rs tests"))
        .base_url(base_url)
        .unwrap()
        .build()
}

#[tokio::test]
async fn builder_defaults_are_those_of_new() {
    let built = Client::builder(IdentificationMethod::from_user_agent("x")).build();
    let new = Client::new(IdentificationMethod::from_user_agent("x"));

    assert_eq!(built.base_url().as_str(), nominatim::DEFAULT_BASE_URL);
    assert_eq!(built.base_url(), new.base_url());
    assert_eq!(built.timeout, nominatim::DEFAULT_TIMEOUT);
    assert_eq!(built.timeout, new.timeout);
}

#[tokio::test]
async fn builder_sets_the_base_url_and_the_timeout() {
    let client = Client::builder(IdentificationMethod::from_user_agent("x"))
        .base_url("http://localhost:8080/nominatim/")
        .unwrap()
        .timeout(Duration::from_secs(60))
        .build();

    assert_eq!(
        client.base_url().as_str(),
        "http://localhost:8080/nominatim/"
    );
    assert_eq!(client.timeout, Duration::from_secs(60));
    assert_eq!(
        client.reverse_request("1", "2").url().as_str(),
        "http://localhost:8080/nominatim/reverse?format=json&lat=1&lon=2"
    );
}

#[tokio::test]
async fn reverse_keeps_its_query_and_identifies_with_the_user_agent() {
    let (base_url, server) = common::serve_once(200, PLACE).await;

    let place = client(&base_url)
        .reverse("40.689249", "-74.044500", Some(18))
        .await
        .unwrap();
    assert_eq!(place.osm_id, 32965412);
    assert_eq!(place.lat, "40.689253");

    let recorded = server.await.unwrap();
    assert_eq!(
        recorded.request_line,
        "GET /reverse?addressdetails=1&extratags=1&format=json&lat=40.689249&lon=-74.044500&zoom=18 HTTP/1.1"
    );
    assert_eq!(recorded.header("user-agent"), Some("nominatim-rs tests"));
}

#[tokio::test]
async fn referer_identification_is_sent_as_a_referer() {
    let (base_url, server) = common::serve_once(200, PLACE).await;

    let client = Client::builder(IdentificationMethod::from_referer("https://example.org"))
        .base_url(base_url.as_str())
        .unwrap()
        .build();
    client.reverse_request("1", "2").send().await.unwrap();

    let recorded = server.await.unwrap();
    assert_eq!(recorded.header("referer"), Some("https://example.org"));
    assert_eq!(recorded.header("user-agent"), None);
}

#[tokio::test]
async fn reverse_request_sends_its_parameters_in_the_order_they_were_set() {
    let (base_url, server) = common::serve_once(200, PLACE).await;

    let place = client(&base_url)
        .reverse_request("45.5", "6.25")
        .zoom(16)
        .address_details(true)
        .send()
        .await
        .unwrap();
    assert_eq!(
        place.display_name,
        "Statue of Liberty, New York, United States"
    );

    let recorded = server.await.unwrap();
    assert_eq!(
        recorded.request_line,
        "GET /reverse?format=json&lat=45.5&lon=6.25&zoom=16&addressdetails=1 HTTP/1.1"
    );
}

#[tokio::test]
async fn reverse_request_writes_every_parameter() {
    let client = client("http://localhost/");

    let url = client
        .reverse_request("45.5", "")
        .address_details(false)
        .extra_tags(true)
        .name_details(true)
        .accept_language("fr,en;q=0.8")
        .param("layer", "natural,poi")
        .url();

    assert_eq!(
        url.as_str(),
        "http://localhost/reverse?format=json&lat=45.5&lon=&addressdetails=0&extratags=1&namedetails=1&accept-language=fr%2Cen%3Bq%3D0.8&layer=natural%2Cpoi"
    );
}

#[tokio::test]
async fn a_given_http_client_is_used_with_its_own_settings() {
    let (base_url, server) = common::serve_once(200, PLACE).await;

    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("x-from-the-caller", "yes".parse().unwrap());
    let http = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let client = Client::builder(IdentificationMethod::from_user_agent("nominatim-rs tests"))
        .base_url(base_url.as_str())
        .unwrap()
        .http_client(http)
        .build();
    client.reverse_request("1", "2").send().await.unwrap();

    let recorded = server.await.unwrap();
    assert_eq!(recorded.header("x-from-the-caller"), Some("yes"));
    assert_eq!(recorded.header("user-agent"), Some("nominatim-rs tests"));
}

#[tokio::test]
async fn the_timeout_applies_to_the_request() {
    // A listener that accepts connections and never answers.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base_url = format!("http://{}/", listener.local_addr().unwrap());

    let client = Client::builder(IdentificationMethod::from_user_agent("nominatim-rs tests"))
        .base_url(base_url.as_str())
        .unwrap()
        .timeout(Duration::from_millis(200))
        .build();

    let error = client.reverse_request("1", "2").send().await.unwrap_err();
    assert!(error.is_timeout(), "{error:?}");
    drop(listener);
}

#[tokio::test]
async fn send_json_returns_the_answer_as_it_came() {
    let (base_url, server) = common::serve_once(
        200,
        r#"{"place_id": 7, "name": "Somewhere", "address": {"viewpoint": "Belvedere", "country_code": "fr"}, "something_new": [1, 2]}"#,
    )
    .await;

    let json = client(&base_url)
        .reverse_request("1", "2")
        .send_json()
        .await
        .unwrap();
    server.await.unwrap();

    assert_eq!(json["address"]["viewpoint"], "Belvedere");
    assert_eq!(json["something_new"], serde_json::json!([1, 2]));
}

#[tokio::test]
async fn an_error_answer_is_still_readable() {
    let (base_url, server) = common::serve_once(404, r#"{"error": "Unable to geocode"}"#).await;

    let json = client(&base_url)
        .reverse_request("0", "0")
        .send_json()
        .await
        .unwrap();
    server.await.unwrap();

    assert_eq!(json["error"], "Unable to geocode");
}
