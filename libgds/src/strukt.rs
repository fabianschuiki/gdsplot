// Copyright (c) 2016 Fabian Schuiki

//! This module implements a wrapper around the gds_struct_t.

use std;
use std::ptr;
use std::ffi::CStr;
use ffi::*;


pub struct Struct {
	opaque: *mut gds_struct_t
}

impl Struct {
	pub fn new(p: *mut gds_struct_t) -> Struct {
		unsafe { gds_struct_ref(p) };
		Struct { opaque: p }
	}

	/// Return the name of the struct.
	pub fn get_name(&self) -> String {
		unsafe {
			CStr::from_ptr(gds_struct_get_name(self.opaque))
		}.to_string_lossy().into_owned()
	}

	pub fn get_num_elems(&self) -> usize {
		unsafe { gds_struct_get_num_elems(self.opaque) as usize }
	}

	pub fn get_elem(&self, idx: usize) -> Elem {
		let p = unsafe { gds_struct_get_elem(self.opaque, idx as _) };
		assert!(!p.is_null());
		Elem::new(p)
	}

	pub fn elems(&self) -> ElemIter {
		ElemIter {
			strukt: self,
			cur: 0,
			end: self.get_num_elems(),
		}
	}
}

impl Drop for Struct {
	fn drop(&mut self) {
		unsafe { gds_struct_unref(self.opaque) };
	}
}


pub struct ElemIter<'a> {
	strukt: &'a Struct,
	cur: usize,
	end: usize,
}

impl<'a> Iterator for ElemIter<'a> {
	type Item = Elem;
	fn next(&mut self) -> Option<Self::Item> {
		if self.cur < self.end {
			let s = self.strukt.get_elem(self.cur);
			self.cur += 1;
			Some(s)
		} else {
			None
		}
	}
}


pub struct Elem {
	opaque: *mut gds_elem_t,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElemKind {
	Boundary,
	Path,
	Sref,
	Aref,
	Text,
	Node,
	Box,
}

impl Elem {
	pub fn new(p: *mut gds_elem_t) -> Elem {
		Elem { opaque: p }
	}

	pub fn get_kind(&self) -> ElemKind {
		match unsafe { gds_elem_get_kind(self.opaque) } {
			1 => ElemKind::Boundary,
			2 => ElemKind::Path,
			3 => ElemKind::Sref,
			4 => ElemKind::Aref,
			5 => ElemKind::Text,
			6 => ElemKind::Node,
			7 => ElemKind::Box,
			x => panic!("unknown elem kind {}", x),
		}
	}

	pub fn get_layer(&self) -> u16 {
		unsafe { gds_elem_get_layer(self.opaque) as u16 }
	}

	pub fn get_type(&self) -> u16 {
		unsafe { gds_elem_get_type(self.opaque) as u16 }
	}

	pub fn get_strans(&self) -> gds_strans_t {
		unsafe { gds_elem_get_strans(self.opaque) }
	}

	pub fn get_xy(&self) -> &[gds_xy_t] {
		unsafe { std::slice::from_raw_parts(
			gds_elem_get_xy(self.opaque),
			gds_elem_get_num_xy(self.opaque) as usize,
		)}
	}

	pub fn get_sname(&self) -> String {
		unsafe {
			CStr::from_ptr(gds_elem_get_sname(self.opaque)).to_string_lossy().into_owned()
		}
	}

	pub fn get_text(&self) -> String {
		unsafe {
			CStr::from_ptr(gds_elem_get_text(self.opaque)).to_string_lossy().into_owned()
		}
	}
}
