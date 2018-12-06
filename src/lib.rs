#[cfg(test)]
extern crate mockito;
extern crate rayon;
extern crate reqwest;
extern crate roxmltree;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate strsim;
extern crate tempdir;
extern crate walkdir;
extern crate zip;

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use rayon::prelude::*;
use tempdir::TempDir;
use walkdir::WalkDir;

mod hall_of_beorn;
mod octgn;

const MAX_SET_LEVENSHTEIN: usize = 5;

struct CardDownload {
    id: String,
    front_url: String,
    back_url: Option<String>,
}

fn octgn_to_hob<'a>(
    octgn_card_name: &String,
    hob_map: &'a HashMap<&String, &hall_of_beorn::Card>,
    hob_cards: &'a Vec<hall_of_beorn::Card>,
) -> &'a hall_of_beorn::Card {
    match hob_map.get(&octgn_card_name) {
        Some(hob_card) => hob_card,
        None => {
            let hob_card = guess_hob_card(hob_cards, &octgn_card_name);
            println!(
                "Warning: Could not find OCTGN Card '{}', using Hall of Beorn Card '{}' instead.",
                &octgn_card_name, hob_card.title
            );

            hob_card
        }
    }
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
            let hob_card = octgn_to_hob(&octgn_card.name, &hob_map, &hob_cards);
            let back_url = octgn_card.back_name.as_ref().map(|back_name| {
                if back_name == &octgn_card.name {
                    // replace A side to B side
                    hob_card.front.image_path.replace("A.jpg", "B.jpg")
                } else {
                    // if the back has a different name, then fetch that card from Hall of Beorn
                    octgn_to_hob(&back_name, &hob_map, &hob_cards)
                        .front
                        .image_path
                        .to_owned()
                }
            });

            CardDownload {
                id: octgn_card.id.to_string(),
                front_url: hob_card.front.image_path.to_owned(),
                back_url: back_url,
            }
        }).collect()
}

