use common::data::{action::Action, identifier::Identifier};
use predicates::{prelude::*, BoxPredicate};
use sha2::{Digest, Sha256};

use crate::{
    error::Error,
    interface::{FilterArgs, UserIdentifier},
};

// TODO: Fixed predicates
// TODO: Vec of comp types
pub struct FilterPredicates {
    predicates: Vec<BoxPredicate<Action>>,
}

impl FilterPredicates {
    pub fn eval(&self, action: &Action) -> bool {
        self.predicates.iter().all(|p| p.eval(action))
    }
}

impl TryFrom<FilterArgs> for FilterPredicates {
    type Error = Error;

    fn try_from(value: FilterArgs) -> Result<Self, Self::Error> {
        let mut predicates = Vec::new();

        add_filter(&mut predicates, value.after, |a, time| a.time > time);
        add_filter(&mut predicates, value.before, |a, time| a.time < time);
        add_filter(&mut predicates, value.colors, |a, index| index == a.index);
        add_filter(&mut predicates, value.regions, |a, region| {
            region.contains(a.x, a.y)
        });
        add_filter(&mut predicates, value.action_kinds, |a, kind| {
            kind.0 == a.kind
        });
        add_filter(&mut predicates, value.users, |a, user| match user {
            UserIdentifier::Key(key) => compare_action_to_key(&key, a),
            UserIdentifier::Username(name) => a.user == name,
        });

        if predicates.is_empty() {
            Err(Error::Config("no filters specified".to_string()))
        } else {
            Ok(FilterPredicates { predicates })
        }
    }
}

fn add_filter<I, T, F>(vec: &mut Vec<BoxPredicate<Action>>, iter: I, func: F)
where
    I: IntoIterator<Item = T>,
    T: Clone + Sync + Send + 'static,
    F: Copy + Sync + Send + Fn(&Action, T) -> bool + 'static,
{
    iter.into_iter().for_each(|item| {
        vec.push(predicate::function::<_, Action>(move |a| func(a, item.clone())).boxed())
    })
}

fn compare_action_to_key(key: &str, action: &Action) -> bool {
    let time = action.time.format("%Y-%m-%d %H:%M:%S,%3f").to_string();
    if let Identifier::Hash(hash) = &action.user {
        let mut hasher = Sha256::new();
        hasher.update(time.as_bytes());
        hasher.update(",");
        hasher.update(action.x.to_string().as_bytes());
        hasher.update(",");
        hasher.update(action.y.to_string().as_bytes());
        hasher.update(",");
        hasher.update(action.index.to_string().as_bytes());
        hasher.update(",");
        hasher.update(key.as_bytes());
        let raw = hasher.finalize();
        let digest = hex::encode(raw);
        &digest[..] == hash
    } else {
        false
    }
}
