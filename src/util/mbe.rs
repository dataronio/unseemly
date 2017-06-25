#![macro_use]
/*
March By Example: a user-friendly way to handle named subterms under Kleene stars,
 expressed through a special kind of environment.
This is intended to organize the subterms of an `Ast` node.
Using this as the evaluation environment of a program is probably interesting,
 but not necessarily in a good way.


The principle, when applied to pattern-based macro definitions, is as follows:
 Kleene stars in a macro grammar
   (e.g. `(f=Identifier (arg=Identifier ":" arg_t=Type)*)` )
  correspond to lists in an AST.
 The original syntactic structure is irrelevant.
  but there's only one name (e.g. `arg_t`) for the entire list of matched syntax.

 But that's okay:
  We will let the user put Kleene stars inside syntax quotation (macro transcription).
  The named pieces of syntax under the Kleene star control the number of repetitions:
   if each identifier either repeats `n` times or wasn't matched under a `*` at all,
    the star repeats `n` times, with the repeated syntax "marching in lockstep",
    and the non-`*`ed syntax duplicated

This whole thing nicely generalizes to nesting: we use variable-arity trees instead of lists.

This also generalizes outside the context of transcription:
 we will store an environment mapping names to variable-arity trees,
  and provide an operation ("march") that, given a set of names
    ("driving names", presumably the names "under the `*`")
   produces `n` environments, in which each of those names has a tree
    that is shorter by one level.

One problem: what if two of the "driving names" repeat a different numbers of times?
Traditionally, this is a runtime error,
 but we'd like it to be a parser error:
  after all, it's the author of the syntax
   who has control over how many repetions there are of each thing.
So, we will let grammars specify when different Kleene stars
 must repeat the same number of times.
Violations of this rule are a parse error,
 and it's only legal to "march" with driving names
  that were matched (at the current level)
   (a) under the same `*`, or
   (b) under `*`s that must repeat the same number of times.
On the other hand, if the user wants "cross product" behavior,
 there's no reason that they can't get it.
We may add a facility to take syntax matched `a`, `b`, `c`... times,
 and produce `a × b × c` different environments.


This is based on Macro By Example, but this implementation isn't strictly about macros,
 which is why I changed the name!
The original MBE paper is
 "Macro-by-example: Deriving syntactic transformations from their specifications"
  by Kohlbecker and Wand
  ftp://www.cs.indiana.edu/pub/techreports/TR206.pdf

Many interesting macros can be defined simply
 by specifying a grammar and a piece of quoted syntax,
 if the syntax transcription supports MBE.
 (This corresponds to Scheme's `syntax-rules` and Rust's `macro-rules`.)
*/

/*
Suppose we want to write code that processes MBE environments.
Obviously, we can use `march` to pull out all the structure as necessary.
But pattern-matching is really nice...
 and sometimes it's nice to abstract over the number of repetitions of something.

So, if you set a particular index is `ddd`, that will be repeated 0 or more times
 in order to match the length of whatever is on the other side.
*/


use util::assoc::Assoc;
use name::*;
use std::rc::Rc;
use std::fmt;

/**
 `EnvMBE` is like an environment,
  except that some of its contents are "repeats",
   which represent _n_ different values
   (or repeats of repeats, etc.).
 Non-repeated values may be retrieved by `get_leaf`.
 To access repeated values, one must `march` them,
  which produces _n_ different environments,
   in which the marched values are not repeated (or one layer less repeated).
 Marching multiple repeated values at once
  is only permitted if they were constructed to repeat the same number of times.

*/

