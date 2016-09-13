# gdsplot

A simple stylesheet-based GDSII plotter. Relies on the [libgds](https://github.com/fabianschuiki/libgds) and [cairo](https://www.cairographics.org/) C libraries to read files and render graphics. The project consists of the main `gdsplot` crate that contains the binary, and the `libgds` subcrate that contains the foreign function interface (FFI) to libgds.


## Usage

    gdsplot [-s STYLESHEET ...] GDS_FILE CELL ...

Given a GDS file name, the program will render all of the given cells. Multiple stylesheets can be defined, with latter overriding options from the former. Take a look at `load_stylesheet(...)` in `src/main.rs` to see the different options available.
