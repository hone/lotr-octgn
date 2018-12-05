use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use roxmltree::Document;
use walkdir::WalkDir;

pub const LOTR_ID: &'static str = "a21af4e8-be4b-4cda-a6b6-534f9717391f";

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub back_name: Option<String>,
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Set {
    pub id: String,
    pub name: String,
    pub cards: Vec<Card>,
}

impl Set {
    pub fn new(doc: &Document) -> Self {
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
                let back_name = card_node
                    .children()
                    .find(|child| child.is_element() && child.tag_name().name() == "alternate")
                    .map(|alternative_node| {
                        let atts = attributes(alternative_node.attributes());
                        atts.get("name").unwrap().to_string()
                    });
                Card {
                    id: atts.get("id").unwrap().to_string(),
                    name: atts.get("name").unwrap().to_string(),
                    back_name: back_name,
                }
            }).collect();

        Self {
            id: atts.get("id").unwrap().to_string(),
            name: atts.get("name").unwrap().to_string(),
            cards: cards,
        }
    }

    pub fn fetch_all(folder: &str) -> Result<Vec<Set>, Box<std::error::Error>> {
        let sets = WalkDir::new(folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter_map(|entry| {
                let path = entry.path();

                path.extension()
                    .filter(|extension| extension.to_str().unwrap() == "xml")
                    .and_then(|_| {
                        let mut file = File::open(&path).unwrap();
                        let mut xml = String::new();
                        file.read_to_string(&mut xml).unwrap();
                        let doc = Document::parse(&xml).unwrap();
                        Some(Set::new(&doc))
                    })
            }).collect();

        Ok(sets)
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

    use std::fs::File;
    use std::io::Read;

    #[test]
    fn test_new() {
        let mut file = File::open("fixtures/set.xml").unwrap();
        let mut xml = String::new();
        file.read_to_string(&mut xml).unwrap();
        let doc = Document::parse(&xml).unwrap();

        let set = Set::new(&doc);
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
        let set = Set::new(&doc);

        let card = set.cards.get(0).unwrap();
        assert!(card.back_name.is_some());
    }

    #[test]
    fn test_all() {
        let result = Set::fetch_all("fixtures/octgn/o8g/Sets");
        assert!(result.is_ok());

        let sets = result.unwrap();
        assert_eq!(sets.len(), 107);
    }
}
