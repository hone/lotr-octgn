#[derive(Serialize, Deserialize)]
pub struct Pack {
    pub name: String,
    pub code: String,
    pub position: u32,
    pub cycle_position: u32,
    pub available: String,
    pub known: u32,
    pub total: u32,
    pub url: String,
    pub id: u32,
}

impl Pack {
    fn fetch_all() -> Vec<Pack> {}
}
