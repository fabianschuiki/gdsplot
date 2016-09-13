// Copyright (c) 2016 Fabian Schuiki
extern crate cairo;
extern crate gds;
extern crate getopts;
// mod parser;

use std::io::{Read, BufRead, BufReader, stderr, stdin, Write};
use std::env;
use std::fs::File;
use getopts::Options;
use std::collections::{HashMap, HashSet, BTreeMap};
use std::rc::Rc;
// use parser::{Parser, ByteIter, Error};


fn print_usage(opts: Options) {
	stderr().write(
		opts.usage(
			"usage: gdsplot [OPTIONS] FILE [CELLNAME...]"
		).as_bytes()
	).unwrap();
}


fn main() {
	let mut args = env::args();
	args.next();

	// Configure and parse the command line options.
	let mut opts = Options::new();
	opts.optflag("h", "help", "print this help page");
	opts.optmulti("s", "style", "load the given stylesheet", "STYLESHEET");
	let matches = match opts.parse(args) {
		Ok(m) => { m },
		Err(m) => {
			writeln!(&mut stderr(), "{}", m).unwrap();
			std::process::exit(1);
		}
	};

	if matches.opt_present("h") {
		print_usage(opts);
		std::process::exit(0);
	}

	if matches.free.is_empty() {
		print_usage(opts);
		std::process::exit(1);
	}
	// println!("matches: {:?}", matches.free);
	let filename = &matches.free[0];
	let structs = &matches.free[1..];

	// Load the GDS file to be plotted.
	let mut rd = match gds::Reader::open_file(filename.as_str(), 0) {
		Ok(rd) => rd,
		Err(_) => {
			writeln!(&mut stderr(), "Unable to open GDS file `{}`", filename).unwrap();
			std::process::exit(1);
		}
	};
	let lib = gds::Library::read(&mut rd).expect("Unable to read GDS file");

	// Assemble the context from the command line arguments.
	let mut ctx = Context::new(&lib);
	for stylesheet in matches.opt_strs("s") {
		// println!("loading stylesheet {}", stylesheet);
		load_stylesheet(&mut ctx, stylesheet.as_str());
	}
	// println!("loaded aliases {:?}", &ctx.aliases);
	// println!("loaded assignments {:?}", &ctx.assignments);
	// println!("loaded classes {:?}", &ctx.classes);

	// Plot the cells passed on the command line.
	for name in structs {
		// println!("plotting {}", name);
		let strukt = match lib.find_struct(name) {
			Some(s) => s,
			None => {
				writeln!(&mut stderr(), "Unable to find cell {}", name).unwrap();
				std::process::exit(1);
			}
		};

		// Plot the structure.
		let s = prepare(&ctx, strukt);
		plot(&ctx, &s);
	}


	// Some sample Cairo code:
	// let mut surface = cairo::surface::Surface::create_image(cairo::surface::format::Format::ARGB32, 1000, 500);
	// let mut cr = cairo::Cairo::create(&mut surface);
	// cr.set_source_rgb(0.0,0.0,0.0);
	// cr.paint();
	// surface.write_to_png("debug.png");
}


struct Context<'a> {
	lib_units: f64,
	lib: &'a gds::Library,
	scale: ScaleMode,
	aliases: HashMap<Box<str>, u16>,
	only_layers: HashSet<u16>,
	boundary_layers: HashSet<u16>,
	assignments: HashMap<u16, Vec<Box<str>>>,
	classes: HashMap<Box<str>, LayerClass>,
	bg_color: Option<ColorRgb>,
	orders: HashMap<u16, i32>,
	margin: i32,
}

impl<'a> Context<'a> {
	fn new(lib: &'a gds::Library) -> Context {
		Context {
			lib_units: lib.get_units_in_m(),
			lib: lib,
			scale: ScaleMode::Size(512,512),
			aliases: HashMap::new(),
			only_layers: HashSet::new(),
			boundary_layers: HashSet::new(),
			assignments: HashMap::new(),
			classes: HashMap::new(),
			bg_color: None,
			orders: HashMap::new(),
			margin: 0,
		}
	}

	fn get_layer_style(&self, layer_id: u16) -> LayerClass {
		let mut style = LayerClass::new();
		if let Some(classes) = self.assignments.get(&layer_id) {
			for cls in classes {
				if let Some(class) = self.classes.get(cls) {
					style.merge(class);
				}
			}
		}
		style
	}
}

