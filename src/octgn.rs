use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::Read;

use roxmltree::Document;
use walkdir::WalkDir;

pub const LOTR_ID: &str = "a21af4e8-be4b-4cda-a6b6-534f9717391f";

#[derive(Debug)]
pub struct PropertyMissingError {
    property: String,
}

impl std::error::Error for PropertyMissingError {}

impl PropertyMissingError {
    pub fn new(property: &str) -> Self {
        PropertyMissingError {
            property: property.to_owned(),
        }
    }
}

impl fmt::Display for PropertyMissingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not find property '{}'.", self.property)
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub back_name: Option<String>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Set {
    pub id: String,
    pub name: String,
    pub cards: Vec<Card>,
}

impl Set {
    #![allow(clippy::new_ret_no_self)]
    pub fn new(doc: &Document) -> Result<Set, Box<std::error::Error>> {
        let node = doc.root().first_child().unwrap();
        let atts = attributes(node.attributes());

        let cards_node = node
            .children()
            .find(|child| child.is_element() && child.tag_name().name() == "cards")
            .ok_or_else(|| PropertyMissingError::new("cards"))?;
        let cards = cards_node
            .children()
            .filter(|card_node| !card_node.attributes().is_empty())
            .map(|card_node| {
                let atts = attributes(card_node.attributes());
                let back_name = card_node
                    .children()
                    .find(|child| child.is_element() && child.tag_name().name() == "alternate")
                    .map(|alternative_node| {
                        let atts = attributes(alternative_node.attributes());
                        atts["name"].to_string()
                    });
                Card {
                    id: atts["id"].to_string(),
                    name: atts["name"].to_string(),
                    back_name,
                }
            })
            .collect();

        Ok(Self {
            id: atts["id"].to_string(),
            name: atts["name"].to_string(),
            cards,
        })
    }

    pub fn fetch_all(folder: &std::path::Path) -> Result<Vec<Set>, Box<std::error::Error>> {
        let sets = WalkDir::new(folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|entry| {
                let path = entry.path();

                match path.extension() {
                    None => false,
                    Some(extension) => extension == "xml",
                }
            })
            .map(|entry| {
                let path = entry.path();
                let mut file = File::open(&path)?;
                let mut xml = String::new();
                file.read_to_string(&mut xml)?;
                let doc = Document::parse(&xml)?;
                Set::new(&doc)
            })
            .collect::<Vec<Result<Set, Box<std::error::Error>>>>();

        if sets.iter().any(|result| result.is_err()) {
            Err(sets
                .into_iter()
                .find(|result| result.is_err())
                .unwrap()
                .unwrap_err())
        } else {
            Ok(sets.into_iter().map(|result| result.unwrap()).collect())
        }
    }
}

fn attributes<'a>(atts: &'a [roxmltree::Attribute]) -> HashMap<&'a str, &'a str> {
    atts.iter().fold(HashMap::new(), |mut acc, attribute| {
        acc.insert(attribute.name(), attribute.value());

        acc
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{fs::File, io::Read, path::Path};

    #[test]
    fn test_new() {
        let mut file = File::open("fixtures/set.xml").unwrap();
        let mut xml = String::new();
        file.read_to_string(&mut xml).unwrap();
        let doc = Document::parse(&xml).unwrap();

        let set = Set::new(&doc).unwrap();
        assert_eq!(&set.name, "The Wilds of Rhovanion");
        assert_eq!(&set.id, "e37145f0-8970-48d3-93bc-cef612226bda");
        // Woodman Village is 2 cards. Backside is Haldan
        assert_eq!(set.cards.len(), 79);
    }

    #[test]
    fn test_card_back() {
        let xml = r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<set xmlns:noNamespaceSchemaLocation="CardSet.xsd" name="The Wilds of Rhovanion" id="e37145f0-8970-48d3-93bc-cef612226bda" gameId="a21af4e8-be4b-4cda-a6b6-534f9717391f" gameVersion="2.3.6.0" version="1.0.0">
 <cards>
  <card id="1d4d59f4-def5-4c9e-ba3f-8a28e7f66c73" name="Woodman Village" size="EncounterCard">
    <property name="Card Number" value="69"/>
    <property name="Quantity" value="1"/>
    <property name="Encounter Set" value="Journey Up the Anduin"/>
    <property name="Type" value="Location"/>
    <property name="Traits" value="Riverland."/>
    <property name="Threat" value="4"/>
    <property name="Quest Points" value="4"/>
    <property name="Text" value="Immune to player card effects. Forced: When Woodman Village is explored, add the top card of the Evil Creatures deck to the staging area. Then, flip over Woodman Village and attach Haldan to the just-added enemy as a guarded objective."/>
    <alternate name="Haldan" type="B">
      <property name="Encounter Set" value="Journey Up the Anduin"/>
      <property name="Type" value="Objective Ally"/>
      <property name="Traits" value="Woodman. Scout."/>
      <property name="Willpower" value="2"/>
      <property name="Attack" value="3"/>
      <property name="Defense" value="1"/>
      <property name="Health" value="4"/>
      <property name="Text" value="The first player gains control of Haldan while he is free of encounters. While there is an active location, Haldan does not exhaust to quest. If Haldan leaves play, the players lose the game."/>
    </alternate>
  </card>
 </cards>
</set>"#;
        let doc = Document::parse(&xml).unwrap();
        let set = Set::new(&doc).unwrap();

        let card = set.cards.get(0).unwrap();
        assert!(card.back_name.is_some());
    }

    #[test]
    fn test_all() {
        let dir = Path::new("fixtures/octgn/o8g/Sets");
        let result = Set::fetch_all(&dir);
        assert!(result.is_ok());

        let sets = result.unwrap();
        assert_eq!(sets.len(), 107);
    }

    #[test]
    fn test_all_err() {
        let dir = Path::new("fixtures/octgn");
        let result = Set::fetch_all(&dir);
        assert!(result.is_err());
    }
}
