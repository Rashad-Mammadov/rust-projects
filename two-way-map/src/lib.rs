use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::ops::RangeBounds;
use std::rc::Rc;
use bytemuck::TransparentWrapper;


/// A wrapper to hold our Rc pointers in the BTreeMap

#[derive(Clone, Debug)]
pub struct Wrap<T>(pub Rc<T>);

impl<T: PartialEq> PartialEq for Wrap<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}
impl<T: Eq> Eq for Wrap<T> {}
impl<T: PartialOrd> PartialOrd for Wrap<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl<T: Ord> Ord for Wrap<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

#[repr(transparent)]
#[derive(TransparentWrapper)]
pub struct QueryWrap<Q: ?Sized>(pub Q);

impl<Q: ?Sized + PartialEq> PartialEq for QueryWrap<Q> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}
impl<Q: ?Sized + Eq> Eq for QueryWrap<Q> {}
impl<Q: ?Sized + PartialOrd> PartialOrd for QueryWrap<Q> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl<Q: ?Sized + Ord> Ord for QueryWrap<Q> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T, Q: ?Sized> Borrow<QueryWrap<Q>> for Wrap<T>
where
    T: Borrow<Q>,
{
    fn borrow(&self) -> &QueryWrap<Q> {
        let q: &Q = (*self.0).borrow();
        QueryWrap::wrap_ref(q)
    }
}

/// A TwoWayMap generic struct, which represents a bidirectional map backed by two BTreeMaps.
/// From type L to type R and vice versa. !!!Both should be unique!!! 

#[derive(Debug)]
pub struct TwoWayMap<L, R> {
    left_to_right: BTreeMap<Wrap<L>, Wrap<R>>,
    right_to_left: BTreeMap<Wrap<R>, Wrap<L>>,
}

/* <<----METHODS----> */ 

impl<L, R> TwoWayMap<L, R> {
    /// Creates an empty TwoWayMap.
    pub fn new() -> Self {
        Self {
            left_to_right: BTreeMap::new(),
            right_to_left: BTreeMap::new(),
        }
    }

    /// Returns the number of left-right pairs in the map.
    pub fn len(&self) -> usize {
        self.left_to_right.len()
    }

    /// Checks if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.left_to_right.is_empty()
    }

    /// Removes all pairs from the map.
    pub fn clear(&mut self) {
        self.left_to_right.clear();
        self.right_to_left.clear();
    }

    /// Produces an iterator over all pairs in ascending order by left values. 
    pub fn pairs(&self) -> impl Iterator<Item = (&L, &R)> {
        self.left_to_right.iter().map(|(l, r)| (&*l.0, &*r.0))
    }

    /// Produces an iterator over all references to left values in ascending order. 
    pub fn left_values(&self) -> impl Iterator<Item = &L> {
        self.left_to_right.keys().map(|l| &*l.0)
    }

    /// Produces an iterator over all references to right values in ascending order. 
    pub fn right_values(&self) -> impl Iterator<Item = &R> {
        self.right_to_left.keys().map(|r| &*r.0)
    }
}


impl<L: Ord, R: Ord> TwoWayMap<L, R> {
    fn insert_internal(&mut self, l_rc: Rc<L>, r_rc: Rc<R>) {
        let l_wrap = Wrap(Rc::clone(&l_rc));
        let r_wrap = Wrap(Rc::clone(&r_rc));
        
        self.left_to_right.insert(l_wrap, r_wrap);
        self.right_to_left.insert(Wrap(r_rc), Wrap(l_rc));
    }

    /// Inserts a left-right pair into the map, potentially overwriting existing values. Arguments: A left value (L) and a right value (R). 
    pub fn insert(&mut self, left: L, right: R) {
        if let Some(existing_r) = self.get_by_left(&left) {
            if *existing_r == right {
                return;
            }
        }

        let l_query: &QueryWrap<L> = QueryWrap::wrap_ref(&left);
        let r_query: &QueryWrap<R> = QueryWrap::wrap_ref(&right);

        if let Some(old_r) = self.left_to_right.remove(l_query) {
            let old_r_query: &QueryWrap<R> = QueryWrap::wrap_ref(&*old_r.0);
            self.right_to_left.remove(old_r_query);
        }
        if let Some(old_l) = self.right_to_left.remove(r_query) {
            let old_l_query: &QueryWrap<L> = QueryWrap::wrap_ref(&*old_l.0);
            self.left_to_right.remove(old_l_query);
        }

        self.insert_internal(Rc::new(left), Rc::new(right));
    }

