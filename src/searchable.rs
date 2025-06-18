use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

type SearchableFn<T> = dyn FnMut(&&T, &str) -> bool;

pub trait SearchableItem {
    fn search_text(&self) -> &str;
}

pub struct Searchable<T>
where
    T: Clone + SearchableItem,
{
    sort_by_levenshtein: bool,
    vec: Vec<T>,
    matcher: SkimMatcherV2,
    filter: Box<SearchableFn<T>>,
    filtered: Vec<T>,
}

impl<T> Searchable<T>
where
    T: Clone + SearchableItem,
{
    #[must_use]
    pub fn new<P>(sort_by_levenshtein: bool, vec: Vec<T>, search_value: &str, predicate: P) -> Self
    where
        P: FnMut(&&T, &str) -> bool + 'static,
    {
        let mut searchable = Self {
            sort_by_levenshtein,
            vec,
            matcher: SkimMatcherV2::default(),
            filter: Box::new(predicate),
            filtered: Vec::new(),
        };
        searchable.search(search_value);
        searchable
    }

    pub fn search(&mut self, value: &str) {
        if value.is_empty() {
            self.filtered.clone_from(&self.vec);
            return;
        }

        let mut items: Vec<_> = self
            .vec
            .iter()
            .filter(|host| (self.filter)(host, value))
            .map(|item| {
                let score = self.matcher.fuzzy_match(item.search_text(), value).unwrap_or(0);
                (item.clone(), score)
            })
            .collect();

        // Sort by Levenshtein distance in descending order (higher score = better match)
        if self.sort_by_levenshtein {
            items.sort_by(|a, b| b.1.cmp(&a.1));
        }

        self.filtered = items.into_iter().map(|(item, _)| item).collect();
    }

    #[allow(clippy::must_use_candidate)]
    pub fn len(&self) -> usize {
        self.filtered.len()
    }

    #[allow(clippy::must_use_candidate)]
    pub fn is_empty(&self) -> bool {
        self.filtered.is_empty()
    }

    pub fn non_filtered_iter(&self) -> std::slice::Iter<T> {
        self.vec.iter()
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.filtered.iter()
    }
}

impl<'a, T> IntoIterator for &'a Searchable<T>
where
    T: Clone + SearchableItem,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.filtered.iter()
    }
}

impl<T> std::ops::Index<usize> for Searchable<T>
where
    T: Clone + SearchableItem,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.filtered[index]
    }
}