enum ScaleMode {
	/// Make the plot large enough to achieve a certain number of pixels per meter.
	Resolution(f64),
	/// Make the plot a specific size.
	Size(i32,i32),
}

#[derive(Debug)]
struct LayerClass {
	general: LayerClassSheet,
	fill: LayerClassSheet,
	stroke: LayerClassSheet,
}

#[derive(Debug, Clone)]
struct LayerClassSheet {
	color: Option<ColorRgb>,
	alpha: Option<f64>,
	width: Option<f64>,
	pattern: Option<FillPattern>,
	dashes: Option<Vec<f64>>,
}

impl LayerClass {
	fn new() -> LayerClass {
		LayerClass {
			general: LayerClassSheet::new(),
			fill: LayerClassSheet::new(),
			stroke: LayerClassSheet::new(),
		}
	}

	fn merge(&mut self, other: &LayerClass) {
		self.general.merge(&other.general);
		self.fill.merge(&other.fill);
		self.stroke.merge(&other.stroke);
	}

	fn get_fill_style(&self) -> Option<FillStyle> {
		let mut combined = self.general.clone();
		combined.merge(&self.fill);
		Some(FillStyle {
			color: match combined.color {
				Some(c) => c,
				None => return None,
			},
			alpha: combined.alpha.unwrap_or(1.0),
			pattern: match combined.pattern {
				Some(p) => p,
				None => return None,
			},
		})
	}

	fn get_stroke_style(&self) -> Option<StrokeStyle> {
		let mut combined = self.general.clone();
		combined.merge(&self.stroke);
		Some(StrokeStyle {
			color: match combined.color {
				Some(c) => c,
				None => return None,
			},
			alpha: combined.alpha.unwrap_or(1.0),
			width: match combined.width {
				Some(w) => w,
				None => return None,
			},
			dashes: combined.dashes,
		})
	}
}

impl LayerClassSheet {
	fn new() -> LayerClassSheet {
		LayerClassSheet {
			color: None,
			alpha: None,
			width: None,
			pattern: None,
			dashes: None,
		}
	}

	fn merge(&mut self, other: &LayerClassSheet) {
		if other.color.is_some() {
			self.color = other.color.clone();
		}
		if other.alpha.is_some() {
			self.alpha = other.alpha;
		}
		if other.width.is_some() {
			self.width = other.width;
		}
		if other.pattern.is_some() {
			self.pattern = other.pattern.clone();
		}
		if other.dashes.is_some() {
			self.dashes = other.dashes.clone();
		}
	}
}

struct FillStyle {
	color: ColorRgb,
	alpha: f64,
	pattern: FillPattern,
}

struct StrokeStyle {
	color: ColorRgb,
	alpha: f64,
	width: f64,
	dashes: Option<Vec<f64>>,
}

#[derive(Debug, Clone)]
enum FillPattern {
	Solid,
}