custom_derive! {
    // `Clone` needs to traverse the whole `Vec` ):
    #[derive(Eq, Clone, Reifiable)]
    pub struct EnvMBE<T> {
        /// Non-repeated values
        leaves: Assoc<Name, T>,

        /// Outer vec holds distinct repetitions
        ///  (i.e. differently-named, or entirely unnamed repetitions)
        /// Note that some of the entries may be obsolete;
        ///  deletions are marked by putting `None` in the `Assoc`s
        ///   that index into this.
        repeats: Vec<Rc<Vec<EnvMBE<T>>>>,

        /// Which, if any, index is supposed to match 0 or more repetitions of something?
        /// This should always have the same length as `repeats`.
        /// If this isn't all `None`, then this MBE is presumably some kind of pattern.
        ddd_rep_idxes: Vec<Option<usize>>,

        /// Where in `repeats` to look, if we want to traverse for a particular leaf.
        /// We use `.unwrap_or(None)` when looking up into this
        ///  so we can delete by storing `None`.
        leaf_locations: Assoc<Name, Option<usize>>,

        /// The location in `repeats` that represents a specific repetition name.
        named_repeats: Assoc<Name, Option<usize>>
    }
}

impl <T: PartialEq> PartialEq for EnvMBE<T> {
   fn eq(&self, other: &EnvMBE<T>) -> bool {
       fn assoc_eq_modulo_none<K : PartialEq + Clone, V: PartialEq>
               (lhs: &Assoc<K, Option<V>>, rhs: &Assoc<K, Option<V>>)
               -> bool {
           for (k, v_maybe) in lhs.iter_pairs() {
               if let Some(ref v) = *v_maybe {
                   if let Some(&Some(ref other_v)) = rhs.find(k) {
                       if !(v == other_v) { return false; }
                   } else { return false; }
               }
           }

           for (other_k, other_v_maybe) in rhs.iter_pairs() {
               if let &Some(ref other_v) = other_v_maybe {
                   if let Some(&Some(ref v)) = rhs.find(other_k) {
                       if !(v == other_v) { return false; }
                   } else { return false; }
               }
           }

           true
       }

       // This ought to handle permutations of `repeats`
       // (matched with permutations of the indices in the assocs)
       // but that's hard.

       self.leaves == other.leaves
       && self.repeats == other.repeats
       && self.ddd_rep_idxes == other.ddd_rep_idxes
       && assoc_eq_modulo_none(&self.leaf_locations, &other.leaf_locations)
       && assoc_eq_modulo_none(&self.named_repeats, &other.named_repeats)
   }
}

impl<T: Clone + fmt::Debug> fmt::Debug for EnvMBE<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.leaves.empty() && self.repeats.is_empty() {
            write!(f, "mbe∅")
        } else {
            try!(write!(f, "mbe{{ 🍂 {:?}, ✶[", self.leaves));
            let mut first = true;
            for (i, rep) in self.repeats.iter().enumerate() {
                if !first { try!(write!(f, ", ")); }
                first = false;

                // is it a named repeat?
                for (name, idx_maybe) in self.named_repeats.iter_pairs() {
                    if let Some(idx) = *idx_maybe {
                        if idx == i { try!(write!(f, "({:?}) ", name)); }
                    }
                }
                try!(write!(f, "{:?}", rep));
            }
            write!(f, "]}}")
        }
    }
}

/// An iterator that expands a dotdotdot a certain number of times.
struct DddIter<'a, S: 'a> {
    underlying: ::std::slice::Iter<'a, S>,
    cur_idx: usize,
    rep_idx: usize,
    repeated: Option<&'a S>,
    extra_needed: usize
}

impl<'a, S: Clone> DddIter<'a, S> {
    fn new(und: ::std::slice::Iter<'a, S>, rep_idx: usize, extra: usize) -> DddIter<'a, S> {
        DddIter {
            underlying: und,
            cur_idx: 0,
            rep_idx: rep_idx,
            repeated: None,
            extra_needed: extra
        }
    }
}

impl<'a, S: Clone> Iterator for DddIter<'a, S> {
    type Item = &'a S;
    fn next(&mut self) -> Option<&'a S> {
        let cur_idx = self.cur_idx;
        self.cur_idx += 1;

        if cur_idx == self.rep_idx {
            self.repeated = self.underlying.next();
        }
        if cur_idx >= self.rep_idx && cur_idx < self.rep_idx + self.extra_needed {
            return self.repeated;
        } else {
            return self.underlying.next();
        }
    }
}

