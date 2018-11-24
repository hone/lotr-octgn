use std::collections::HashMap;

use roxmltree::Document;

pub const LOTR_ID: &'static str = "a21af4e8-be4b-4cda-a6b6-534f9717391f";

pub struct Card {
    pub id: String,
    pub name: String,
}

pub struct Set {
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
                }
            }).collect();

        Self {
            id: atts.get("id").unwrap().to_string(),
            name: atts.get("name").unwrap().to_string(),
            cards: cards,
        }
    }
}

fn attributes<'a>(atts: &'a [roxmltree::Attribute]) -> HashMap<&'a str, &'a str> {
    atts.iter().fold(HashMap::new(), |mut acc, attribute| {
        acc.insert(attribute.name(), attribute.value());

        acc
    })
}
