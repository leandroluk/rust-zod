pub mod codec;
pub mod coerce;
pub mod collections;
pub mod combinators;
pub mod number;
pub mod object;
pub mod primitives;
pub mod string;

pub use codec::*;
pub use coerce as coerce_mod;
pub use collections::*;
pub use combinators::*;
pub use number::*;
pub use object::*;
pub use primitives::*;
pub use string::*;

use crate::error::{SizeOrigin, ZodIssueCode};
use alloc::string::String;
use alloc::vec::Vec;

pub(crate) fn too_small(origin: SizeOrigin, minimum: f64, inclusive: bool, exact: bool) -> ZodIssueCode {
    ZodIssueCode::TooSmall {
        origin,
        minimum,
        inclusive,
        exact,
    }
}

pub(crate) fn too_big(origin: SizeOrigin, maximum: f64, inclusive: bool, exact: bool) -> ZodIssueCode {
    ZodIssueCode::TooBig {
        origin,
        maximum,
        inclusive,
        exact,
    }
}

/// Option lists accepted by `z::options(...)`, `.extract(...)` and `.exclude(...)`:
/// an array, a slice or a `Vec` (always a plain list, like `z.enum([...])`).
pub trait IntoOptions {
    fn into_options(self) -> Vec<String>;
}

impl<const N: usize> IntoOptions for [&str; N] {
    fn into_options(self) -> Vec<String> {
        self.into_iter().map(String::from).collect()
    }
}

impl IntoOptions for &[&str] {
    fn into_options(self) -> Vec<String> {
        self.iter().map(|&s| String::from(s)).collect()
    }
}

impl IntoOptions for Vec<&str> {
    fn into_options(self) -> Vec<String> {
        self.into_iter().map(String::from).collect()
    }
}

impl IntoOptions for Vec<String> {
    fn into_options(self) -> Vec<String> {
        self
    }
}
