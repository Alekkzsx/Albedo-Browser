pub struct Uuid(uuid::Uuid);

impl Uuid {
    pub fn new_v4() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}
