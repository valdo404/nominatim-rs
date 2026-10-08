use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The status of a Nominatim server.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Status {
    pub status: usize,
    pub message: String,
    pub data_updated: Option<String>,
    pub software_version: Option<String>,
    pub database_version: Option<String>,
}

/// A location returned by the Nominatim server.
///
/// [Output format](https://nominatim.org/release-docs/develop/api/Output/)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Place {
    #[serde(default)]
    pub place_id: usize,
    #[serde(default)]
    pub licence: String,
    #[serde(default)]
    pub osm_type: String,
    #[serde(default)]
    pub osm_id: usize,
    #[serde(default)]
    pub boundingbox: Vec<String>,
    #[serde(default)]
    pub lat: String,
    #[serde(default)]
    pub lon: String,
    #[serde(default)]
    pub display_name: String,
    /// The localised name of the place.
    pub name: Option<String>,
    pub class: Option<String>,
    #[serde(rename = "type")]
    pub _type: Option<String>,
    /// Same as [`Place::class`] in the `jsonv2` format.
    pub category: Option<String>,
    /// The kind of address element the place is (`city`, `road`, `tourism`…),
    /// which is also the key of its name in [`Place::address`].
    pub addresstype: Option<String>,
    pub place_rank: Option<u32>,
    pub importance: Option<f64>,
    pub icon: Option<String>,
    #[serde(default)]
    pub address: Option<Address>,
    pub extratags: Option<ExtraTags>,
    /// The names of the place, when asked for with `namedetails=1`.
    pub namedetails: Option<BTreeMap<String, Option<String>>>,
    /// Everything else the server sent (`error` when it could not answer,
    /// `geojson` and other geometry outputs, future fields…).
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

