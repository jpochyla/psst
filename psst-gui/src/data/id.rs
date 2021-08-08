pub trait Id {
    type Id: PartialEq;

    fn id(&self) -> Self::Id;

    fn has_id(&self, id: &Self::Id) -> bool {
        id == &self.id()
    }
}