impl<T: Clone> EnvMBE<T> {
    pub fn new() -> EnvMBE<T> {
        EnvMBE {
            leaves: Assoc::new(),
            repeats: vec![],
            ddd_rep_idxes: vec![],
            leaf_locations: Assoc::new(),
            named_repeats: Assoc::new()
        }
    }

    /// Creates an `EnvMBE` without any repetition
    pub fn new_from_leaves(l: Assoc<Name, T>) -> EnvMBE<T> {
        EnvMBE {
            leaves: l,
            repeats: vec![],
            ddd_rep_idxes: vec![],
            leaf_locations: Assoc::new(),
            named_repeats: Assoc::new()
        }
    }

    /// Creates an `EnvMBE` containing a single anonymous repeat
    pub fn new_from_anon_repeat(r: Vec<EnvMBE<T>>) -> EnvMBE<T> {
        let mut res = EnvMBE::new();
        res.add_anon_repeat(r, None);
        res
    }

    /// Creates an `EnvMBE` containing a single anonymous repeat
    pub fn new_from_anon_repeat_ddd(r: Vec<EnvMBE<T>>, ddd_idx: Option<usize>) -> EnvMBE<T> {
        let mut res = EnvMBE::new();
        res.add_anon_repeat(r, ddd_idx);
        res
    }


    /// Creates an `EnvMBE` containing a single named repeat
    pub fn new_from_named_repeat(n: Name, r: Vec<EnvMBE<T>>) -> EnvMBE<T> {
        let mut res = EnvMBE::new();
        res.add_named_repeat(n, r, None);
        res
    }

    /// Combine two `EnvMBE`s whose names (both environment names and repeat names) are disjoint,
    /// or just overwrite the contents of the previous one.
    /// This should maybe not be `pub` if we can avoid it.
    /// Note: ideally, the larger one should be on the LHS.
    pub fn combine_overriding(&self, rhs: &EnvMBE<T>) -> EnvMBE<T> {
        let adjust_rhs_by = self.repeats.len();

        let mut new_repeats = self.repeats.clone();
        new_repeats.append(&mut rhs.repeats.clone());

        let mut new__ddd_rep_idxes = self.ddd_rep_idxes.clone();
        new__ddd_rep_idxes.append(&mut rhs.ddd_rep_idxes.clone());

        EnvMBE {
            leaves: self.leaves.set_assoc(&rhs.leaves),
            repeats: new_repeats,
            ddd_rep_idxes: new__ddd_rep_idxes,
            leaf_locations: self.leaf_locations.set_assoc(
                &rhs.leaf_locations.map(&|idx_opt| idx_opt.map(|idx| idx+adjust_rhs_by))),
            named_repeats: self.named_repeats.set_assoc(
                &rhs.named_repeats.map(&|idx_opt| idx_opt.map(|idx| idx+adjust_rhs_by)))
        }
    }

    /// Combine two `EnvMBE`s whose leaves should be disjoint, but which can contain
    /// named repeats with the same name. This should make sense for combining the results of
    /// matching two different chunks of a patern.
    pub fn merge(&self, rhs: &EnvMBE<T>) -> EnvMBE<T> {
        let mut res = self.clone();

        let mut rhs_idx_is_named : Vec<bool> = rhs.repeats.iter().map(|_| false).collect();

        // This could be made more efficient by just reusing the `Rc`s instead of cloning the
        // arrays, but that would require reworking the interface.

        for (n, rep_idx) in rhs.named_repeats.iter_pairs() {
            if let Some(rep_idx) = *rep_idx {
                res.add_named_repeat(
                    *n, (*rhs.repeats[rep_idx]).clone(), rhs.ddd_rep_idxes[rep_idx]);
                rhs_idx_is_named[rep_idx] = true;
            }
        }

        for (idx, (rep, ddd_rep_idx)) in
                rhs.repeats.iter().zip(rhs.ddd_rep_idxes.iter()).enumerate() {

            if !rhs_idx_is_named[idx] {
                res.add_anon_repeat((**rep).clone(), *ddd_rep_idx);
            }
        }

        res.leaves = res.leaves.set_assoc(&rhs.leaves);

        res
    }