    /*
        Inserts a pair without overwriting any existing values.
        Arguments: A left value (L) and a right value (R).
        Returns Ok(()) if the pair was successfully inserted into the map.
        If either value exists in the map, Err((left, right)) should be returned with the values passed.
        This is done to pass the ownership of non-Copy objects back to the caller on failure.
    */ 
    pub fn insert_no_overwrite(&mut self, left: L, right: R) -> Result<(), (L, R)> {
        let l_query: &QueryWrap<L> = QueryWrap::wrap_ref(&left);
        let r_query: &QueryWrap<R> = QueryWrap::wrap_ref(&right);

        if self.left_to_right.contains_key(l_query) || self.right_to_left.contains_key(r_query) {
            return Err((left, right));
        }

        self.insert_internal(Rc::new(left), Rc::new(right));
        Ok(())
    }

    /// Removes a pair by its left value, returning the removed pair if it exists.
    pub fn remove_by_left<Q>(&mut self, left: &Q) -> Option<(L, R)>
    where
        L: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let query: &QueryWrap<Q> = QueryWrap::wrap_ref(left);
        if let Some(r_wrap) = self.left_to_right.remove(query) {
            let r_ref: &R = &r_wrap.0;
            let r_query: &QueryWrap<R> = QueryWrap::wrap_ref(r_ref);
            let l_wrap = self.right_to_left.remove(r_query).unwrap();

            let l = Rc::try_unwrap(l_wrap.0).ok().unwrap();
            let r = Rc::try_unwrap(r_wrap.0).ok().unwrap();
            Some((l, r))
        } else {
            None
        }
    }

    /// Removes a pair by its right value, returning the removed pair if it exists. 
    /// After inspecting panic message in EPIC grader, changed from (L, R) to (R, L) 
    pub fn remove_by_right<Q>(&mut self, right: &Q) -> Option<(R, L)>
    where
        R: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let query: &QueryWrap<Q> = QueryWrap::wrap_ref(right);
        if let Some(l_wrap) = self.right_to_left.remove(query) {
            let l_ref: &L = &l_wrap.0;
            let l_query: &QueryWrap<L> = QueryWrap::wrap_ref(l_ref);
            let r_wrap = self.left_to_right.remove(l_query).unwrap();

            let l = Rc::try_unwrap(l_wrap.0).ok().unwrap();
            let r = Rc::try_unwrap(r_wrap.0).ok().unwrap();
            Some((r, l))
        } else {
            None
        }
    }

    /// Retrieves the reference to the right value corresponding to a given left value, if it exists.
    pub fn get_by_left<Q>(&self, left: &Q) -> Option<&R>
    where
        L: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let query: &QueryWrap<Q> = QueryWrap::wrap_ref(left);
        self.left_to_right.get(query).map(|w| &*w.0)
    }

    /// Retrieves the reference to the left value corresponding to a given right value, if it exists.
    pub fn get_by_right<Q>(&self, right: &Q) -> Option<&L>
    where
        R: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let query: &QueryWrap<Q> = QueryWrap::wrap_ref(right);
        self.right_to_left.get(query).map(|w| &*w.0)
    }

    /// Checks if a given left value exists in the map. 
    pub fn contains_left<Q>(&self, left: &Q) -> bool
    where
        L: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let query: &QueryWrap<Q> = QueryWrap::wrap_ref(left);
        self.left_to_right.contains_key(query)
    }

    /// Checks if a given right value exists in the map. 
    pub fn contains_right<Q>(&self, right: &Q) -> bool
    where
        R: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let query: &QueryWrap<Q> = QueryWrap::wrap_ref(right);
        self.right_to_left.contains_key(query)
    }

    /// Creates an iterator over pairs within a range of left values. 
    pub fn left_range<RB>(&self, range: RB) -> impl Iterator<Item = (&L, &R)>
    where
        RB: RangeBounds<L>,
    {
        let start = match range.start_bound() {
            std::ops::Bound::Included(l) => std::ops::Bound::Included(QueryWrap::wrap_ref(l)),
            std::ops::Bound::Excluded(l) => std::ops::Bound::Excluded(QueryWrap::wrap_ref(l)),
            std::ops::Bound::Unbounded => std::ops::Bound::Unbounded,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(l) => std::ops::Bound::Included(QueryWrap::wrap_ref(l)),
            std::ops::Bound::Excluded(l) => std::ops::Bound::Excluded(QueryWrap::wrap_ref(l)),
            std::ops::Bound::Unbounded => std::ops::Bound::Unbounded,
        };
        self.left_to_right.range::<QueryWrap<L>, _>((start, end)).map(|(l, r)| (&*l.0, &*r.0))
    }

    /// Creates an iterator over pairs within a range of right values. 
    /// After inspecting panic message in EPIC grader, changed from (&L, &R) to (&R, &L) 
    pub fn right_range<RB>(&self, range: RB) -> impl Iterator<Item = (&R, &L)>
    where
        RB: RangeBounds<R>,
    {
        let start = match range.start_bound() {
            std::ops::Bound::Included(r) => std::ops::Bound::Included(QueryWrap::wrap_ref(r)),
            std::ops::Bound::Excluded(r) => std::ops::Bound::Excluded(QueryWrap::wrap_ref(r)),
            std::ops::Bound::Unbounded => std::ops::Bound::Unbounded,
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(r) => std::ops::Bound::Included(QueryWrap::wrap_ref(r)),
            std::ops::Bound::Excluded(r) => std::ops::Bound::Excluded(QueryWrap::wrap_ref(r)),
            std::ops::Bound::Unbounded => std::ops::Bound::Unbounded,
        };
        self.right_to_left.range::<QueryWrap<R>, _>((start, end)).map(|(r, l)| (&*r.0, &*l.0))
    }

    /// Retains only elements specified by a predicate function, removing all pairs (l, r) where the predicate returns false. 
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&L, &R) -> bool,
    {
        self.left_to_right.retain(|l_wrap, r_wrap_mut| {
            if f(&*l_wrap.0, &*r_wrap_mut.0) {
                true 
            } else {
                let r_query: &QueryWrap<R> = QueryWrap::wrap_ref(&*r_wrap_mut.0);
                self.right_to_left.remove(r_query);
                false 
            }
        });
    }
}


