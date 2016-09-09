// Copyright (c) 2016 Fabian Schuiki
extern crate libc;
pub mod reader;
pub mod library;
pub mod strukt;
mod ffi;

pub use reader::Reader;
pub use library::Library;
pub use strukt::{Struct, Elem, ElemKind};


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
