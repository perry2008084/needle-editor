#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Region {
    pub a: usize,
    pub b: usize,
}

impl Region {
    pub fn new(a: usize, b: usize) -> Self {
        Self { a, b }
    }

    pub fn begin(&self) -> usize {
        self.a.min(self.b)
    }

    pub fn end(&self) -> usize {
        self.a.max(self.b)
    }

    pub fn caret(&self) -> usize {
        self.b
    }

    pub fn size(&self) -> usize {
        self.end() - self.begin()
    }

    pub fn empty(&self) -> bool {
        self.a == self.b
    }

    pub fn contains(&self, point: usize) -> bool {
        point >= self.begin() && point <= self.end()
    }

    pub fn intersects(&self, other: &Region) -> bool {
        self.begin() <= other.end() && other.begin() <= self.end()
    }

    pub fn cover(&self, other: &Region) -> Region {
        Region::new(self.begin().min(other.begin()), self.end().max(other.end()))
    }
}

#[derive(Debug, Clone, Default)]
pub struct SelectionSet {
    regions: Vec<Region>,
}

impl SelectionSet {
    pub fn new() -> Self {
        Self { regions: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.regions.clear();
    }

    pub fn add(&mut self, region: Region) {
        self.regions.push(region);
        self.normalize();
    }

    pub fn add_all<I>(&mut self, regions: I)
    where
        I: IntoIterator<Item = Region>,
    {
        self.regions.extend(regions);
        self.normalize();
    }

    pub fn subtract(&mut self, region: Region) {
        self.regions.retain(|r| *r != region);
    }

    pub fn len(&self) -> usize {
        self.regions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Region> {
        self.regions.iter()
    }

    pub fn iter_rev(&self) -> impl DoubleEndedIterator<Item = &Region> {
        self.regions.iter().rev()
    }

    pub fn first(&self) -> Option<&Region> {
        self.regions.first()
    }

    pub fn last(&self) -> Option<&Region> {
        self.regions.last()
    }

    pub fn as_slice(&self) -> &[Region] {
        &self.regions
    }

    fn normalize(&mut self) {
        self.regions.sort_by_key(|region| (region.begin(), region.end()));

        let mut merged: Vec<Region> = Vec::with_capacity(self.regions.len());
        for region in self.regions.drain(..) {
            match merged.last_mut() {
                Some(last) if last.intersects(&region) || last.end() == region.begin() => {
                    *last = last.cover(&region);
                }
                _ => merged.push(region),
            }
        }

        self.regions = merged;
    }
}

#[cfg(test)]
mod tests {
    use super::{Region, SelectionSet};

    #[test]
    fn selection_merges_overlapping_regions() {
        let mut selection = SelectionSet::new();
        selection.add(Region::new(1, 4));
        selection.add(Region::new(3, 8));

        assert_eq!(selection.len(), 1);
        assert_eq!(selection.first(), Some(&Region::new(1, 8)));
    }

    #[test]
    fn region_caret_returns_b_side() {
        let region = Region::new(9, 3);
        assert_eq!(region.caret(), 3);
    }
}
