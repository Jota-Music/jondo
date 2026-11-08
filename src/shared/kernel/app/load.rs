use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

/// A remote resource: absent, in flight, failed, or ready.
///
/// One shape for every fetch keeps the page code from inventing a new trio of
/// `value`/`loading`/`error` fields per entity.
#[derive(Clone, Debug)]
pub struct Load<T> {
    pub value: Option<T>,
    pub loading: bool,
    pub error: Option<String>,
}

impl<T> Default for Load<T> {
    fn default() -> Self {
        Self {
            value: None,
            loading: false,
            error: None,
        }
    }
}

impl<T> Load<T> {
    pub fn ready(value: T) -> Self {
        Self {
            value: Some(value),
            loading: false,
            error: None,
        }
    }

    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            value: None,
            loading: false,
            error: Some(error.into()),
        }
    }

    pub fn begin(&mut self) {
        self.loading = true;
        self.error = None;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn settle(&mut self, result: Result<T, String>) {
        match result {
            Ok(value) => {
                self.value = Some(value);
                self.error = None;
            }
            Err(error) => {
                self.value = None;
                self.error = Some(error);
            }
        }
        self.loading = false;
    }
}

impl<T> Load<Vec<T>> {
    pub fn list(&self) -> &[T] {
        self.value.as_deref().unwrap_or_default()
    }

    pub fn empty(&self) -> bool {
        self.list().is_empty()
    }
}

/// Keyed [`Load`]s: what a page asks for by id, fetched once unless it failed.
#[derive(Clone, Debug)]
pub struct Pending<K, V> {
    entries: HashMap<K, Load<V>>,
}

impl<K, V> Default for Pending<K, V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<K: Eq + Hash + Clone, V> Pending<K, V> {
    pub fn value<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries.get(key).and_then(|entry| entry.value.as_ref())
    }

    pub fn value_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries
            .get_mut(key)
            .and_then(|entry| entry.value.as_mut())
    }

    pub fn entry<Q>(&self, key: &Q) -> Option<&Load<V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries.get(key)
    }

    pub fn loading<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries.get(key).is_some_and(|entry| entry.loading)
    }

    pub fn failed<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries
            .get(key)
            .is_some_and(|entry| entry.error.is_some())
    }

    /// True while the resource must not be requested again: ready, in flight
    /// or already failed.
    pub fn waiting<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries
            .get(key)
            .is_some_and(|entry| entry.value.is_some() || entry.loading || entry.error.is_some())
    }

    /// Marks the fetch in flight. Returns false when the resource is already
    /// ready, loading or failed, so the caller can skip the request.
    pub fn begin<Q>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized + ToOwned<Owned = K>,
    {
        if self.waiting(key) {
            return false;
        }
        self.entries.entry(key.to_owned()).or_default().begin();
        true
    }

    pub fn ready(&mut self, key: K, value: V) {
        self.entries.insert(key, Load::ready(value));
    }

    pub fn fail(&mut self, key: K, error: impl Into<String>) {
        self.entries.insert(key, Load::failed(error));
    }

    /// Drops the children of a parent, whose ids are no longer valid.
    pub fn remove<Q>(&mut self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.entries.remove(key);
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&K) -> bool) {
        self.entries.retain(|key, _| keep(key));
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries
            .values()
            .filter_map(|entry| entry.value.as_ref())
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries
            .values_mut()
            .filter_map(|entry| entry.value.as_mut())
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries
            .iter()
            .filter_map(|(key, entry)| entry.value.as_ref().map(|value| (key, value)))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
