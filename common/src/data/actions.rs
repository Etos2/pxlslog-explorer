use super::{
    action::{Action, Index},
    actionkind::ActionKind,
    identifier::Identifier,
};

pub struct ActionsIterator<'a> {
    actions: &'a Actions,
    i: usize,
}

impl<'a> Iterator for ActionsIterator<'a> {
    type Item = ActionsView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let out = self.actions.get_view(self.i);
        self.i += 1;
        out
    }
}

pub struct ActionsView<'a> {
    pub time: i64,
    pub user: Option<&'a Identifier>,
    pub coord: (u32, u32),
    pub index: Option<Index>,
    pub kind: Option<ActionKind>,
}

// todo: Change to i64 rather than NaiveDateTime?
#[derive(Clone, Debug)]
pub struct Actions {
    pub time: Vec<i64>,
    pub user: Option<Vec<Identifier>>,
    pub coord: Vec<(u32, u32)>,
    pub index: Option<Vec<Index>>,
    pub kind: Option<Vec<ActionKind>>,
    pub bounds: (u32, u32, u32, u32),
}

#[derive(Default, Clone, Debug)]
pub struct ActionsBuilder {
    time: Vec<i64>,
    user: Vec<Identifier>,
    coord: Vec<(u32, u32)>,
    index: Vec<Index>,
    kind: Vec<ActionKind>,
    bounds: (u32, u32, u32, u32),
}

impl Actions {
    pub fn get_view(&self, i: usize) -> Option<ActionsView<'_>> {
        Some(ActionsView {
            time: *self.time.get(i)?,
            user: self.user.as_ref().map(|v| v.get(i)).unwrap_or_default(),
            coord: self.coord.get(i).cloned()?,
            index: self
                .index
                .as_ref()
                .map(|v| v.get(i))
                .unwrap_or_default()
                .cloned(),
            kind: self
                .kind
                .as_ref()
                .map(|v| v.get(i))
                .unwrap_or_default()
                .cloned(),
        })
    }

    pub fn iter(&self) -> ActionsIterator {
        ActionsIterator {
            actions: self,
            i: 0,
        }
    }
}

impl ActionsBuilder {
    pub fn new() -> ActionsBuilder {
        ActionsBuilder {
            bounds: (u32::MAX, u32::MAX, u32::MIN, u32::MIN),
            ..Default::default()
        }
    }

    pub fn push(&mut self, action: Action) -> &mut Self {
        self.time.push(action.time);
        if let Some(a_user) = action.user {
            self.user.push(a_user);
        }
        self.coord.push((action.x, action.y));
        if let Some(a_index) = action.index {
            self.index.push(a_index);
        }
        if let Some(a_kind) = action.kind {
            self.kind.push(a_kind);
        }

        self.bounds.0 = std::cmp::min(action.x, self.bounds.0);
        self.bounds.1 = std::cmp::min(action.y, self.bounds.1);
        self.bounds.2 = std::cmp::max(action.x, self.bounds.2);
        self.bounds.3 = std::cmp::max(action.y, self.bounds.3);

        self
    }

    pub fn build(mut self) -> Actions {
        self.bounds.2 += 1;
        self.bounds.3 += 1;

        Actions {
            time: self.time,
            user: (!self.user.is_empty()).then_some(self.user),
            coord: self.coord,
            index: (!self.index.is_empty()).then_some(self.index),
            kind: (!self.kind.is_empty()).then_some(self.kind),
            bounds: self.bounds,
        }
    }
}
