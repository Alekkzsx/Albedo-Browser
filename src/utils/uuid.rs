pub struct Uuid(uuid::Uuid);

impl Uuid {
    /// TODO: add docs
    pub fn new_v4() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// TODO: add docs
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}
