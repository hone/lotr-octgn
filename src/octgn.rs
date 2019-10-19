use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::Read;

use roxmltree::Document;
use walkdir::WalkDir;

pub const LOTR_ID: &str = "a21af4e8-be4b-4cda-a6b6-534f9717391f";
pub const ARKHAM_HORROR_ID: &str = "a6d114c7-2e2a-4896-ad8c-0330605c90bf";

#[derive(Debug)]
pub struct NoMatchingGameError {
    game_id: String,
}

impl std::error::Error for NoMatchingGameError {}

impl NoMatchingGameError {
    pub fn new(game_id: &str) -> Self {
        Self {
            game_id: game_id.to_string(),
        }
    }
}

impl fmt::Display for NoMatchingGameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "This is not a valid Game ID: '{}'.", self.game_id)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Game {
    LOTR,
    ArkhamHorror,
}

impl Game {
    fn from(s: &str) -> Option<Self> {
        match s {
            LOTR_ID => Some(Game::LOTR),
            ARKHAM_HORROR_ID => Some(Game::ArkhamHorror),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct PropertyMissingError {
    property: String,
}

impl std::error::Error for PropertyMissingError {}

impl fmt::Display for PropertyMissingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not find property '{}'.", self.property)
    }
}

impl PropertyMissingError {
    pub fn new(property: &str) -> Self {
        PropertyMissingError {
            property: property.to_owned(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct AttributeMissingError {
    tag: String,
    attribute: String,
}

impl std::error::Error for AttributeMissingError {}

impl fmt::Display for AttributeMissingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<{}> is missing attribute '{}'.",
            self.tag, self.attribute
        )
    }
}

impl AttributeMissingError {
    pub fn new(tag: &str, attribute: &str) -> Self {
        Self {
            tag: tag.to_string(),
            attribute: attribute.to_string(),
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub back_name: Option<String>,
}

#[derive(Debug)]
pub struct Set {
    pub id: String,
    pub name: String,
    pub cards: Vec<Card>,
    pub game: Game,
}

impl Set {
    #![allow(clippy::new_ret_no_self)]
    pub fn new(doc: &Document) -> Result<Set, Box<dyn std::error::Error>> {
        let node = doc.root().first_child().unwrap();
        let atts = attributes(node.attributes());
        let id = atts
            .get("id")
            .ok_or_else(|| AttributeMissingError::new(node.tag_name().name(), "id"))?
            .to_string();
        let name = atts
            .get("name")
            .ok_or_else(|| AttributeMissingError::new(node.tag_name().name(), "name"))?
            .to_string();
        let game_id = atts
            .get("gameId")
            .ok_or_else(|| AttributeMissingError::new(node.tag_name().name(), "gameId"))?;
        let game = Game::from(game_id).ok_or_else(|| NoMatchingGameError::new(game_id))?;

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
            id,
            name,
            cards,
            game,
        })
    }

    pub fn fetch_all(folder: &std::path::Path) -> Result<Vec<Set>, Box<dyn std::error::Error>> {
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
                let file = File::open(&path)?;
                let mut reader = std::io::BufReader::new(file);
                let mut xml = String::new();
                reader.read_to_string(&mut xml)?;
                let doc = Document::parse(&xml)?;
                Set::new(&doc)
            })
            .collect::<Vec<Result<Set, Box<dyn std::error::Error>>>>();

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
    use crate::tests::fixtures;

    use std::{fs::File, io::Read, path::Path};

    #[test]
    fn test_new_lotr() {
        let mut file = File::open(fixtures::lotr::octgn::set("The Wilds of Rhovanion")).unwrap();
        let mut xml = String::new();
        file.read_to_string(&mut xml).unwrap();
        let doc = Document::parse(&xml).unwrap();

        let set = Set::new(&doc).unwrap();
        assert_eq!(&set.name, "The Wilds of Rhovanion");
        assert_eq!(&set.id, "e37145f0-8970-48d3-93bc-cef612226bda");
        assert_eq!(set.game, Game::LOTR);
        // Woodman Village is 2 cards. Backside is Haldan
        assert_eq!(set.cards.len(), 79);
    }

    #[test]
    fn test_new_ah() {
        let mut file = File::open(fixtures::arkham_horror::octgn::set("Dunwich Legacy")).unwrap();
        let mut xml = String::new();
        file.read_to_string(&mut xml).unwrap();
        let doc = Document::parse(&xml).unwrap();

        let set = Set::new(&doc).unwrap();
        assert_eq!(&set.name, "Dunwich Legacy");
        assert_eq!(&set.id, "dfa9b3bf-58f2-4611-ae55-e25562726d62");
        assert_eq!(set.game, Game::ArkhamHorror);
        assert_eq!(set.cards.len(), 109);
    }

    #[test]
    fn test_new_missing_id() {
        let xml = r#"
<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<set xmlns:noNamespaceSchemaLocation="CardSet.xsd" name="The Wilds of Rhovanion" gameId="a21af4e8-be4b-4cda-a6b6-534f9717391f" gameVersion="2.3.6.0" version="1.0.0">"
 <cards>
 </cards>
</set>"#;
        let doc = Document::parse(&xml).unwrap();
        let result = Set::new(&doc);

        assert!(result.is_err());
        assert_eq!(
            *result
                .unwrap_err()
                .downcast::<AttributeMissingError>()
                .unwrap(),
            AttributeMissingError::new("set", "id")
        );
    }

    #[test]
    fn test_new_missing_name() {
        let xml = r#"
<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<set xmlns:noNamespaceSchemaLocation="CardSet.xsd" gameId="a21af4e8-be4b-4cda-a6b6-534f9717391f" id="e37145f0-8970-48d3-93bc-cef612226bda" gameVersion="2.3.6.0" version="1.0.0">"
 <cards>
 </cards>
</set>"#;
        let doc = Document::parse(&xml).unwrap();
        let result = Set::new(&doc);

        assert!(result.is_err());
        assert_eq!(
            *result
                .unwrap_err()
                .downcast::<AttributeMissingError>()
                .unwrap(),
            AttributeMissingError::new("set", "name")
        );
    }

    #[test]
    fn test_new_missing_game_id() {
        let xml = r#"
<?xml version="1.0" encoding="utf-8" standalone="yes"?>
<set xmlns:noNamespaceSchemaLocation="CardSet.xsd" name="The Wilds of Rhovanion" id="e37145f0-8970-48d3-93bc-cef612226bda" gameVersion="2.3.6.0" version="1.0.0">
 <cards>
 </cards>
</set>"#;
        let doc = Document::parse(&xml).unwrap();
        let result = Set::new(&doc);

        assert!(result.is_err());
        assert_eq!(
            *result
                .unwrap_err()
                .downcast::<AttributeMissingError>()
                .unwrap(),
            AttributeMissingError::new("set", "gameId")
        );
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
    fn test_fetch_all() {
        let dir = Path::new(fixtures::lotr::octgn::SETS);
        let result = Set::fetch_all(&dir);
        assert!(result.is_ok());

        let sets = result.unwrap();
        assert_eq!(sets.len(), 107);
    }

    #[test]
    fn test_fetch_all_err() {
        let dir = Path::new("fixtures/lotr");
        let result = Set::fetch_all(&dir);
        assert!(result.is_err());
    }
}