    /// Given `driving_names`, marches the whole set of names that can march with them.
    /// (Adding an additional name to `driving_names` will result in the same behavior,
    ///  or a panic, in the case that the new name can't be marched with the existing ones.)
    ///
    /// This takes a `Vec` of `Name` instead of just one because a particular name might
    /// not be transcribed at all here, and thus can't tell us how to repeat.
    pub fn march_all(&self, driving_names: &[Name]) -> Vec<EnvMBE<T>> {
        let mut first_march : Option<(usize, Name)> = None;

        for &n in driving_names {
            match (first_march, self.leaf_locations.find(&n).unwrap_or(&None)) {
                 (_, &None) => {}
                 (None, &Some(loc)) => { first_march = Some((loc, n)) }
                 (Some((old_loc, old_name)), &Some(new_loc)) => {
                     if old_loc != new_loc {
                         panic!("{:?} and {:?} cannot march together; they weren't matched to have the same number of repeats",
                                old_name, n);
                     }
                 }
            }
        }

        let march_loc = match first_march {
            None => { panic!("None of {:?} are repeated.", driving_names) }
            Some((loc, _)) => loc
        };

        let mut result = vec![];
        for marched_out in self.repeats[march_loc].iter() {
            result.push(self.combine_overriding(marched_out));
        }

        result
    }

    /// Get a non-repeated thing in the enviornment
    pub fn get_leaf(&self, n: &Name) -> Option<&T> {
        self.leaves.find(n)
    }

    pub fn get_rep_leaf(&self, n: &Name) -> Option<Vec<&T>> {
        let mut res = vec![];
        let leaf_loc = match self.leaf_locations.find(n) {
            Some(ll) => ll, None => { return None; }
        };
        for r in &*self.repeats[leaf_loc.unwrap()] {
            match r.get_leaf(n) {
                Some(leaf) => { res.push(leaf) }
                None => { return None; }
            }
        }
        Some(res)
    }


    /// Extend with a non-repeated thing
    pub fn add_leaf(&mut self, n: Name, v: T) {
        self.leaves = self.leaves.set(n, v);
    }

    pub fn add_named_repeat(&mut self, n: Name, sub: Vec<EnvMBE<T>>, sub_ddd_idx: Option<usize>) {
        if sub.is_empty() { return; } // no-op-ish, but keep the repeats clean (good for `eq`)

        match *self.named_repeats.find(&n).unwrap_or(&None) {
            None => {
                let new_index = self.repeats.len();
                self.update_leaf_locs(new_index, &sub);

                self.repeats.push(Rc::new(sub));
                self.ddd_rep_idxes.push(sub_ddd_idx);
                self.named_repeats = self.named_repeats.set(n, Some(new_index));
            }
            Some(idx) => {
                if self.repeats[idx].len() != sub.len() {
                    panic!("Named repetition {:?} is repeated {:?} times in one place, {:?} times in another.",
                        n, self.repeats[idx].len(), sub.len())
                }

                self.update_leaf_locs(idx, &sub);

                let mut new_repeats_at_idx = vec![];
                for pairs in self.repeats[idx].iter().zip(sub.iter()) {
                    new_repeats_at_idx.push(pairs.0.combine_overriding(pairs.1));
                }
                self.repeats[idx] = Rc::new(new_repeats_at_idx);
                if self.ddd_rep_idxes[idx] != sub_ddd_idx {
                    // Maybe we should support this usecase!
                    panic!("Named repetition {:?} has mismatched ddd rep indices {:?} and {:?}.",
                           n, self.ddd_rep_idxes[idx], sub_ddd_idx);
                }
            }
        }
    }