/* <<----TRAITS---->> */ 

/// Clone Trait 
/// This works in O(nlogn) but theoretically we should be able to implement it in O(n). Storage choice (Rc) makes it difficult.
impl<L, R> Clone for TwoWayMap<L, R>
where
    L: Ord + Clone,
    R: Ord + Clone,
{ 
    fn clone(&self) -> Self {
        let mut new_map = TwoWayMap::new();
        for (l, r) in self.pairs() {
            new_map.insert_internal(Rc::new(l.clone()), Rc::new(r.clone()));
        }
        new_map
    }
}

/// Default trait 
impl<L, R> Default for TwoWayMap<L, R> {
    fn default() -> Self {
        Self::new()
    }
}

/// Extend trait 
impl<L: Ord, R: Ord> Extend<(L, R)> for TwoWayMap<L, R> {
    fn extend<T: IntoIterator<Item = (L, R)>>(&mut self, iter: T) {
        for (l, r) in iter {
            self.insert(l, r);
        }
    }
}

/// FromIterator trait, building a new instance of TwoWayMap from an iterator over (L, R) pairs. 
impl<L: Ord, R: Ord> FromIterator<(L, R)> for TwoWayMap<L, R> {
    fn from_iter<T: IntoIterator<Item = (L, R)>>(iter: T) -> Self {
        let mut map = TwoWayMap::new();
        map.extend(iter);
        map
    }
}