fn load_stylesheet(ctx: &mut Context, filename: &str) {
	// Open the file.
	let mut file = match File::open(filename) {
		Ok(f) => f,
		Err(e) => {
			writeln!(&mut stderr(), "Unable to open stylesheet {}: {}", filename, e).unwrap();
			std::process::exit(1);
		}
	};
	let mut lines = BufReader::new(file).lines();

	// Process each line.
	for line in lines {
		let line_unwrapped = line.unwrap();
		let args: Vec<&str> = line_unwrapped.split_whitespace().take_while(|x| !x.starts_with("//")).collect();
		if args.is_empty() {
			continue;
		}

		let mut it = args.iter();
		match *it.next().unwrap() {
			"alias" => {
				let id: u16 = it.next().unwrap().parse().unwrap();
				let alias = *it.next().unwrap();
				ctx.aliases.insert(alias.to_owned().into_boxed_str(), id);
				for cls in it {
					if ctx.assignments.contains_key(&id) {
						ctx.assignments.get_mut(&id).unwrap().push((*cls).to_owned().into_boxed_str());
					} else {
						ctx.assignments.insert(id, vec![(*cls).to_owned().into_boxed_str()]);
					}
				}
			},

			"general" => {
				let classname = *it.next().unwrap();
				if !ctx.classes.contains_key(&*classname) {
					ctx.classes.insert(classname.to_owned().into_boxed_str(), LayerClass::new());
				}
				let mut class = ctx.classes.get_mut(&*classname).unwrap();
				load_layer_class_sheet(&mut class.general, it);
			},

			"fill" => {
				let classname = *it.next().unwrap();
				if !ctx.classes.contains_key(&*classname) {
					ctx.classes.insert(classname.to_owned().into_boxed_str(), LayerClass::new());
				}
				let mut class = ctx.classes.get_mut(&*classname).unwrap();
				load_layer_class_sheet(&mut class.fill, it);
			},

			"stroke" => {
				let classname = *it.next().unwrap();
				if !ctx.classes.contains_key(&*classname) {
					ctx.classes.insert(classname.to_owned().into_boxed_str(), LayerClass::new());
				}
				let mut class = ctx.classes.get_mut(&*classname).unwrap();
				load_layer_class_sheet(&mut class.stroke, it);
			},

			"bgcolor" => {
				ctx.bg_color = Some(parse_color(*it.next().unwrap()).expect("invalid bgcolor"));
			},

			"only" => {
				for layer in it {
					let id: u16 = match ctx.aliases.get(*layer) {
						Some(v) => *v,
						None => layer.parse().expect("invalid layer ID"),
					};
					ctx.only_layers.insert(id);
				}
			},

			"order" => {
				let layer = *it.next().unwrap();
				let id: u16 = match ctx.aliases.get(layer) {
					Some(v) => *v,
					None => layer.parse().expect("invalid layer ID"),
				};
				ctx.orders.insert(id, it.next().unwrap().parse().expect("invalid layer order"));
			},

			"resolution" => {
				ctx.scale = ScaleMode::Resolution(it.next().unwrap().parse().expect("invalid resolution"));
			},

			"size" => {
				ctx.scale = ScaleMode::Size(
					it.next().unwrap().parse().expect("invalid width"),
					it.next().unwrap().parse().expect("invalid height"),
				);
			},

			"margin" => {
				ctx.margin = it.next().unwrap().parse().expect("invalid margin");
			},

			x => {
				writeln!(&mut stderr(), "{}: Unknown stylesheet command `{}`", filename, x).unwrap();
				std::process::exit(1);
			}
		};
	}
}

fn load_layer_class_sheet(dst: &mut LayerClassSheet, mut it: std::slice::Iter<&str>) {
	while let Some(opt) = it.next() {
		match *opt {
			"color" => {
				dst.color = Some(parse_color(*it.next().unwrap()).expect("invalid color"));
			},
			"alpha" => {
				dst.alpha = Some(it.next().unwrap().parse().expect("invalid alpha"));
			},
			"width" => {
				dst.width = Some(it.next().unwrap().parse().expect("invalid width"));
			},
			"dashes" => {
				dst.dashes = Some(it.map(|x| {
					let v: f64 = x.parse().expect("invalid dash width");
					v
				}).collect());
				break;
			},
			"pattern" => {
				dst.pattern = Some(match *it.next().unwrap() {
					"solid" => FillPattern::Solid,
					x => panic!("Unknown pattern `{}`", x)
				});
			}
			x => panic!("Unknown style parameter `{}`", x)
		}
	}
}

fn parse_color(s: &str) -> Result<ColorRgb, &str> {
	match s.chars().nth(0).unwrap() {
		'#' => {
			if s.len() != 7 {
				return Err(s);
			}
			let (_,rem) = s.split_at(1);
			let (rs,rem) = rem.split_at(2);
			let (gs,rem) = rem.split_at(2);
			let (bs,rem) = rem.split_at(2);
			let r = u8::from_str_radix(rs, 16).unwrap();
			let g = u8::from_str_radix(gs, 16).unwrap();
			let b = u8::from_str_radix(bs, 16).unwrap();
			Ok(ColorRgb(
				r as f64 / 255.0,
				g as f64 / 255.0,
				b as f64 / 255.0
			))
		}
		_ => Err(s),
	}
}


struct Struct {
	layers: Vec<Rc<Layer>>,
	name: Box<str>,
	boundaries: Vec<Boundary>,
	extents: Extents,
}

#[derive(Debug, Clone, Copy)]
struct Point {
	x: f64,
	y: f64,
}

impl std::ops::Sub for Point {
	type Output = Vector;
	fn sub(self, rhs: Point) -> Vector {
		Vector { x: self.x - rhs.x, y: self.y - rhs.y }
	}
}

#[derive(Debug, Clone, Copy)]
struct Vector {
	x: f64,
	y: f64,
}

#[derive(Debug, Clone, Copy)]
struct Transform {
	va: Vector,
	vb: Vector,
	vt: Vector,
}