    pub fn add_anon_repeat(&mut self, sub: Vec<EnvMBE<T>>, sub_ddd_idx: Option<usize>) {
        if sub.is_empty() { return; } // no-op-ish, but keep the repeats clean (good for `eq`)

        let new_index = self.repeats.len();
        self.update_leaf_locs(new_index, &sub);

        self.repeats.push(Rc::new(sub));
        self.ddd_rep_idxes.push(sub_ddd_idx);
    }

    pub fn anonimize_repeat(&mut self, n: Name) {
        // Now you can't find me!
        self.named_repeats = self.named_repeats.set(n, None);
    }


    pub fn map<NewT>(&self, f: &Fn(&T) -> NewT) -> EnvMBE<NewT> {
        EnvMBE {
            leaves: self.leaves.map(f),
            repeats: self.repeats.iter().map(
                &|rc_vec_mbe : &Rc<Vec<EnvMBE<T>>>| Rc::new(rc_vec_mbe.iter().map(
                    &|mbe : &EnvMBE<T>| mbe.map(f)
                ).collect())).collect(),
            ddd_rep_idxes: self.ddd_rep_idxes.clone(),
            leaf_locations: self.leaf_locations.clone(),
            named_repeats: self.named_repeats.clone()
        }
    }

    // TODO: for efficiency, this ought to return iterators
    fn resolve_ddd<'a>(lhs: &'a Rc<Vec<EnvMBE<T>>>, lhs_ddd: &'a Option<usize>,
                           rhs: &'a Rc<Vec<EnvMBE<T>>>, rhs_ddd: &'a Option<usize>)
            -> Vec<(&'a EnvMBE<T>, &'a EnvMBE<T>)> {

        let len_diff = lhs.len() as i32 - (rhs.len() as i32);

        let matched: Vec<(&EnvMBE<T>, &EnvMBE<T>)> = match (lhs_ddd, rhs_ddd) {
            (&None, &None) => {
                if len_diff != 0 { panic!("mismatched MBE lengths") }
                lhs.iter().zip(rhs.iter()).collect()
            }
            (&Some(ddd_idx), &None) => {
                if len_diff - 1 > 0 { panic!("abstract MBE LHS too long") }
                DddIter::new(lhs.iter(), ddd_idx, -(len_diff - 1) as usize)
                    .zip(rhs.iter()).collect()
            }
            (&None, &Some(ddd_idx)) => {
                if len_diff + 1 < 0 { panic!("abstract MBE RHS too long") }
                lhs.iter().zip(
                    DddIter::new(rhs.iter(), ddd_idx, (len_diff + 1) as usize)).collect()
            }
            (&Some(_), &Some(_)) => panic!("repetition on both sides")
        };

        matched
    }

    pub fn map_with<NewT: Clone>(&self, o: &EnvMBE<T>, f: &Fn(&T, &T) -> NewT)
            -> EnvMBE<NewT> {
        EnvMBE {
            leaves: self.leaves.map_with(&o.leaves, f),
            repeats:
            self.repeats.iter().zip(self.ddd_rep_idxes.iter())
                .zip(o.repeats.iter().zip(o.ddd_rep_idxes.iter())).map(

                &|((rc_vec_mbe, ddd_idx), (o_rc_vec_mbe, o_ddd_idx)) :
                  ((&Rc<Vec<EnvMBE<T>>>, &Option<usize>), (&Rc<Vec<EnvMBE<T>>>, &Option<usize>))| {

                    let mapped : Vec<_>
                        = Self::resolve_ddd(rc_vec_mbe, ddd_idx, o_rc_vec_mbe, o_ddd_idx).iter()
                        .map(&|&(mbe, o_mbe) : &(&EnvMBE<T>, &EnvMBE<T>)| mbe.map_with(o_mbe, f))
                        .collect();

                  Rc::new(mapped)}).collect(),
            ddd_rep_idxes: self.repeats.iter().map(|_| None).collect(), // remove all dotdotdots
            leaf_locations: self.leaf_locations.clone(),
            named_repeats: self.named_repeats.clone()
        }
    }

    pub fn map_reduce_with<NewT: Clone>(&self,  other: &EnvMBE<T>,
            f: &Fn(&T, &T) -> NewT, red: &Fn(&NewT, &NewT) -> NewT, base: NewT) -> NewT {
        // TODO: this panics all over the place if anything goes wrong
        let mut reduced : NewT = self.leaves.map_with(&other.leaves, f)
            .reduce(&|_k, v, res| red(v, &res), base);

        let mut already_processed : Vec<bool> = self.repeats.iter().map(|_| false).collect();

        for (leaf_name, self_idx) in self.leaf_locations.iter_pairs() {
            let self_idx = match *self_idx {
                Some(si) => si, None => { continue; }
            };
            if already_processed[self_idx] { continue; }
            already_processed[self_idx] = true;

            let other_idx = other.leaf_locations.find_or_panic(leaf_name).unwrap();

            let matched = Self::resolve_ddd(
                &self.repeats[self_idx], &self.ddd_rep_idxes[self_idx],
                &other.repeats[other_idx], &other.ddd_rep_idxes[other_idx]);

            for (self_elt, other_elt) in matched {
                reduced = self_elt.map_reduce_with(other_elt, f, &red, reduced);
            }
        }

        reduced
    }

    fn update_leaf_locs(&mut self, idx: usize, sub: &[EnvMBE<T>]) {
        let mut already_placed_leaves = ::std::collections::HashSet::<Name>::new();
        let mut already_placed_repeats = ::std::collections::HashSet::<Name>::new();

        for sub_mbe in sub {
            for leaf_name in sub_mbe.leaf_locations.iter_keys()
                    .chain(sub_mbe.leaves.iter_keys()) {
                if !already_placed_leaves.contains(&leaf_name) {
                    self.leaf_locations = self.leaf_locations.set(leaf_name, Some(idx));
                    already_placed_leaves.insert(leaf_name);
                }
            }
            for repeat_name in sub_mbe.named_repeats.iter_keys() {
                if !already_placed_repeats.contains(&repeat_name) {
                    self.named_repeats = self.named_repeats.set(repeat_name, Some(idx));
                    already_placed_repeats.insert(repeat_name);
                }
            }
        }
    }
}

