// Copyright (c) 2016 Fabian Schuiki
//! Raw bindings to the libgds C library.
#![allow(improper_ctypes, non_camel_case_types, dead_code)]

use libc::{c_char, c_void, c_int, size_t, int32_t, uint16_t, c_double};


#[link(name = "gds")]
extern {
	pub fn gds_lib_read(lib: *mut *mut gds_lib_t, rd: *mut c_void) -> c_int;
	pub fn gds_lib_write(lib: *mut gds_lib_t, wr: *mut c_void) -> c_int;
	pub fn gds_lib_create() -> *mut gds_lib_t;
	pub fn gds_lib_destroy(lib: *mut gds_lib_t);
	pub fn gds_lib_get_num_structs(lib: *mut gds_lib_t) -> size_t;
	pub fn gds_lib_get_struct(lib: *mut gds_lib_t, idx: size_t) -> *mut gds_struct_t;
	pub fn gds_lib_get_units(lib: *mut gds_lib_t) -> gds_units_t;

	pub fn gds_struct_ref(strukt: *mut gds_struct_t);
	pub fn gds_struct_unref(strukt: *mut gds_struct_t);
	pub fn gds_struct_get_name(strukt: *mut gds_struct_t) -> *const c_char;
	pub fn gds_struct_get_num_elems(strukt: *mut gds_struct_t) -> size_t;
	pub fn gds_struct_get_elem(strukt: *mut gds_struct_t, idx: size_t) -> *mut gds_elem_t;

	pub fn gds_elem_get_kind(elem: *mut gds_elem_t) -> c_int;
	pub fn gds_elem_get_layer(elem: *mut gds_elem_t) -> uint16_t;
	pub fn gds_elem_get_type(elem: *mut gds_elem_t) -> uint16_t;
	pub fn gds_elem_get_strans(elem: *mut gds_elem_t) -> gds_strans_t;
	pub fn gds_elem_get_num_xy(elem: *mut gds_elem_t) -> uint16_t;
	pub fn gds_elem_get_xy(elem: *mut gds_elem_t) -> *mut gds_xy_t;
	pub fn gds_elem_get_sname(elem: *mut gds_elem_t) -> *const c_char;
	pub fn gds_elem_get_text(elem: *mut gds_elem_t) -> *const c_char;
}

pub struct gds_lib_t;
pub struct gds_struct_t;
pub struct gds_elem_t;

#[repr(C)]
pub struct gds_units_t {
	pub dbu_in_uu: c_double,
	pub dbu_in_m: c_double,
}

#[repr(C)]
pub struct gds_strans_t {
	pub flags: uint16_t,
	pub mag: c_double,
	pub angle: c_double,
}

#[repr(C)]
pub struct gds_xy_t {
	pub x: int32_t,
	pub y: int32_t,
}