/// Declares [`Address`] and its lookup by key from one list of fields.
macro_rules! address {
    ($( $(#[doc = $doc:literal])* $field:ident => $key:literal, )+) => {
        /// An address for a place.
        ///
        /// The server names the elements of an address after what they are:
        /// besides the administrative levels, the place itself comes under
        /// the key of its own kind (`tourism`, `viewpoint`, `peak`…), which
        /// can be any OSM value. The usual keys are fields; the others are
        /// kept in [`Address::other`], and [`Address::get`] reads both.
        ///
        /// [Address details](https://nominatim.org/release-docs/develop/api/Output/#addressdetails)
        #[derive(Serialize, Deserialize, Debug, Clone, Default)]
        pub struct Address {
            $(
                $(#[doc = $doc])*
                #[serde(rename = $key, default, skip_serializing_if = "Option::is_none")]
                pub $field: Option<String>,
            )+
            /// The elements under any other key.
            #[serde(flatten)]
            pub other: BTreeMap<String, Value>,
        }

        impl Address {
            /// The address element under `key`, as the server names it,
            /// whether it is one of the fields or not.
            pub fn get(&self, key: &str) -> Option<&str> {
                match key {
                    $( $key => self.$field.as_deref(), )+
                    _ => self.other.get(key).and_then(Value::as_str),
                }
            }
        }
    };
}

address! {
    continent => "continent",

    country => "country",
    country_code => "country_code",

    region => "region",
    state => "state",
    state_district => "state_district",
    county => "county",
    iso3166_2_lvl4 => "ISO3166-2-lvl4",

    municipality => "municipality",
    city => "city",
    town => "town",
    village => "village",

    city_district => "city_district",
    district => "district",
    borough => "borough",
    suburb => "suburb",
    subdivision => "subdivision",

    hamlet => "hamlet",
    croft => "croft",
    isolated_dwelling => "isolated_dwelling",

    neighbourhood => "neighbourhood",
    allotments => "allotments",
    quarter => "quarter",

    city_block => "city_block",
    residential => "residential",
    farm => "farm",
    farmyard => "farmyard",
    industrial => "industrial",
    commercial => "commercial",
    retail => "retail",

    road => "road",

    house_number => "house_number",
    house_name => "house_name",

    emergency => "emergency",
    historic => "historic",
    military => "military",
    natural => "natural",
    landuse => "landuse",
    place => "place",
    railway => "railway",
    man_made => "man_made",
    aerialway => "aerialway",
    boundary => "boundary",
    amenity => "amenity",
    aeroway => "aeroway",
    club => "club",
    craft => "craft",
    leisure => "leisure",
    office => "office",
    mountain_pass => "mountain_pass",
    shop => "shop",
    tourism => "tourism",
    bridge => "bridge",
    tunnel => "tunnel",
    waterway => "waterway",

    postcode => "postcode",
}

/// Extra metadata that a place may have.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtraTags {
    pub capital: Option<String>,
    pub website: Option<String>,
    pub wikidata: Option<String>,
    pub wikipedia: Option<String>,
    pub population: Option<String>,
    /// The tags under any other key.
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const REVERSE: &str = r#"{
        "place_id": 104373226,
        "licence": "Data © OpenStreetMap contributors, ODbL 1.0. http://osm.org/copyright",
        "osm_type": "node",
        "osm_id": 4504463389,
        "lat": "45.8326221",
        "lon": "6.8651706",
        "class": "tourism",
        "type": "viewpoint",
        "place_rank": 30,
        "importance": 0.00001,
        "addresstype": "tourism",
        "name": "Belvédère",
        "display_name": "Belvédère, Chemin des Crêtes, Le Lavancher, Chamonix-Mont-Blanc, Haute-Savoie, 74400, France",
        "address": {
            "tourism": "Belvédère",
            "viewpoint": "Point de vue",
            "sports_centre": "Stade",
            "road": "Chemin des Crêtes",
            "hamlet": "Le Lavancher",
            "village": "Chamonix-Mont-Blanc",
            "county": "Haute-Savoie",
            "ISO3166-2-lvl6": "FR-74",
            "state": "Auvergne-Rhône-Alpes",
            "ISO3166-2-lvl4": "FR-ARA",
            "postcode": "74400",
            "country": "France",
            "country_code": "fr"
        },
        "extratags": {"ele": "1250", "wikidata": "Q1"},
        "namedetails": {"name": "Belvédère", "name:en": "Viewpoint"},
        "boundingbox": ["45.8325721", "45.8326721", "6.8651206", "6.8652206"],
        "geojson": {"type": "Point", "coordinates": [6.8651706, 45.8326221]}
    }"#;

    #[test]
    fn a_place_keeps_everything_the_server_sent() {
        let place: Place = serde_json::from_str(REVERSE).unwrap();

        assert_eq!(place.osm_id, 4504463389);
        assert_eq!(place.name.as_deref(), Some("Belvédère"));
        assert_eq!(place.addresstype.as_deref(), Some("tourism"));
        assert_eq!(place._type.as_deref(), Some("viewpoint"));
        assert_eq!(place.place_rank, Some(30));
        assert_eq!(place.boundingbox.len(), 4);
        assert_eq!(
            place.namedetails.unwrap()["name:en"].as_deref(),
            Some("Viewpoint")
        );
        assert_eq!(place.other["geojson"]["type"], "Point");

        let extratags = place.extratags.unwrap();
        assert_eq!(extratags.wikidata.as_deref(), Some("Q1"));
        assert_eq!(extratags.other["ele"], "1250");
    }

    #[test]
    fn an_address_has_its_usual_elements_as_fields() {
        let address = serde_json::from_str::<Place>(REVERSE)
            .unwrap()
            .address
            .unwrap();

        assert_eq!(address.tourism.as_deref(), Some("Belvédère"));
        assert_eq!(address.road.as_deref(), Some("Chemin des Crêtes"));
        assert_eq!(address.hamlet.as_deref(), Some("Le Lavancher"));
        assert_eq!(address.village.as_deref(), Some("Chamonix-Mont-Blanc"));
        assert_eq!(address.county.as_deref(), Some("Haute-Savoie"));
        assert_eq!(address.iso3166_2_lvl4.as_deref(), Some("FR-ARA"));
        assert_eq!(address.country_code.as_deref(), Some("fr"));
        assert_eq!(address.city, None);
        assert_eq!(address.town, None);
    }

    #[test]
    fn an_address_keeps_the_elements_it_has_no_field_for() {
        let address = serde_json::from_str::<Place>(REVERSE)
            .unwrap()
            .address
            .unwrap();

        assert_eq!(address.other["viewpoint"], "Point de vue");
        assert_eq!(address.other["sports_centre"], "Stade");
        assert_eq!(address.other["ISO3166-2-lvl6"], "FR-74");
        assert!(!address.other.contains_key("road"));
    }

    #[test]
    fn get_reads_an_address_element_by_its_key() {
        let address = serde_json::from_str::<Place>(REVERSE)
            .unwrap()
            .address
            .unwrap();

        assert_eq!(address.get("village"), Some("Chamonix-Mont-Blanc"));
        assert_eq!(address.get("ISO3166-2-lvl4"), Some("FR-ARA"));
        assert_eq!(address.get("viewpoint"), Some("Point de vue"));
        assert_eq!(address.get("sports_centre"), Some("Stade"));
        assert_eq!(address.get("city"), None);
        assert_eq!(address.get("no_such_key"), None);
    }

    #[test]
    fn an_address_is_written_back_as_it_was_read() {
        let json: Value = serde_json::from_str(REVERSE).unwrap();
        let address: Address = serde_json::from_value(json["address"].clone()).unwrap();

        assert_eq!(serde_json::to_value(&address).unwrap(), json["address"]);
    }

    #[test]
    fn an_error_answer_is_a_place_with_the_error_kept() {
        let place: Place = serde_json::from_str(r#"{"error": "Unable to geocode"}"#).unwrap();

        assert_eq!(place.other["error"], "Unable to geocode");
        assert!(place.address.is_none());
        assert_eq!(place.display_name, "");
    }
}
