pub struct Uuid;

impl Uuid {
    pub fn new_v4() -> Self {
        Self
    }
    pub fn to_string(&self) -> String {
        "00000000-0000-0000-0000-000000000000".to_string()
    }
}
