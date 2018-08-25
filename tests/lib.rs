extern crate cached_file_view;

use std::fs::File;
use std::path::Path;
use cached_file_view::FileView;
use cached_file_view::FileViewError;

#[test]
fn test_read1() {
    let f = File::open(Path::new("tests/test.txt")).expect("file not found");
    let view = FileView::new(f);
    assert!(view.read(0..12).unwrap().get() == "hello world!".as_bytes());
}

#[test]
fn test_read2() {
    let f = File::open(Path::new("tests/test.txt")).expect("file not found");
    let view = FileView::new(f);
    assert!(view.read(11..12).unwrap().get() == "!".as_bytes());
}

#[test]
fn test_oob_end1() {
    let f = File::open(Path::new("tests/test.txt")).expect("file not found");
    let view = FileView::new(f);

    if let Err(err) = view.read(0..13) {
        assert!(err == FileViewError::EndOfFileError);
    } else {
        panic!("oob read did not fail")
    }
}

#[test]
fn test_oob_end2() {
    let f = File::open(Path::new("tests/test.txt")).expect("file not found");
    let view = FileView::new(f);

    if let Err(err) = view.read(10..13) {
        assert!(err == FileViewError::EndOfFileError);
    } else {
        panic!("oob read did not fail")
    }
}