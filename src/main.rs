extern crate rayon;
extern crate reqwest;
extern crate roxmltree;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tempdir;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use rayon::prelude::*;
use roxmltree::Document;
use tempdir::TempDir;

const HOB_URL: &'static str = "http://hallofbeorn.com/Export/Search?CardSet=";
const LOTR_OCTGN_ID: &'static str = "a21af4e8-be4b-4cda-a6b6-534f9717391f";

struct Set {
    pub id: String,
    pub name: String,
    pub cards: Vec<Card>,
}

impl Set {
    pub fn new(doc: Document) -> Self {
        let node = doc.root().first_child().unwrap();
        let atts = attributes(node.attributes());

        let cards_node = node
            .children()
            .find(|child| child.is_element() && child.tag_name().name() == "cards")
            .unwrap();
        let cards = cards_node
            .children()
            .filter(|card_node| card_node.attributes().len() > 0)
            .map(|card_node| {
                let atts = attributes(card_node.attributes());
                Card {
                    id: atts.get("id").unwrap().to_string(),
                    name: atts.get("name").unwrap().to_string(),
                    ctype: "Ally".to_string(),
                }
            }).collect();

        Self {
            id: atts.get("id").unwrap().to_string(),
            name: atts.get("name").unwrap().to_string(),
            cards: cards,
        }
    }
}

struct Card {
    pub id: String,
    pub name: String,
    pub ctype: String,
}

fn attributes<'a>(atts: &'a [roxmltree::Attribute]) -> HashMap<&'a str, &'a str> {
    atts.iter().fold(HashMap::new(), |mut acc, attribute| {
        acc.insert(attribute.name(), attribute.value());

        acc
    })
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Stats {
    threat_cost: Option<String>,
    resource_cost: Option<String>,
    willpower: Option<String>,
    attack: Option<String>,
    defense: Option<String>,
    hit_points: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Side {
    subtitle: Option<String>,
    image_path: String,
    stats: Option<Stats>,
    traits: Vec<String>,
    keywords: Vec<String>,
    text: Vec<String>,
    flavor_text: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HallOfBeornCard {
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

fn fetch(set_name: &str) -> Result<Vec<HallOfBeornCard>, reqwest::Error> {
    let cards: Vec<HallOfBeornCard> = reqwest::Client::new()
        .get(&format!("{}{}", HOB_URL, set_name))
        .send()?
        .json()?;

    Ok(cards)
}

fn fetch_images(
    set_id: &str,
    octgn_cards: &Vec<Card>,
    hob_cards: &Vec<HallOfBeornCard>,
) -> Result<Vec<String>, Box<std::error::Error>> {
    let octgn_map = octgn_cards.iter().fold(HashMap::new(), |mut acc, card| {
        acc.insert(&card.name, &card.id);

        acc
    });

    //let tmp_dir = TempDir::new("lotr")?;
    let tmp_dir = format!("lotr/{}/{}", LOTR_OCTGN_ID, set_id);
    std::fs::create_dir_all(&tmp_dir)?;

    let error_cards = hob_cards
        .par_iter()
        .filter_map(|hob_card| {
            match octgn_map.get(&hob_card.title) {
                Some(octgn_card) => {
                    //let file_path = tmp_dir.path().join(octgn_card).join(".jpg");
                    let file_path = format!("{}/{}.jpg", tmp_dir, octgn_card);
                    let mut file = File::create(file_path).expect("can't create file");
                    let mut resp =
                        reqwest::get(&hob_card.front.image_path).expect("can't process request");
                    std::io::copy(&mut resp, &mut file).expect("can't write download to file");

                    None
                }
                None => Some(hob_card.title.clone()),
            }
        }).collect();

    Ok(error_cards)
}

fn main() {
    let mut file = File::open("set.xml").unwrap();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();

    let doc = Document::parse(&text).unwrap();
    let set = Set::new(doc);
    println!("{}: {}", set.name, set.id);
    let hob_cards = fetch(&set.name).unwrap();
    let error_cards = fetch_images(&set.id, &set.cards, &hob_cards).unwrap();
    for card in error_cards {
        println!("{}", card);
    }
}
