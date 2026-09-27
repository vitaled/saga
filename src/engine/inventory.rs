//! The player inventory.

/// An ordered set of item identifiers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inventory {
    items: Vec<String>,
}

impl Inventory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an item. Items are unique: adding twice is a no-op.
    pub fn add(&mut self, item: impl Into<String>) -> bool {
        let item = item.into();
        if self.contains(&item) {
            return false;
        }
        self.items.push(item);
        true
    }

    /// Removes an item, returning whether it was present.
    pub fn remove(&mut self, item: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|candidate| candidate != item);
        before != self.items.len()
    }

    pub fn contains(&self, item: &str) -> bool {
        self.items.iter().any(|candidate| candidate == item)
    }

    pub fn items(&self) -> &[String] {
        &self.items
    }

    pub fn get(&self, index: usize) -> Option<&String> {
        self.items.get(index)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_are_unique_and_keep_insertion_order() {
        let mut inventory = Inventory::new();
        assert!(inventory.add("rope"));
        assert!(inventory.add("key"));
        assert!(!inventory.add("rope"));
        assert_eq!(inventory.items(), ["rope", "key"]);
    }

    #[test]
    fn removing_reports_whether_the_item_was_present() {
        let mut inventory = Inventory::new();
        inventory.add("key");
        assert!(inventory.remove("key"));
        assert!(!inventory.remove("key"));
        assert!(inventory.is_empty());
    }
}
