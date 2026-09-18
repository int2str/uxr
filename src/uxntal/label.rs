#[derive(Debug, Clone)]
pub struct Label {
    name: String,
    address: u16,
    references: u16,
}

impl Label {
    pub fn new(name: &str, address: u16) -> Self {
        Self {
            name: name.to_string(),
            address,
            references: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn address(&self) -> u16 {
        self.address
    }

    pub fn references(&self) -> u16 {
        self.references
    }

    pub fn increment_references(&mut self) {
        self.references = self.references.saturating_add(1);
    }
}

impl PartialEq for Label {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