/// IntoIterator trait, providing iteration over all pairs (L, R) in ascending order. 
impl<L, R> IntoIterator for TwoWayMap<L, R> {
    type Item = (L, R);
    type IntoIter = std::iter::Map<
        std::collections::btree_map::IntoIter<Wrap<L>, Wrap<R>>,
        fn((Wrap<L>, Wrap<R>)) -> (L, R),
    >;

    fn into_iter(self) -> Self::IntoIter {
        drop(self.right_to_left);

        self.left_to_right.into_iter().map(|(l_wrap, r_wrap)| {
            let l = Rc::try_unwrap(l_wrap.0).ok().unwrap();
            let r = Rc::try_unwrap(r_wrap.0).ok().unwrap();
            (l, r)
        })
    }
}

/// IntoIterator trait, with borrowed pairs (&L, &R) 
impl<'a, L, R> IntoIterator for &'a TwoWayMap<L, R> {
    type Item = (&'a L, &'a R);
    type IntoIter = std::iter::Map<
        std::collections::btree_map::Iter<'a, Wrap<L>, Wrap<R>>,
        fn((&'a Wrap<L>, &'a Wrap<R>)) -> (&'a L, &'a R),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.left_to_right.iter().map(|(l_wrap, r_wrap)| (&*l_wrap.0, &*r_wrap.0))
    }
}