fn fetch_images(
    work_dir: &Path,
    set_id: &str,
    cards: &Vec<CardDownload>,
) -> Result<(), Box<std::error::Error>> {
    let set_dir = work_dir
        .join(octgn::LOTR_ID)
        .join("Sets")
        .join(set_id)
        .join("Cards");
    std::fs::create_dir_all(&set_dir)?;

    cards.par_iter().for_each(|card| {
        {
            let mut file_path = set_dir.join(&card.id);
            file_path.set_extension("jpg");
            let mut file = File::create(&file_path).expect("can't create file");
            let mut resp = reqwest::get(&card.front_url).expect("can't process request");
            std::io::copy(&mut resp, &mut file).expect("can't write download to file");
            println!("Front: {}", &file_path.to_str().unwrap());
        }

        if card.back_url.is_some() {
            let back_url = card.back_url.as_ref().unwrap();
            // set_extension replaces the .B
            let file_path = set_dir.join(format!("{}.B.jpg", &card.id));
            println!("Back: {}", &file_path.to_str().unwrap());
            let mut file = File::create(&file_path).expect("can't create file");
            let mut resp = reqwest::get(back_url).expect("can't process request");
            std::io::copy(&mut resp, &mut file).expect("can't write download to file");
        }
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
    unknown_card_name: &str,
) -> &'a hall_of_beorn::Card {
    let title = hob_cards
        .par_iter()
        .map(|hob_card| {
            (
                &hob_card.title,
                strsim::levenshtein(&unknown_card_name, &hob_card.title),
            )
        }).min_by_key(|&(_, value)| value)
        .unwrap()
        .0;

    hob_cards
        .iter()
        .find(|ref hob_card| &hob_card.title == title)
        .unwrap()
}

pub fn pack(set: &octgn::Set) -> Result<(), Box<std::error::Error>> {
    println!("{}: {}", set.name, set.id);
    println!("Fetching data from Hall of Beorn");
    let hob_cards = hall_of_beorn::Card::fetch_all(&set.name)?;
    println!("Generating image urls");
    let card_downloads = get_image_urls(&set.cards, &hob_cards);

    let tmp_dir = TempDir::new("lotr")?;

    println!("Downloading images");
    fetch_images(&tmp_dir.path(), &set.id, &card_downloads)?;
    let zip_file = format!("{}.o8c", set.name).replace(" ", "-");
    println!("Zipping file {}", &zip_file);
    zip_directory(&tmp_dir.path().to_str().unwrap(), &zip_file)?;

    Ok(())
}

pub fn sets() -> Result<Vec<octgn::Set>, Box<std::error::Error>> {
    let dir = "fixtures/octgn/o8g/Sets";
    let octgn_sets = octgn::Set::fetch_all(dir)?;
    let hob_sets = hall_of_beorn::CardSet::fetch_all()?;

    // only care about octgn sets that also have a matching hob set
    let ordered_set_names: Vec<String> = hob_sets
        .into_par_iter()
        .filter_map(|hob_set| {
            let min_match_set = octgn_sets
                .par_iter()
                .map(|octgn_set| {
                    (
                        &octgn_set.name,
                        strsim::levenshtein(&hob_set.name, &octgn_set.name),
                    )
                }).min_by_key(|&(_, value)| value)
                .unwrap();

            if min_match_set.1 < MAX_SET_LEVENSHTEIN {
                Some(min_match_set.0.clone())
            } else {
                None
            }
        }).collect();

    let mut ordered_sets = octgn_sets
        .into_par_iter()
        .filter(|set| ordered_set_names.contains(&set.name))
        .collect::<Vec<octgn::Set>>();
    ordered_sets.sort_by_key(|set| {
        ordered_set_names
            .par_iter()
            .position_any(|name| &set.name == name)
    });

    Ok(ordered_sets)
}

#[cfg(test)]
mod tests {
    pub mod mocks;

    use super::*;

    use self::mocks::hall_of_beorn as hob_mocks;
    use mockito::mock;

    fn load_hall_of_beorn() -> Vec<hall_of_beorn::Card> {
        let set = "The Wilds of Rhovanion";
        let _m = hob_mocks::card_set(&set).unwrap();

        hall_of_beorn::Card::fetch_all(set).unwrap()
    }

    #[test]
    fn test_get_image_urls_normal_card() {
        let hob_cards = load_hall_of_beorn();
        let brand_son_of_bain = octgn::Card {
            id: "2b75792d-5873-4fc6-9272-d20dd517d36b".to_string(),
            name: "Brand son of Bain".to_string(),
            back_name: None,
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
            back_name: None,
        };
        let octgn_cards = vec![fire_drake];

        let card_downloads = get_image_urls(&octgn_cards, &hob_cards);
        assert_eq!(card_downloads.len(), 1);

        let card = card_downloads.get(0).unwrap();
        assert_eq!(&card.id, "42a5a608-0699-4cd5-b69d-f7c3413cd5cd");
        assert_eq!(&card.front_url, "https://s3.amazonaws.com/hallofbeorn-resources/Images/Cards/The-Wilds-of-Rhovanion/Fire-drake.jpg");
        assert!(&card.back_url.is_none());
    }

    #[test]
    fn test_get_image_urls_quest_card() {
        let hob_cards = load_hall_of_beorn();
        let traveling_north = octgn::Card {
            id: "b4b0bdc8-5edb-40c9-a9ca-dac60fe7cb38".to_string(),
            name: "Traveling North".to_string(),
            back_name: Some("Traveling North".to_string()),
        };
        let octgn_cards = vec![traveling_north];

        let card_downloads = get_image_urls(&octgn_cards, &hob_cards);
        assert_eq!(card_downloads.len(), 1);

        let card = card_downloads.get(0).unwrap();
        assert_eq!(&card.id, "b4b0bdc8-5edb-40c9-a9ca-dac60fe7cb38");
        assert_eq!(&card.front_url, "https://s3.amazonaws.com/hallofbeorn-resources/Images/Cards/The-Wilds-of-Rhovanion/Traveling-North-1A.jpg");
        assert!(&card.back_url.is_some());

        let result = card.back_url.as_ref().unwrap();
        assert_eq!(result, "https://s3.amazonaws.com/hallofbeorn-resources/Images/Cards/The-Wilds-of-Rhovanion/Traveling-North-1B.jpg");
    }

    #[test]
    fn test_get_image_urls_back_name_card() {
        let hob_cards = load_hall_of_beorn();
        let woodman_village = octgn::Card {
            id: "1d4d59f4-def5-4c9e-ba3f-8a28e7f66c73".to_string(),
            name: "Woodman Village".to_string(),
            back_name: Some("Haldan".to_string()),
        };
        let octgn_cards = vec![woodman_village];

        let card_downloads = get_image_urls(&octgn_cards, &hob_cards);
        assert_eq!(card_downloads.len(), 1);

        let card = card_downloads.get(0).unwrap();
        assert_eq!(&card.id, "1d4d59f4-def5-4c9e-ba3f-8a28e7f66c73");
        assert_eq!(&card.front_url, "https://s3.amazonaws.com/hallofbeorn-resources/Images/Cards/The-Wilds-of-Rhovanion/Woodmen-Village.jpg");
        assert!(&card.back_url.is_some());

        let result = card.back_url.as_ref().unwrap();
        assert_eq!(result, "https://s3.amazonaws.com/hallofbeorn-resources/Images/Cards/The-Wilds-of-Rhovanion/Haldan.jpg");
    }

    #[test]
    fn test_fetch_images_normal() {
        let tmp_dir = TempDir::new("lotr-test").unwrap();
        let set_id = "e37145f0-8970-48d3-93bc-cef612226bda";
        let card_id = "2b75792d-5873-4fc6-9272-d20dd517d36b";
        let brand_son_of_bain = CardDownload {
            id: card_id.to_string(),
            front_url: format!("{}/Images/Cards/Brand-son-of-Bain.jpg", mockito::SERVER_URL),
            back_url: None,
        };
        let _m = mock("GET", "/Images/Cards/Brand-son-of-Bain.jpg")
            .with_header("content-type", "image/jpeg")
            .with_body(card_id)
            .create();

        let cards = vec![brand_son_of_bain];

        let result = fetch_images(&tmp_dir.path(), set_id, &cards);
        assert!(result.is_ok());

        let image_path = &tmp_dir
            .path()
            .join(octgn::LOTR_ID)
            .join("Sets")
            .join(&set_id)
            .join("Cards")
            .join(format!("{}.jpg", card_id));
        let mut file = File::open(&image_path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        assert_eq!(card_id, content);
    }

    #[test]
    fn test_fetch_images_back() {
        let tmp_dir = TempDir::new("lotr-test").unwrap();
        let set_id = "e37145f0-8970-48d3-93bc-cef612226bda";
        let card_id = "1d4d59f4-def5-4c9e-ba3f-8a28e7f66c73";
        let woodman_village = CardDownload {
            id: card_id.to_string(),
            front_url: format!("{}/Images/Cards/Woodmen-village.jpg", mockito::SERVER_URL),
            back_url: Some(format!("{}/Images/Cards/Haldan.jpg", mockito::SERVER_URL)),
        };
        let _m = mock("GET", "/Images/Cards/Woodmen-village.jpg")
            .with_header("content-type", "image/jpeg")
            .with_body("Woodmen Village")
            .create();
        let _m2 = mock("GET", "/Images/Cards/Haldan.jpg")
            .with_header("content-type", "image/jpeg")
            .with_body("Haldan")
            .create();

        let cards = vec![woodman_village];

        let result = fetch_images(&tmp_dir.path(), set_id, &cards);
        assert!(result.is_ok());

        let image_path = &tmp_dir
            .path()
            .join(octgn::LOTR_ID)
            .join("Sets")
            .join(&set_id)
            .join("Cards")
            .join(format!("{}.jpg", card_id));
        let mut file = File::open(&image_path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        assert_eq!(content, "Woodmen Village");

        let image_path = &tmp_dir
            .path()
            .join(octgn::LOTR_ID)
            .join("Sets")
            .join(&set_id)
            .join("Cards")
            .join(format!("{}.B.jpg", card_id));
        let mut file = File::open(&image_path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        assert_eq!(content, "Haldan");
    }

    #[test]
    fn test_guess_hob_card() {
        let hob_cards = load_hall_of_beorn();
        let fire_drake = guess_hob_card(&hob_cards, "Fire Drake");
        assert_eq!(fire_drake.title, "Fire-drake");
    }

    #[test]
    fn test_sets() {
        let _m = hob_mocks::card_sets().unwrap();
        let result = sets();
        assert!(result.is_ok());

        let card_sets = result.unwrap();
        let mut title_check = HashMap::new();
        title_check.insert(0, "Core Set");
        title_check.insert(1, "The Hunt for Gollum");
        // check for accents
        title_check.insert(7, "Khazad-dum");
        // check for missing beginning article
        title_check.insert(21, "Voice of Isengard");
        title_check.insert(52, "The Hobbit - Over Hill and Under Hill");
        title_check.insert(61, "The Massing at Osgiliath");
        for (index, name) in title_check {
            assert_eq!(card_sets.get(index).unwrap().name, name);
        }
    }
}
