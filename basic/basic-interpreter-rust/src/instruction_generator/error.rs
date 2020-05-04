use crate::common::{Locatable, Location};

pub type Error = Locatable<String>;
pub type Result<T> = std::result::Result<T, Error>;

pub fn err<T, S: AsRef<str>>(msg: S, pos: Location) -> Result<T> {
    Err(Locatable::new(format!("[IG] {}", msg.as_ref()), pos))
}