impl<T: Clone, E: Clone> EnvMBE<Result<T, E>> {
    // Is `lift` the right term?
    pub fn lift_result(&self) -> Result<EnvMBE<T>, E> {
        // There's probably a nice and elegant way to do this with Result::from_iter, but not today
        let mut leaves : Assoc<Name, T> = Assoc::new();
        for (k,v) in self.leaves.iter_pairs() {
            leaves = leaves.set(*k,try!((*v).clone()));
        }

        let mut repeats = vec![];
        for rep in &self.repeats {
            let mut items = vec![];
            for item in &**rep {
                items.push(try!(item.lift_result()));
            }
            repeats.push(Rc::new(items));
        }


        Ok(EnvMBE {
            leaves: leaves,
            repeats: repeats,
            ddd_rep_idxes: self.ddd_rep_idxes.clone(),
            leaf_locations: self.leaf_locations.clone(),
            named_repeats: self.named_repeats.clone()
        })
    }

}


impl<T: Clone + fmt::Debug> EnvMBE<T> {
    pub fn get_leaf_or_panic(&self, n: &Name) -> &T {
        match self.leaves.find(n) {
            Some(v) => { v }
            None => { panic!(" {:?} not found in {:?} (perhaps it is still repeated?)", n, self) }
        }
    }

    pub fn get_rep_leaf_or_panic(&self, n: &Name) -> Vec<&T> {
        let mut res = vec![];
        for r in &*self.repeats[self.leaf_locations.find_or_panic(n).unwrap()] {
            res.push(r.get_leaf_or_panic(n))
        }
        res
    }
}