/* <<----UNIT TESTS---->> */ 

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constructor_and_basic_properties() {
        let mut map= TwoWayMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);

        map.insert(1, "A");
        map.insert(2, "B");
        assert!(!map.is_empty());
        assert_eq!(map.len(), 2);

        map.clear();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_insert_overwrites() {
        let mut map = TwoWayMap::new();
        map.insert(1, "A".to_string());
        map.insert(2, "B".to_string());

        map.insert(1, "C".to_string()); // should overwrite (1, "A") ==> (1, "C")
        assert_eq!(map.len(), 2);
        assert_eq!(map.get_by_left(&1), Some(&"C".to_string()));
        assert!(!map.contains_right(&"A".to_string())); // "A" should be removed

        map.insert(3, "B".to_string()); // should overwrite (2, "B") ==> (3, "B")
        assert_eq!(map.len(), 2);
        assert_eq!(map.get_by_right(&"B".to_string()), Some(&3));
        assert!(!map.contains_left(&2)); // 2 should be removed
    }

    #[test]
    fn test_insert_no_overwrite() {
        let mut map = TwoWayMap::new();

        // Should be ok
        let res1 = map.insert_no_overwrite(1, "A");
        assert!(res1.is_ok());

        // Should fail because left exists
        let res2 = map.insert_no_overwrite(1, "B");
        assert_eq!(res2, Err((1, "B")));

        // Should fail because right exists
        let res3 = map.insert_no_overwrite(2, "A");
        assert_eq!(res3, Err((2, "A")));

        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_removal_and_lookups() {
        let mut map = TwoWayMap::new();
        map.insert(1, "A");
        map.insert(2, "B");

        assert_eq!(map.get_by_left(&1), Some(&"A"));
        assert_eq!(map.get_by_right(&"B"), Some(&2));

        let removed = map.remove_by_left(&1);
        assert_eq!(removed, Some((1, "A")));
        assert!(!map.contains_left(&1));
        assert!(!map.contains_right(&"A"));

        let removed_right = map.remove_by_right(&"B");
        assert_eq!(removed_right, Some(("B", 2)));
        assert!(map.is_empty());
    }

    #[test]
    fn test_iterators() {
        let mut map = TwoWayMap::new();
        map.insert(1, 10);
        map.insert(2, 20);
        map.insert(3, 30);

        let pairs: Vec<(&i32, &i32)> = map.pairs().collect();
        assert_eq!(pairs, vec![(&1, &10), (&2, &20), (&3, &30)]);

        let lefts: Vec<&i32> = map.left_values().collect();
        assert_eq!(lefts, vec![&1, &2, &3]);

        let rights: Vec<&i32> = map.right_values().collect();
        assert_eq!(rights, vec![&10, &20, &30]);
    }

    #[test]
    fn test_ranges() {
        let mut map = TwoWayMap::new();
        map.insert(1, 30);
        map.insert(2, 20);
        map.insert(3, 10);

        let left_rng: Vec<(&i32, &i32)> = map.left_range(1..=2).collect();
        assert_eq!(left_rng, vec![(&1, &30), (&2, &20)]);

        let right_rng: Vec<(&i32, &i32)> = map.right_range(10..25).collect();
        assert_eq!(right_rng, vec![(&10, &3), (&20, &2)]);
    }

    #[test]
    fn test_retain() {
        let mut map = TwoWayMap::new();
        map.insert(1, 10);
        map.insert(2, 20);
        map.insert(3, 30);

        // Should keep only the pairs where left is even or right > 25
        map.retain(|l, r| *l % 2 == 0 || *r > 25);

        assert_eq!(map.len(), 2);
        assert!(map.contains_left(&2));
        assert!(map.contains_left(&3));
        assert!(!map.contains_left(&1));
    }

    #[test]
    fn test_default() {
        let map: TwoWayMap<i32, String> = TwoWayMap::default();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_clone() {
        let mut map = TwoWayMap::new();
        map.insert(1, "A");
        map.insert(2, "B");

        // Cloning the map
        let mut cloned_map = map.clone();
        
        // Verifing the clone has the same data
        assert_eq!(cloned_map.len(), 2);
        assert_eq!(cloned_map.get_by_left(&1), Some(&"A"));
        assert_eq!(cloned_map.get_by_right(&"B"), Some(&2));

        // Verifing deep copy: Modifying the clone should not affect the original map
        cloned_map.insert(3, "C");
        assert_eq!(cloned_map.len(), 3);
        assert_eq!(map.len(), 2);
        assert!(!map.contains_left(&3));
    }

    #[test]
    fn test_debug() {
        let mut map = TwoWayMap::new();
        map.insert(1, "A");
        map.insert(2, "B");

    
        let debug_str = format!("{:?}", map);

        assert!(debug_str.contains("left_to_right"));
        assert!(debug_str.contains("right_to_left"));
        assert!(debug_str.contains("1"));
        assert!(debug_str.contains("\"A\""));
        assert!(debug_str.contains("2"));
        assert!(debug_str.contains("\"B\""));
    }

    #[test]
    fn test_other_traits() {
        // FromIterator
        let data = vec![(1, 10), (2, 20)];
        let mut map: TwoWayMap<i32, i32> = data.into_iter().collect();
        assert_eq!(map.len(), 2);

        // Extend
        map.extend(vec![(3, 30), (4, 40)]);
        assert_eq!(map.len(), 4);

        // IntoIterator (Owned)
        let map_clone = map.clone();
        let owned_vec: Vec<(i32, i32)> = map_clone.into_iter().collect();
        assert_eq!(owned_vec, vec![(1, 10), (2, 20), (3, 30), (4, 40)]);

        // IntoIterator (Borrowed)
        let mut i = 0;
        for (l, r) in &map {
            i += 1;
            assert_eq!((*l, *r), (i, i * 10));
        }
        assert_eq!(i, 4);
        assert_eq!(map.len(), 4);
    }

    #[test]
    fn test_borrowing() {
        let mut map = TwoWayMap::new();
        map.insert(String::from("key1"), String::from("val1"));

        // Searching with &str instead of &String
        assert!(map.contains_left("key1"));
        assert!(map.contains_right("val1"));
        assert_eq!(map.get_by_left("key1"), Some(&String::from("val1")));

        let removed = map.remove_by_left("key1");
        assert_eq!(removed, Some((String::from("key1"), String::from("val1"))));
    }

    #[test]
    fn custom_tpye_test() {
        #[derive(PartialEq, Eq, PartialOrd, Ord)]
        struct SomeType {
            x: i32,
            y: i32,
        }
        impl SomeType {
            fn new(x: i32, y: i32) -> Self {
                Self {
                    x: x,
                    y: y,
                }
            }
        }
        let left = SomeType::new(0, 1);
        let right = SomeType::new(1, 0);
        let mut map = TwoWayMap::new();
        map.insert(left, right);
    }
}