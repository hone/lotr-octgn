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
    let octgn_map = octgn_cards.iter().fold(HashMap::new(), |mut acc, card| {
        acc.insert(&card.name, &card.id);

        acc
    });

    hob_cards
        .par_iter()
        .map(|hob_card| {
            let octgn_id = match octgn_map.get(&hob_card.title) {
                Some(octgn_id) => octgn_id.to_string(),
                None => {
                    let (octgn_id, octgn_name) = error_card_match_id(&octgn_cards, &hob_card);
                    println!(
                        "Warning: Could not find {}, using {} instead.",
                        &hob_card.title, octgn_name
                    );

                    octgn_id
                }
            };

            CardDownload {
                id: octgn_id.to_string(),
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

fn error_card_match_id(
    octgn_cards: &Vec<octgn::Card>,
    error_card: &hall_of_beorn::Card,
) -> (String, String) {
    let distance_map = octgn_cards
        .iter()
        .fold(HashMap::new(), |mut acc, octgn_card| {
            acc.insert(
                &octgn_card.id,
                (
                    &octgn_card.name,
                    strsim::levenshtein(&error_card.title, &octgn_card.name),
                ),
            );

            acc
        });

    let card = distance_map
        .iter()
        .min_by_key(|(_, &value)| value.1)
        .unwrap();

    (card.0.to_string(), (card.1).0.to_string())
}

fn main() {
    let mut file = File::open("set.xml").unwrap();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();

    let doc = Document::parse(&text).unwrap();
    let set = octgn::Set::new(doc);
    println!("{}: {}", set.name, set.id);
    let hob_cards = hall_of_beorn::fetch(&set.name).unwrap();
    let card_downloads = get_image_urls(&set.cards, &hob_cards);
    fetch_images(&set.id, &card_downloads).unwrap();
    zip_directory("lotr", &format!("{}.o8c", set.name).replace(" ", "-")).unwrap();
}
