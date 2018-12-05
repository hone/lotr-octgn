#[cfg(not(test))]
const HOB_URL: &'static str = "http://hallofbeorn.com";
#[cfg(test)]
const HOB_URL: &str = mockito::SERVER_URL;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Card {
    pub title: String,
    pub is_unique: bool,
    pub card_type: String,
    pub card_sub_type: String,
    pub sphere: Option<String>,
    pub front: Side,
    pub back: Option<Side>,
    pub card_set: String,
    pub number: u32,
    pub quantity: u32,
    pub artist: String,
    pub has_errata: bool,
    pub categories: Option<Vec<String>>,
}

impl Card {
    pub fn fetch_all(set_name: &str) -> Result<Vec<Card>, reqwest::Error> {
        let cards: Vec<Card> = reqwest::Client::new()
            .get(&format!("{}/Export/Search?CardSet={}", HOB_URL, set_name))
            .send()?
            .json()?;

        Ok(cards)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Stats {
    pub threat_cost: Option<String>,
    pub resource_cost: Option<String>,
    pub willpower: Option<String>,
    pub attack: Option<String>,
    pub defense: Option<String>,
    pub hit_points: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Side {
    pub subtitle: Option<String>,
    pub image_path: String,
    pub stats: Option<Stats>,
    pub traits: Vec<String>,
    pub keywords: Vec<String>,
    pub text: Vec<String>,
    pub flavor_text: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CardSet {
    pub name: String,
    pub cycle: Option<String>,
    pub set_type: String,
}

impl CardSet {
    pub fn fetch_all() -> Result<Vec<CardSet>, reqwest::Error> {
        let card_sets = reqwest::Client::new()
            .get(&format!("{}/Export/CardSets", HOB_URL))
            .send()?
            .json()?;

        Ok(card_sets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;
    use std::io::Read;

    use mockito::mock;

    #[test]
    fn test_card_fetch_all() {
        let set = "The Wilds of Rhovanion";
        let mut file = File::open("fixtures/hob/search.json").unwrap();
        let mut body = String::new();
        file.read_to_string(&mut body).unwrap();

        let _m = mock("GET", "/Export/Search?CardSet=The%20Wilds%20of%20Rhovanion")
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();

        let result = Card::fetch_all(set);
        assert!(result.is_ok());

        let cards = result.unwrap();
        assert_eq!(cards.len(), 80);
    }

    #[test]
    fn test_card_sets_fetch_all() {
        let mut file = File::open("fixtures/hob/card_sets.json").unwrap();
        let mut body = String::new();
        file.read_to_string(&mut body).unwrap();

        let _m = mock("GET", "/Export/CardSets")
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();

        let result = CardSet::fetch_all();
        assert!(result.is_ok());

        let card_sets = result.unwrap();
        assert_eq!(card_sets.len(), 144);
    }
}
