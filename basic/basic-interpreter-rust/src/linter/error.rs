use crate::common::*;

//
// Result and error of this module
//

pub type Error = Locatable<String>;
pub type Result<T> = std::result::Result<T, Error>;
pub fn err<T, S: AsRef<str>>(msg: S, pos: Location) -> Result<T> {
    Err(Locatable::new(format!("[L] {}", msg.as_ref()), pos))
}
