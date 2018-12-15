pub type ArenaId = u16;
// pub struct ArenaId(u16);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arena<T: PartialEq> {
    data: Vec<T>,
}

impl<T: PartialEq> Arena<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        Arena {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn intern<U: Into<T> + PartialEq<T>>(&mut self, member: U) -> ArenaId {
        self.find(&member).unwrap_or_else(|| {
            let id = self.data.len() as ArenaId;
            self.data.push(member.into());
            id
        })
    }

    pub fn resolve(&self, id: ArenaId) -> &T {
        &self.data[id as usize]
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    pub fn find<U: ?Sized + PartialEq<T>>(&self, it: &U) -> Option<ArenaId> {
        for (id, other) in self.data.iter().enumerate() {
            if it == other {
                return Some(id as ArenaId);
            }
        }
        None
    }
}