impl Transform {
	fn identity() -> Transform {
		Transform {
			va: Vector { x: 1.0, y: 0.0 },
			vb: Vector { x: 0.0, y: 1.0 },
			vt: ZERO_VECTOR,
		}
	}

	fn scale(&mut self, sx: f64, sy: f64) {
		self.va.x *= sx;
		self.va.y *= sy;
		self.vb.x *= sx;
		self.vb.y *= sy;
		self.vt.x *= sx;
		self.vt.y *= sy;
	}

	fn trans(&mut self, tx: f64, ty: f64) {
		self.vt.x += tx;
		self.vt.y += ty;
	}
}

impl std::ops::Mul<Vector> for Transform {
	type Output = Vector;
	fn mul(self, rhs: Vector) -> Vector {
		Vector {
			x: self.va.x * rhs.x + self.vb.x * rhs.y,
			y: self.va.y * rhs.x + self.vb.y * rhs.y,
		}
	}
}

impl std::ops::Mul<Point> for Transform {
	type Output = Point;
	fn mul(self, rhs: Point) -> Point {
		Point {
			x: self.va.x * rhs.x + self.vb.x * rhs.y + self.vt.x,
			y: self.va.y * rhs.x + self.vb.y * rhs.y + self.vt.y,
		}
	}
}

#[derive(Debug, Clone, Copy)]
struct Rect {
	min: Point,
	max: Point,
}

const ZERO_POINT: Point = Point { x: 0.0, y: 0.0 };
const ZERO_VECTOR: Vector = Vector { x: 0.0, y: 0.0 };
const ZERO_RECT: Rect = Rect { min: ZERO_POINT, max: ZERO_POINT };

struct Boundary {
	layer: Rc<Layer>,
	points: Vec<Point>,
}

#[derive(Clone, Copy)]
struct Extents {
	rect: Rect,
	empty: bool,
}

impl Extents {
	fn new() -> Extents {
		Extents {
			rect: ZERO_RECT,
			empty: true,
		}
	}

	fn add_point(&mut self, p: &Point) {
		if self.empty {
			self.rect = Rect { min: *p, max: *p };
			self.empty = false;
		} else {
			if self.rect.min.x > p.x {
				self.rect.min.x = p.x;
			}
			if self.rect.min.y > p.y {
				self.rect.min.y = p.y;
			}
			if self.rect.max.x < p.x {
				self.rect.max.x = p.x;
			}
			if self.rect.max.y < p.y {
				self.rect.max.y = p.y;
			}
		}
	}
}

#[derive(Debug, Clone, Copy)]
struct ColorRgb {
	r: f64,
	g: f64,
	b: f64,
}

#[allow(non_snake_case)]
fn ColorRgb(r: f64, g: f64, b: f64) -> ColorRgb {
	ColorRgb {
		r: r,
		g: g,
		b: b,
	}
}

struct Layer {
	id: u16,
	order: i32,
	style: LayerClass,
}

impl std::cmp::PartialEq for Layer {
	fn eq(&self, other: &Layer) -> bool {
		self.id == other.id
	}
}


fn prepare(ctx: &Context, strukt: gds::Struct) -> Struct {
	let prepared = BTreeMap::<Box<str>, Rc<Struct>>::new();

	let mut layers = BTreeMap::<u16, Rc<Layer>>::new();
	let mut extents = Extents::new();
	let mut boundaries = Vec::new();

	// Collect the elements of this struct.
	for elem in strukt.elems() {
		// println!("- found a {:?} on layer {}:{}", elem.get_kind(), elem.get_layer(), elem.get_type());

		// Pick the layer for this element.
		let layer_id = elem.get_layer();
		if !ctx.only_layers.is_empty() && !ctx.only_layers.contains(&layer_id) {
			continue;
		}

		let layer =
			if layers.contains_key(&layer_id) {
				layers.get(&layer_id).unwrap().clone()
			} else {
				let style = ctx.get_layer_style(layer_id);
				// println!("- create layer with style {:?}", style);
				let l = Rc::new(Layer {
					id: layer_id,
					order: match ctx.orders.get(&layer_id) {
						Some(v) => *v,
						None => layer_id as i32,
					},
					style: style,
				});
				layers.insert(layer_id, l.clone());
				l
			};

		match elem.get_kind() {
			gds::ElemKind::Boundary => {
				let pts = elem.get_xy().iter().map(|xy| Point {
					x: xy.x as f64 * ctx.lib_units,
					y: xy.y as f64 * ctx.lib_units,
				}).collect();
				boundaries.push(Boundary {
					layer: layer,
					points: pts,
				});
			},
			_ => ()
		}
	}

	// Calculate the extents.
	for b in &boundaries {
		for p in &b.points {
			extents.add_point(p);
		}
	}

	// Make an ordered list of layers.
	let mut ordered_layers: Vec<Rc<Layer>> = Vec::new();
	for (_,l) in layers {
		ordered_layers.push(l);
	}
	ordered_layers.sort_by_key(|l| l.order);

	Struct {
		layers: ordered_layers,
		name: strukt.get_name().into_boxed_str(),
		boundaries: boundaries,
		extents: extents,
	}
}