#[test]
fn basic_mbe() {
    let mut mbe = EnvMBE::new();
    mbe.add_leaf(n("eight"), 8 as i32);
    mbe.add_leaf(n("nine"), 9);

    assert!(mbe != EnvMBE::new());
    assert!(EnvMBE::new() != mbe);

    let teens_mbe = vec![
        EnvMBE::new_from_leaves(assoc_n!("t" => 11)),
        EnvMBE::new_from_leaves(assoc_n!("t" => 12)),
        EnvMBE::new_from_leaves(assoc_n!("t" => 13))
    ];

    mbe.add_named_repeat(n("low_two_digits"), teens_mbe, None);

    let big_mbe = vec![
        EnvMBE::new_from_leaves(assoc_n!("y" => 9001)),
        EnvMBE::new_from_leaves(assoc_n!("y" => 9002))
    ];

    mbe.add_anon_repeat(big_mbe, None);


    for (sub_mbe, teen) in mbe.march_all(&vec![n("t"), n("eight")]).iter().zip(vec![11,12,13]) {
        assert_eq!(sub_mbe.get_leaf(&n("eight")), Some(&8));
        assert_eq!(sub_mbe.get_leaf(&n("nine")), Some(&9));
        assert_eq!(sub_mbe.get_leaf(&n("t")), Some(&teen));
        assert_eq!(sub_mbe.get_leaf(&n("y")), None);

        for (sub_sub_mbe, big) in sub_mbe.march_all(&vec![n("y"), n("eight")]).iter().zip(vec![9001, 9002]) {
            assert_eq!(sub_sub_mbe.get_leaf(&n("eight")), Some(&8));
            assert_eq!(sub_sub_mbe.get_leaf(&n("nine")), Some(&9));
            assert_eq!(sub_sub_mbe.get_leaf(&n("t")), Some(&teen));
            assert_eq!(sub_sub_mbe.get_leaf(&n("y")), Some(&big));
        }
    }

    let neg_teens_mbe = vec![
        EnvMBE::new_from_leaves(assoc_n!("nt" => -11)),
        EnvMBE::new_from_leaves(assoc_n!("nt" => -12)),
        EnvMBE::new_from_leaves(assoc_n!("nt" => -13))
    ];

    mbe.add_named_repeat(n("low_two_digits"), neg_teens_mbe, None);

    for (sub_mbe, teen) in mbe.march_all(&vec![n("t"), n("nt"), n("eight")]).iter().zip(vec![11,12,13]) {
        assert_eq!(sub_mbe.get_leaf(&n("eight")), Some(&8));
        assert_eq!(sub_mbe.get_leaf(&n("nine")), Some(&9));
        assert_eq!(sub_mbe.get_leaf(&n("t")), Some(&teen));
        assert_eq!(sub_mbe.get_leaf(&n("nt")), Some(&-teen));

        for (sub_sub_mbe, big) in sub_mbe.march_all(&vec![n("y"), n("eight")]).iter().zip(vec![9001, 9002]) {
            assert_eq!(sub_sub_mbe.get_leaf(&n("eight")), Some(&8));
            assert_eq!(sub_sub_mbe.get_leaf(&n("nine")), Some(&9));
            assert_eq!(sub_sub_mbe.get_leaf(&n("t")), Some(&teen));
            assert_eq!(sub_sub_mbe.get_leaf(&n("nt")), Some(&-teen));
            assert_eq!(sub_sub_mbe.get_leaf(&n("y")), Some(&big));
        }
    }

    let all_zeroes = mbe.map_with(&mbe, &|a, b| a - b);
    for sub_mbe in all_zeroes.march_all(&vec![n("t"), n("nt"), n("eight")]) {
        assert_eq!(sub_mbe.get_leaf(&n("eight")), Some(&0));
        assert_eq!(sub_mbe.get_leaf(&n("nine")), Some(&0));
        assert_eq!(sub_mbe.get_leaf(&n("t")), Some(&0));
        assert_eq!(sub_mbe.get_leaf(&n("nt")), Some(&0));

        for (sub_sub_mbe, _) in sub_mbe.march_all(&vec![n("y"), n("eight")]).iter().zip(vec![9001, 9002]) {
            assert_eq!(sub_sub_mbe.get_leaf(&n("eight")), Some(&0));
            assert_eq!(sub_sub_mbe.get_leaf(&n("nine")), Some(&0));
            assert_eq!(sub_sub_mbe.get_leaf(&n("t")), Some(&0));
            assert_eq!(sub_sub_mbe.get_leaf(&n("nt")), Some(&0));
            assert_eq!(sub_sub_mbe.get_leaf(&n("y")), Some(&0));
        }
    }

    assert_eq!(mbe, mbe);
    assert!(mbe != mbe.map(&|x| x - 1));
    assert_eq!(mbe, mbe.map(&|x| x - 0));
    assert!(mbe != EnvMBE::new());
    assert!(EnvMBE::new() != mbe);

    assert_eq!(mbe, mbe.map_with(&all_zeroes, &|a,b| a+b));
    assert_eq!(mbe, all_zeroes.map_with(&mbe, &|a,b| a+b));

    assert_eq!(
        mbe.map_reduce_with(&all_zeroes, &|a,b| if *a<*b { *a } else { *b }, &|a, b| (*a+*b), 0),
        -11 + -12 + -13);

    assert_eq!(
        Err(()),
        mbe.clone().map(&|x: &i32| if *x == 12     { Err(()) } else { Ok(*x)} ).lift_result());
    assert_eq!(
        Ok(mbe.clone()),
        mbe.clone().map(&|x: &i32| if *x == 121212 { Err(()) } else { Ok(*x)} ).lift_result());


    let mapped_mbe = mbe.map(&|x : &i32| (*x, *x - 9000));

    let first_sub_mbe = &mapped_mbe.march_all(&vec![n("y")])[0];

    assert_eq!(first_sub_mbe.get_leaf(&n("y")), Some(&(9001, 1)));
    assert_eq!(first_sub_mbe.get_leaf(&n("eight")), Some(&(8, (8 - 9000))));
    assert_eq!(first_sub_mbe.get_leaf(&n("x")), None);
}

