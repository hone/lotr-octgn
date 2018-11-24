extern crate rayon;
extern crate reqwest;
extern crate roxmltree;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[cfg(test)]
extern crate mockito;
extern crate serde_json;
extern crate strsim;
extern crate tempdir;
extern crate walkdir;
extern crate zip;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};

use rayon::prelude::*;
use roxmltree::Document;
use tempdir::TempDir;
use walkdir::WalkDir;

mod hall_of_beorn;
mod octgn;

struct CardDownload {
    id: String,
    front_url: String,
    back_url: Option<String>,
}

fn get_image_urls(
    octgn_cards: &Vec<octgn::Card>,
    hob_cards: &Vec<hall_of_beorn::Card>,
) -> Vec<CardDownload> {
    let hob_map = hob_cards.iter().fold(HashMap::new(), |mut acc, card| {
        acc.insert(&card.title, card);

        acc
    });

    octgn_cards
        .par_iter()
        .map(|octgn_card| {
            let hob_card = match hob_map.get(&octgn_card.name) {
                Some(hob_card) => hob_card,
                None => {
                    let hob_card = guess_hob_card(&hob_cards, &octgn_card);
                    println!(
                        "Warning: Could not find OCTGN Card '{}', using Hall of Beorn Card '{}' instead.",
                        &octgn_card.name, hob_card.title
                    );

                    hob_card
                }
            };

            CardDownload {
                id: octgn_card.id.to_string(),
                front_url: hob_card.front.image_path.to_owned(),
                back_url: None,
            }
        }).collect()
}

fn fetch_images(set_id: &str, cards: &Vec<CardDownload>) -> Result<(), Box<std::error::Error>> {
    let tmp_dir = TempDir::new("lotr")?;
    let set_dir = tmp_dir.path().join(octgn::LOTR_ID).join(set_id);
    std::fs::create_dir_all(&set_dir)?;

    cards.par_iter().for_each(|card| {
        let mut file_path = set_dir.join(&card.id);
        file_path.set_extension("jpg");
        let mut file = File::create(file_path).expect("can't create file");
        let mut resp = reqwest::get(&card.front_url).expect("can't process request");
        std::io::copy(&mut resp, &mut file).expect("can't write download to file");
    });

    Ok(())
}

fn zip_directory(dir: &str, output: &str) -> Result<(), Box<std::error::Error>> {
    let file = File::create(output)?;
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut buffer = Vec::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path
            .strip_prefix(std::path::Path::new(dir))
            .unwrap()
            .to_str()
            .unwrap();

        if path.is_file() {
            zip.start_file(name, options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&*buffer)?;
            buffer.clear();
        }
    }

    Ok(())
}

fn guess_hob_card<'a>(
    hob_cards: &'a Vec<hall_of_beorn::Card>,
    unknown_card: &octgn::Card,
) -> &'a hall_of_beorn::Card {
    let distance_map = hob_cards.iter().fold(HashMap::new(), |mut acc, hob_card| {
        acc.insert(
            &hob_card.title,
            strsim::levenshtein(&unknown_card.name, &hob_card.title),
        );

        acc
    });

    let title = distance_map
        .iter()
        .min_by_key(|(_, &value)| value)
        .unwrap()
        .0;

    hob_cards
        .iter()
        .find(|ref hob_card| &&hob_card.title == title)
        .unwrap()
}

fn main() {
    let mut file = File::open("fixtures/set.xml").unwrap();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();

    let doc = Document::parse(&text).unwrap();
    let set = octgn::Set::new(&doc);
    println!("{}: {}", set.name, set.id);
    let hob_cards =
        hall_of_beorn::fetch(&set.name).expect("Could not fetch data from hallofbeorn.com");
    let card_downloads = get_image_urls(&set.cards, &hob_cards);
    fetch_images(&set.id, &card_downloads).unwrap();
    zip_directory("lotr", &format!("{}.o8c", set.name).replace(" ", "-")).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    use mockito::mock;

    fn load_hall_of_beorn() -> Vec<hall_of_beorn::Card> {
        let set = "The Wilds of Rhovanion";
        let mut file = File::open("fixtures/hob.json").unwrap();
        let mut body = String::new();
        file.read_to_string(&mut body).unwrap();

        let _m = mock("GET", "/Export/Search?CardSet=The%20Wilds%20of%20Rhovanion")
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();

        hall_of_beorn::fetch(set).unwrap()
    }

    #[test]
    fn test_get_image_urls_normal_card() {
        let hob_cards = load_hall_of_beorn();
        let brand_son_of_bain = octgn::Card {
            id: "2b75792d-5873-4fc6-9272-d20dd517d36b".to_string(),
            name: "Brand son of Bain".to_string(),
        };
        let octgn_cards = vec![brand_son_of_bain];

        let card_downloads = get_image_urls(&octgn_cards, &hob_cards);
        assert_eq!(card_downloads.len(), 1);

        let card = card_downloads.get(0).unwrap();
        assert_eq!(&card.id, "2b75792d-5873-4fc6-9272-d20dd517d36b");
        assert_eq!(&card.front_url, "https://s3.amazonaws.com/hallofbeorn-resources/Images/Cards/The-Wilds-of-Rhovanion/Brand-son-of-Bain.jpg");
        assert!(&card.back_url.is_none());
    }

    #[test]
    fn test_get_image_urls_unknown_card() {
        let hob_cards = load_hall_of_beorn();
        let fire_drake = octgn::Card {
            id: "42a5a608-0699-4cd5-b69d-f7c3413cd5cd".to_string(),
            name: "Fire Drake".to_string(),
        };
        let octgn_cards = vec![fire_drake];

        let card_downloads = get_image_urls(&octgn_cards, &hob_cards);
        assert_eq!(card_downloads.len(), 1);

        let card = card_downloads.get(0).unwrap();
        assert_eq!(&card.id, "42a5a608-0699-4cd5-b69d-f7c3413cd5cd");
        assert_eq!(&card.front_url, "https://s3.amazonaws.com/hallofbeorn-resources/Images/Cards/The-Wilds-of-Rhovanion/Fire-drake.jpg");
        assert!(&card.back_url.is_none());
    }
}