fn plot(ctx: &Context, strukt: &Struct) {

	// Calculate the overall plot size and transformation.
	let r = &strukt.extents.rect;
	let phys_size = r.max - r.min;
	let mut tx = Transform::identity();
	tx.trans(-r.min.x, -r.min.y);
	let plot_size = match ctx.scale {
		ScaleMode::Resolution(ppm) => {
			tx.scale(ppm,ppm);
			let sz = tx * phys_size;
			((sz.x + 0.5) as i32, (sz.y + 0.5) as i32)
		},
		ScaleMode::Size(w,h) => {
			let fw = w as f64 / phys_size.x;
			let fh = h as f64 / phys_size.y;
			if fw < fh {
				tx.scale(fw,fw);
				(w, ((h as f64)/fw + 0.5) as i32)
			} else {
				tx.scale(fh,fh);
				(((w as f64)/fh + 0.5) as i32, h)
			}
		},
	};
	tx.scale(1.0, -1.0);
	tx.trans(0.0, plot_size.1 as f64);

	let plot_size = (plot_size.0 + 2*ctx.margin, plot_size.1 + 2*ctx.margin);
	tx.trans(ctx.margin as f64, ctx.margin as f64);

	// println!("plotting struct of physical size {:?} onto {:?}", phys_size, plot_size);

	// Prepare the plot surface.
	let mut surface = cairo::surface::Surface::create_image(cairo::surface::format::Format::ARGB32, plot_size.0 as i32, plot_size.1 as i32);
	let mut cr = cairo::Cairo::create(&mut surface);
	cr.set_fill_rule(cairo::fill_rule::FillRule::EvenOdd);

	// Draw the background.
	if let Some(bgc) = ctx.bg_color {
		cr.set_source_rgb(bgc.r, bgc.g, bgc.b);
		cr.paint();
	}

	// Plot.
	plot_struct(ctx, strukt, tx, &mut cr);

	// Write the file.
	let output_name = format!("{}.png", strukt.name);
	surface.write_to_png(output_name.as_str());
}


fn plot_struct(ctx: &Context, strukt: &Struct, tx: Transform, cr: &mut cairo::Cairo) {
	for layer in &strukt.layers {
		// Fill the geometry on this layer.
		if let Some(fs) = layer.style.get_fill_style() {
			cr.push_group();
			cr.set_source_rgb(fs.color.r, fs.color.g, fs.color.b);
			gather_geometry(strukt, layer, tx, cr, Pass::Fill);
			cr.pop_group_to_source();
			cr.paint_with_alpha(fs.alpha);
		}
	}
	for layer in &strukt.layers {
		// Stroke the geometry on this layer.
		if let Some(ss) = layer.style.get_stroke_style() {
			cr.save();
			cr.set_source_rgba(ss.color.r, ss.color.g, ss.color.b, ss.alpha);
			if let Some(mut dashes) = ss.dashes {
				cr.set_dash(&mut dashes[..], 0.0);
			}
			cr.set_line_width(ss.width);
			gather_geometry(strukt, layer, tx, cr, Pass::Stroke);
			cr.restore();
		}
	}
}


enum Pass {
	Fill,
	Stroke,
}

fn gather_geometry(strukt: &Struct, layer: &Rc<Layer>, tx: Transform, cr: &mut cairo::Cairo, pass: Pass) {
	// Boundaries.
	for b in &strukt.boundaries {
		if b.layer != *layer {
			continue;
		}

		let mut it = b.points.iter();
		it.next();

		if let Some(pt) = it.next() {
			let p = tx * *pt;
			cr.move_to(p.x, p.y);
		}

		for pt in it {
			let p = tx * *pt;
			cr.line_to(p.x, p.y);
		}

		cr.close_path();

		match pass {
			Pass::Fill => cr.fill(),
			Pass::Stroke => cr.stroke(),
		}
	}
}
