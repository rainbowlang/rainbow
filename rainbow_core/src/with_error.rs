use std::fmt::{Debug, Display};

pub trait WithError {
    type Error: for<'a> From<&'a str> + From<String> + Display + Debug;
}