#[test]
fn ddd_iter() {
    assert_eq!(DddIter::new([0,1,2].iter(), 0, 0).collect::<Vec<_>>(), [&1,&2]);
    assert_eq!(DddIter::new([0,1,2].iter(), 1, 0).collect::<Vec<_>>(), [&0,&2]);
    assert_eq!(DddIter::new([0,1,2].iter(), 2, 0).collect::<Vec<_>>(), [&0,&1]);

    assert_eq!(DddIter::new([0,1,2].iter(), 1, 1).collect::<Vec<_>>(), [&0,&1,&2]);
    assert_eq!(DddIter::new([0,1,2].iter(), 1, 3).collect::<Vec<_>>(), [&0,&1,&1,&1,&2]);
}

#[test]
fn mbe_ddd_map_with() {
    use ast::{Ast, Atom};

    let lhs = mbe!( "a" => ["0", "1", "2", "3", "4"] );
    let rhs = mbe!( "a" => ["0" ...("1")..., "4"] );

    fn concat(l: &Ast, r: &Ast) -> Ast {
        match (l, r) {
            (&Atom(ln), &Atom(rn)) => Atom(n( format!("{}{}", ln, rn).as_str() )),
            _ => panic!()
        }
    }

    assert_eq!(lhs.map_with(&rhs, &concat),
               mbe!( "a" => ["00", "11", "21", "31", "44"] ));
    assert_eq!(rhs.map_with(&lhs, &concat),
               mbe!( "a" => ["00", "11", "12", "13", "44"] ));

    assert_eq!(lhs.map_reduce_with(&rhs, &concat, &concat, ast!("")),
               ast!("4431211100")); // N.B. order is arbitrary


    let lhs = mbe!( "a" => [["a", "b"], ["c", "d"], ["c", "d"]]);
    let rhs = mbe!( "a" => [["a", "b"] ...(["c", "d"])...]);

    assert_eq!(lhs.map_with(&rhs, &concat),
               mbe!( "a" => [["aa", "bb"], ["cc", "dd"], ["cc", "dd"]]));
}
