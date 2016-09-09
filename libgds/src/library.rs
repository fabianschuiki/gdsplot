// Copyright (c) 2016 Fabian Schuiki

//! An implementation of a GDS library that contains various cells.

use std::ptr;
use reader::Reader;
use strukt::Struct;
use ffi::*;


/// A GDS library.
pub struct Library {
	opaque: *mut gds_lib_t,
}

impl Library {
	/// Create a new, empty library.
	pub fn create() -> Library {
		let ptr = unsafe { gds_lib_create() };
		assert!(!ptr.is_null());
		Library {
			opaque: ptr
		}
	}

	/// Create a new library from a GDS stream.
	pub fn read(rd: &mut Reader) -> Result<Library, ()> {
		let mut p: *mut gds_lib_t = ptr::null_mut();
		let fr = unsafe { gds_lib_read(&mut p, rd.opaque) };
		if fr == 0 {
			assert!(!p.is_null());
			Ok(Library { opaque: p })
		} else {
			Err(())
		}
	}

	pub fn get_num_structs(&self) -> usize {
		unsafe { gds_lib_get_num_structs(self.opaque) as usize }
	}

	pub fn get_struct(&self, idx: usize) -> Struct {
		let p = unsafe { gds_lib_get_struct(self.opaque, idx as _) };
		assert!(!p.is_null());
		Struct::new(p)
	}

	/// Return an iterator over all structs in the library.
	pub fn structs(&self) -> StructIter {
		StructIter {
			library: self,
			cur: 0,
			end: self.get_num_structs(),
		}
	}

	/// Find a structure with the given name. Returns `None` if no such
	/// structure exists in the library.
	pub fn find_struct(&self, name: &str) -> Option<Struct> {
		for s in self.structs() {
			if s.get_name() == name {
				return Some(s);
			}
		}
		None
	}

	pub fn get_units_in_m(&self) -> f64 {
		unsafe { gds_lib_get_units(self.opaque).dbu_in_m }
	}

	pub fn get_units_in_uu(&self) -> f64 {
		unsafe { gds_lib_get_units(self.opaque).dbu_in_uu }
	}
}

impl Drop for Library {
	fn drop(&mut self) {
		unsafe {
			gds_lib_destroy(self.opaque);
		}
	}
}


pub struct StructIter<'a> {
	library: &'a Library,
	cur: usize,
	end: usize,
}

impl<'a> Iterator for StructIter<'a> {
	type Item = Struct;
	fn next(&mut self) -> Option<Self::Item> {
		if self.cur < self.end {
			let s = self.library.get_struct(self.cur);
			self.cur += 1;
			Some(s)
		} else {
			None
		}
	}
}
