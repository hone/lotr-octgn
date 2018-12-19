pub mod lotr {
    pub mod hall_of_beorn {
        pub const SEARCH: &str = "fixtures/lotr/hob/search.json";
        pub const CARD_SETS: &str = "fixtures/lotr/hob/card_sets.json";
    }

    pub mod octgn {
        pub const BASE: &str = "fixtures/lotr/octgn";
        pub const SETS: &str = "fixtures/lotr/octgn/o8g/Sets";

        pub fn set(name: &str) -> String {
            format!("{}/{}/set.xml", SETS, name)
        }
    }
}

pub mod arkham_horror {
    pub mod octgn {
        pub const SETS: &str = "fixtures/arkham-horror/octgn/o8g/Sets";

        pub fn set(name: &str) -> String {
            format!("{}/{}/set.xml", SETS, name)
        }
    }
}
