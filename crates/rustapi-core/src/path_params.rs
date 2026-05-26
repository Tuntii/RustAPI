//! Path parameter types.

use std::collections::HashMap;

/// Path parameters collection.
#[derive(Debug, Clone, Default)]
pub struct PathParams {
    inner: Vec<(String, String)>,
}

impl PathParams {
    /// Create a new empty path params collection.
    #[inline]
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Create path params with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    /// Insert a key-value pair.
    #[inline]
    pub fn insert(&mut self, key: String, value: String) {
        self.inner.push((key, value));
    }

    /// Get a value by key.
    #[inline]
    pub fn get(&self, key: &str) -> Option<&String> {
        self.inner.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    /// Check if a key exists.
    #[inline]
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.iter().any(|(k, _)| k == key)
    }

    /// Check if the collection is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the number of parameters.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Iterate over key-value pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.inner.iter().map(|(k, v)| (k, v))
    }

    /// Convert to a HashMap (for backwards compatibility).
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        self.inner.iter().cloned().collect()
    }
}

impl FromIterator<(String, String)> for PathParams {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<'a> FromIterator<(&'a str, &'a str)> for PathParams {
    fn from_iter<I: IntoIterator<Item = (&'a str, &'a str)>>(iter: I) -> Self {
        Self {
            inner: iter
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }
}

impl From<HashMap<String, String>> for PathParams {
    fn from(map: HashMap<String, String>) -> Self {
        Self {
            inner: map.into_iter().collect(),
        }
    }
}

impl From<PathParams> for HashMap<String, String> {
    fn from(params: PathParams) -> Self {
        params.inner.into_iter().collect()
    }
}

impl<'a> IntoIterator for &'a PathParams {
    type Item = &'a (String, String);
    type IntoIter = std::slice::Iter<'a, (String, String)>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_params() {
        let mut params = PathParams::new();
        params.insert("id".to_string(), "123".to_string());
        params.insert("name".to_string(), "test".to_string());

        assert_eq!(params.get("id"), Some(&"123".to_string()));
        assert_eq!(params.get("name"), Some(&"test".to_string()));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_many_params() {
        let mut params = PathParams::new();
        for i in 0..10 {
            params.insert(format!("key{}", i), format!("value{}", i));
        }

        assert_eq!(params.len(), 10);
    }

    #[test]
    fn test_from_iterator() {
        let params: PathParams = [("a", "1"), ("b", "2"), ("c", "3")].into_iter().collect();

        assert_eq!(params.get("a"), Some(&"1".to_string()));
        assert_eq!(params.get("b"), Some(&"2".to_string()));
        assert_eq!(params.get("c"), Some(&"3".to_string()));
    }

    #[test]
    fn test_to_hashmap_conversion() {
        let mut params = PathParams::new();
        params.insert("id".to_string(), "42".to_string());

        let map = params.to_hashmap();
        assert_eq!(map.get("id"), Some(&"42".to_string()));
    }
}
