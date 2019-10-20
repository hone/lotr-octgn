use crate::tests::fixtures;

use std::fs::File;
use std::io::Read;

use mockito::{mock, Mock};

pub fn card_set(set_name: &str) -> Result<Mock, Box<dyn std::error::Error>> {
    let mut file = File::open(fixtures::lotr::hall_of_beorn::SEARCH)?;
    let mut body = String::new();
    file.read_to_string(&mut body)?;

    let mut url = reqwest::Url::parse(mockito::SERVER_URL)?;
    url.set_path("/Export/Search");
    url.set_query(Some(&format!("CardSet={}", &set_name)));
    let path = format!("{}?{}", url.path(), url.query().unwrap());

    let m = mock("GET", path.as_str())
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    Ok(m)
}

pub fn card_sets() -> Result<Mock, std::io::Error> {
    let mut file = File::open(fixtures::lotr::hall_of_beorn::CARD_SETS)?;
    let mut body = String::new();

    file.read_to_string(&mut body)?;
    let m = mock("GET", "/Export/CardSets")
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    Ok(m)
}
